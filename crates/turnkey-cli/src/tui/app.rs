//! Main TUI application state and event loop.
//!
//! Orchestrates all TUI components and handles keyboard input.

use std::io;
use std::time::Duration;

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    layout::{Constraint, Direction, Layout},
    prelude::CrosstermBackend,
};
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};
use turnkey_emulator::{EmulatorEvent, EmulatorHandle, InputCommand, TurnstileState};

use super::{
    display::DisplayComponent, keypad::KeypadComponent, logs::LogsPanel, status::StatusBar,
};

/// Minimum terminal width.
const MIN_TERMINAL_WIDTH: u16 = 80;

/// Minimum terminal height.
const MIN_TERMINAL_HEIGHT: u16 = 24;

/// Frame duration for 60 FPS (16ms).
const FRAME_DURATION_MS: u64 = 16;

/// Main TUI application state.
pub struct App {
    /// Emulator handle for state observation.
    emulator_handle: EmulatorHandle,
    /// Display component.
    display: DisplayComponent,
    /// Keypad component.
    keypad: KeypadComponent,
    /// Logs panel.
    logs: LogsPanel,
    /// Status bar.
    status: StatusBar,
    /// Current turnstile state.
    turnstile_state: TurnstileState,
    /// Should quit.
    should_quit: bool,
    /// Needs redraw flag to optimize rendering.
    needs_redraw: bool,
}

impl App {
    /// Create a new TUI application.
    pub fn new(emulator_handle: EmulatorHandle) -> Self {
        let turnstile_state = emulator_handle.current_state();

        Self {
            emulator_handle,
            display: DisplayComponent::new(),
            keypad: KeypadComponent::new(),
            logs: LogsPanel::new(),
            status: StatusBar::new(),
            turnstile_state,
            should_quit: false,
            needs_redraw: true,
        }
    }

    /// Run the TUI event loop.
    pub async fn run(&mut self) -> anyhow::Result<()> {
        info!("Starting TUI");

        self.logs.info("Inicializando...".to_string());
        self.logs.success("Sistema pronto".to_string());

        let mut terminal = setup_terminal()?;

        let mut events_rx = self.emulator_handle.subscribe_events();
        let mut state_rx = self.emulator_handle.state_receiver();
        let mut display_rx = self.emulator_handle.display_receiver();

        loop {
            if self.should_quit {
                break;
            }

            // Only redraw if something changed to reduce CPU usage
            if self.needs_redraw {
                terminal.draw(|f| self.render(f))?;
                self.needs_redraw = false;
            }

            tokio::select! {
                result = tokio::time::timeout(
                    Duration::from_millis(FRAME_DURATION_MS),
                    tokio::task::spawn_blocking(|| event::poll(Duration::from_millis(0)))
                ) => {
                    if let Ok(Ok(Ok(true))) = result
                        && let Err(e) = self.handle_keyboard_input().await {
                            error!(error = %e, "Error handling keyboard input");
                        }
                }

                event = events_rx.recv() => {
                    match event {
                        Ok(e) => {
                            self.handle_emulator_event(e);
                            self.needs_redraw = true;
                        }
                        Err(broadcast::error::RecvError::Lagged(skipped)) => {
                            warn!(skipped, "Event receiver lagged, some events were skipped");
                            self.needs_redraw = true;
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            info!("Event channel closed, shutting down TUI");
                            break;
                        }
                    }
                }

                _ = state_rx.changed() => {
                    self.turnstile_state = *state_rx.borrow();
                    debug!(?self.turnstile_state, "State changed");
                    self.needs_redraw = true;
                }

                _ = display_rx.changed() => {
                    debug!("Display updated");
                    self.needs_redraw = true;
                }
            }
        }

        restore_terminal(&mut terminal)?;
        info!("TUI shutdown complete");
        Ok(())
    }

    /// Render the TUI.
    fn render(&mut self, f: &mut ratatui::Frame) {
        let size = f.area();

        if size.width < MIN_TERMINAL_WIDTH || size.height < MIN_TERMINAL_HEIGHT {
            self.render_size_warning(f);
            return;
        }

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
            .split(size);

        self.render_emulator_column(f, chunks[0]);
        self.logs.render(f, chunks[1]);
    }

    /// Render size warning when terminal is too small.
    fn render_size_warning(&self, f: &mut ratatui::Frame) {
        use ratatui::{
            style::{Color, Modifier, Style},
            text::{Line, Span},
            widgets::Paragraph,
        };

        let size = f.area();
        let warning = Paragraph::new(vec![
            Line::from(Span::styled(
                "Terminal muito pequeno",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::raw("")),
            Line::from(Span::raw(format!(
                "Tamanho atual: {}x{}",
                size.width, size.height
            ))),
            Line::from(Span::raw(format!(
                "Tamanho minimo: {}x{}",
                MIN_TERMINAL_WIDTH, MIN_TERMINAL_HEIGHT
            ))),
            Line::from(Span::raw("")),
            Line::from(Span::styled(
                "Por favor, redimensione o terminal",
                Style::default().fg(Color::Yellow),
            )),
        ]);

        f.render_widget(warning, f.area());
    }

    /// Render the emulator column (display + keypad + status).
    fn render_emulator_column(&mut self, f: &mut ratatui::Frame, area: ratatui::layout::Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(30),
                Constraint::Percentage(60),
                Constraint::Percentage(10),
            ])
            .split(area);

        let display_state = self.emulator_handle.current_display();
        self.display
            .render(f, chunks[0], &display_state, self.turnstile_state);
        self.keypad.render(f, chunks[1]);
        self.status.render(f, chunks[2]);
    }

    /// Handle keyboard input.
    async fn handle_keyboard_input(&mut self) -> anyhow::Result<()> {
        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                return Ok(());
            }

            match key.code {
                KeyCode::Char('q') | KeyCode::Char('Q') => {
                    self.should_quit = true;
                    self.emulator_handle.shutdown().await?;
                }
                KeyCode::Char(c) if c.is_ascii_digit() => {
                    // Safe: is_ascii_digit() guarantees this conversion succeeds
                    let digit = c
                        .to_digit(10)
                        .expect("BUG: is_ascii_digit() guarantees digit conversion")
                        as u8;
                    self.keypad.set_last_pressed(c);
                    self.needs_redraw = true;
                    self.emulator_handle
                        .send_input(InputCommand::KeypadDigit(digit))
                        .await?;
                }
                KeyCode::Enter => {
                    self.needs_redraw = true;
                    self.emulator_handle
                        .send_input(InputCommand::KeypadEnter)
                        .await?;
                }
                KeyCode::Esc => {
                    self.needs_redraw = true;
                    self.emulator_handle
                        .send_input(InputCommand::KeypadCancel)
                        .await?;
                }
                KeyCode::Backspace => {
                    self.needs_redraw = true;
                    self.emulator_handle
                        .send_input(InputCommand::KeypadClear)
                        .await?;
                }
                KeyCode::Up => {
                    self.logs.scroll_up();
                    self.needs_redraw = true;
                }
                KeyCode::Down => {
                    self.logs.scroll_down();
                    self.needs_redraw = true;
                }
                KeyCode::Home => {
                    self.logs.scroll_to_top();
                    self.needs_redraw = true;
                }
                KeyCode::End => {
                    self.logs.scroll_to_bottom();
                    self.needs_redraw = true;
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Handle emulator event.
    fn handle_emulator_event(&mut self, event: EmulatorEvent) {
        debug!(?event, "Handling emulator event");

        self.status.increment_event_count();

        match &event {
            EmulatorEvent::KeypadInput(c) => {
                self.logs.info(format!("Tecla: {}", c));
            }
            EmulatorEvent::CardRead(card) => {
                self.logs.info(format!("Cartao: {}", card));
            }
            EmulatorEvent::StateTransition { from, to } => {
                self.logs.info(format!("Estado: {:?} -> {:?}", from, to));
            }
            EmulatorEvent::ValidationRequest {
                credential,
                reader_type,
            } => {
                self.logs
                    .info(format!("REQ->SRV ({:?}): {}", reader_type, credential));
            }
            EmulatorEvent::ValidationResult(response) => {
                if response.is_grant() {
                    self.logs
                        .success(format!("GRANT: {}", response.display_message()));
                } else {
                    self.logs
                        .warning(format!("DENY: {}", response.display_message()));
                }
            }
            EmulatorEvent::Error {
                message,
                category,
                recoverable,
            } => {
                if *recoverable {
                    self.logs.warning(format!("{}: {}", category, message));
                } else {
                    self.logs.error(format!("{}: {}", category, message));
                }
            }
            EmulatorEvent::ProtocolMessageSent {
                command_code,
                state,
            } => {
                self.logs
                    .info(format!("PROTOCOL: {} ({:?})", command_code, state));
            }
            _ => {
                debug!(?event, "Unhandled event type");
            }
        }
    }
}

/// Setup the terminal for TUI rendering.
fn setup_terminal() -> anyhow::Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

/// Restore the terminal to normal mode.
fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> anyhow::Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

/// Run the TUI with the given emulator handle.
///
/// This is the main entry point for the TUI application.
pub async fn run_tui(emulator_handle: EmulatorHandle) -> anyhow::Result<()> {
    let mut app = App::new(emulator_handle);
    app.run().await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants() {
        assert_eq!(MIN_TERMINAL_WIDTH, 80);
        assert_eq!(MIN_TERMINAL_HEIGHT, 24);
        assert_eq!(FRAME_DURATION_MS, 16);
    }
}
