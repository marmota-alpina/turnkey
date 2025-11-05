//! Main client-emulator TUI application state and event loop.
//!
//! Orchestrates all TUI components and handles keyboard input for the
//! validation server interface.

use std::io;
use std::net::SocketAddr;
use std::time::Duration;

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    layout::{Constraint, Direction, Layout, Rect},
    prelude::CrosstermBackend,
    text::{Line, Span},
    widgets::{Block, Paragraph},
};
use tokio::time::sleep;
use tracing::{debug, error, info};
use turnkey_network::{TcpServer, TcpServerConfig};
use turnkey_protocol::{
    CommandCode, FieldData, Message, MessageBuilder, commands::access::AccessRequest,
};

use super::{
    connected_devices::ConnectedDevicesPanel, decision_panel::DecisionButton,
    decision_panel::DecisionPanel, decision_panel::InputField, history::HistoryPanel,
    request_panel::RequestPanel, theme::header_style,
};

/// Minimum terminal width.
const MIN_TERMINAL_WIDTH: u16 = 80;

/// Minimum terminal height.
const MIN_TERMINAL_HEIGHT: u16 = 30;

/// Frame duration for 60 FPS (16ms).
const FRAME_DURATION_MS: u64 = 16;

/// Interval for updating connected devices list (1 second).
const DEVICE_UPDATE_INTERVAL_MS: u64 = 1000;

/// Server operational state.
#[derive(Debug, Clone, PartialEq, Eq)]
enum ServerState {
    /// Server is listening for connections.
    Listening,
    /// Server encountered an error.
    Error(String),
}

/// Main client-emulator TUI application state.
pub struct App {
    /// TCP server for receiving connections.
    server: TcpServer,
    /// Connected devices panel.
    devices_panel: ConnectedDevicesPanel,
    /// Current request panel.
    request_panel: RequestPanel,
    /// Decision panel.
    decision_panel: DecisionPanel,
    /// History panel.
    history_panel: HistoryPanel,
    /// Should quit flag.
    should_quit: bool,
    /// Needs redraw flag.
    needs_redraw: bool,
    /// Server bind address.
    bind_addr: SocketAddr,
    /// Current server state.
    server_state: ServerState,
    /// Last error message to display.
    last_error: Option<String>,
    /// Pending response to send (device_id, message).
    pending_response: Option<(turnkey_core::DeviceId, Message)>,
}

impl App {
    /// Create a new TUI application.
    pub async fn new(bind_addr: SocketAddr) -> anyhow::Result<Self> {
        let config = TcpServerConfig {
            bind_addr,
            max_connections: 100,
        };

        let server = TcpServer::bind(config).await?;

        info!("Client-emulator TUI started on {}", bind_addr);

        Ok(Self {
            server,
            devices_panel: ConnectedDevicesPanel::new(),
            request_panel: RequestPanel::new(),
            decision_panel: DecisionPanel::new(),
            history_panel: HistoryPanel::new(),
            should_quit: false,
            needs_redraw: true,
            bind_addr,
            server_state: ServerState::Listening,
            last_error: None,
            pending_response: None,
        })
    }

    /// Run the TUI event loop.
    pub async fn run(&mut self) -> anyhow::Result<()> {
        info!("Starting client-emulator TUI");

        let mut terminal = setup_terminal()?;
        let mut device_update_timer =
            tokio::time::interval(Duration::from_millis(DEVICE_UPDATE_INTERVAL_MS));

        loop {
            if self.should_quit {
                break;
            }

            // Only redraw if something changed
            if self.needs_redraw {
                terminal.draw(|f| self.render(f))?;
                self.needs_redraw = false;
            }

            tokio::select! {
                // Handle keyboard input
                result = tokio::time::timeout(
                    Duration::from_millis(FRAME_DURATION_MS),
                    tokio::task::spawn_blocking(|| event::poll(Duration::from_millis(0)))
                ) => {
                    if let Ok(Ok(Ok(true))) = result
                        && let Ok(event) = event::read() {
                            self.handle_input(event);
                        }
                }

                // Handle incoming messages from turnstiles
                result = self.server.recv_any() => {
                    match result {
                        Ok((device_id, message)) => {
                            debug!("Received message from device {}: {:?}", device_id, message);
                            self.handle_message(device_id, message).await;
                            self.server_state = ServerState::Listening;
                            self.last_error = None;
                        }
                        Err(e) => {
                            let error_msg = format!("Receive error: {}", e);
                            error!("{}", error_msg);
                            self.server_state = ServerState::Error(error_msg.clone());
                            self.last_error = Some(error_msg);
                            self.needs_redraw = true;
                        }
                    }
                }

                // Periodic device list update
                _ = device_update_timer.tick() => {
                    let devices = self.server.all_connections_info();
                    self.devices_panel.update_devices(devices);
                    self.needs_redraw = true;
                }
            }

            // Send pending response if any
            if let Some((device_id, message)) = self.pending_response.take() {
                match self.server.send(device_id, message).await {
                    Ok(_) => {
                        info!("Response sent to device {}", device_id);
                    }
                    Err(e) => {
                        let error_msg = format!("Send error: {}", e);
                        error!("{}", error_msg);
                        self.last_error = Some(error_msg);
                        self.needs_redraw = true;
                    }
                }
            }

            sleep(Duration::from_millis(FRAME_DURATION_MS)).await;
        }

        cleanup_terminal(&mut terminal)?;

        Ok(())
    }

    /// Handle keyboard input.
    fn handle_input(&mut self, event: Event) {
        if let Event::Key(key) = event {
            if key.kind != KeyEventKind::Press {
                return;
            }

            match key.code {
                KeyCode::Char('q') | KeyCode::Char('Q') => {
                    self.should_quit = true;
                }
                KeyCode::Char('e') | KeyCode::Char('E') => {
                    self.decision_panel
                        .select_button(DecisionButton::GrantEntry);
                    self.needs_redraw = true;
                }
                KeyCode::Char('x') | KeyCode::Char('X') => {
                    self.decision_panel.select_button(DecisionButton::GrantExit);
                    self.needs_redraw = true;
                }
                KeyCode::Char('b') | KeyCode::Char('B') => {
                    self.decision_panel.select_button(DecisionButton::GrantBoth);
                    self.needs_redraw = true;
                }
                KeyCode::Char('d') | KeyCode::Char('D') => {
                    self.decision_panel.select_button(DecisionButton::Deny);
                    self.needs_redraw = true;
                }
                KeyCode::Tab => {
                    // Toggle focus between message and timeout fields
                    if self.decision_panel.is_focused(InputField::Message) {
                        self.decision_panel.focus_field(InputField::Timeout);
                    } else {
                        self.decision_panel.focus_field(InputField::Message);
                    }
                    self.needs_redraw = true;
                }
                KeyCode::Esc => {
                    self.decision_panel.unfocus();
                    self.needs_redraw = true;
                }
                KeyCode::Enter => {
                    self.send_response();
                    self.needs_redraw = true;
                }
                KeyCode::Backspace => {
                    self.decision_panel.backspace();
                    self.needs_redraw = true;
                }
                KeyCode::Char(c) => {
                    self.decision_panel.append_char(c);
                    self.needs_redraw = true;
                }
                KeyCode::Up => {
                    // Only scroll if not editing fields
                    if !self.decision_panel.is_focused(InputField::Message)
                        && !self.decision_panel.is_focused(InputField::Timeout)
                    {
                        self.history_panel.scroll_up();
                        self.needs_redraw = true;
                    }
                }
                KeyCode::Down => {
                    // Only scroll if not editing fields
                    if !self.decision_panel.is_focused(InputField::Message)
                        && !self.decision_panel.is_focused(InputField::Timeout)
                    {
                        self.history_panel.scroll_down();
                        self.needs_redraw = true;
                    }
                }
                KeyCode::PageUp => {
                    self.history_panel.scroll_to_top();
                    self.needs_redraw = true;
                }
                KeyCode::PageDown => {
                    self.history_panel.scroll_to_bottom();
                    self.needs_redraw = true;
                }
                KeyCode::Home => {
                    self.history_panel.scroll_to_top();
                    self.needs_redraw = true;
                }
                KeyCode::End => {
                    self.history_panel.scroll_to_bottom();
                    self.needs_redraw = true;
                }
                _ => {}
            }
        }
    }

    /// Handle incoming message from turnstile.
    async fn handle_message(&mut self, device_id: turnkey_core::DeviceId, message: Message) {
        // Check if this is an access request
        if message.command == CommandCode::AccessRequest {
            // Convert FieldData to Vec<String> for parsing
            let fields: Vec<String> = message
                .fields
                .iter()
                .map(|f| f.as_str().to_string())
                .collect();
            match AccessRequest::parse(&fields) {
                Ok(request) => {
                    info!(
                        "Access request from device {}: card={}",
                        device_id.as_u8(),
                        request.card_number()
                    );

                    self.request_panel.set_request(device_id, &request);
                    self.needs_redraw = true;
                }
                Err(e) => {
                    error!("Failed to parse access request: {}", e);
                }
            }
        }
    }

    /// Send response to current request.
    ///
    /// Builds and sends an access response message to the turnstile,
    /// adds the decision to history, and clears the current request.
    fn send_response(&mut self) {
        if !self.request_panel.has_request() {
            return;
        }

        // Get device ID from current request
        let device_id = match self.request_panel.current_device_id() {
            Some(id) => id,
            None => {
                error!("Cannot send response: no device ID for current request");
                return;
            }
        };

        // Get card number from current request (use empty string if missing)
        let card_number = self
            .request_panel
            .current_card_number()
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                debug!("No card number in current request, using empty string for history");
                String::new()
            });

        // Build response from decision panel
        let response = self.decision_panel.build_response();

        // Map AccessDecision to CommandCode
        let command_code = match response.decision() {
            turnkey_protocol::commands::access::AccessDecision::GrantEntry => {
                CommandCode::GrantEntry
            }
            turnkey_protocol::commands::access::AccessDecision::GrantExit => CommandCode::GrantExit,
            turnkey_protocol::commands::access::AccessDecision::GrantBoth => CommandCode::GrantBoth,
            turnkey_protocol::commands::access::AccessDecision::Deny => CommandCode::DenyAccess,
        };

        // Build timeout field
        let timeout_field = match FieldData::new(response.timeout_seconds().to_string()) {
            Ok(field) => field,
            Err(e) => {
                error!("Failed to create timeout field: {}", e);
                return;
            }
        };

        // Build message field (should always succeed since we validate input)
        let message_field = match FieldData::new(response.display_message().to_string()) {
            Ok(field) => field,
            Err(e) => {
                error!(
                    "Failed to create message field (contains protocol delimiters): {}",
                    e
                );
                // Use fallback message
                match FieldData::new("Erro na mensagem".to_string()) {
                    Ok(fallback) => fallback,
                    Err(_) => {
                        error!("Fatal: cannot create even fallback message");
                        return;
                    }
                }
            }
        };

        // Build protocol message
        let message = MessageBuilder::new(device_id, command_code)
            .field(timeout_field)
            .field(message_field)
            .build();

        let message = match message {
            Ok(msg) => msg,
            Err(e) => {
                error!("Failed to build response message: {}", e);
                return;
            }
        };

        // Queue response for sending
        // (actual send happens in main loop where we have &mut server)
        self.pending_response = Some((device_id, message));

        // Add to history
        self.history_panel.add_entry(
            device_id,
            card_number,
            response.decision(),
            response.timeout_seconds(),
        );

        // Clear current request and reset decision panel
        self.request_panel.clear();
        self.decision_panel.reset();
    }

    /// Render the TUI.
    fn render(&self, frame: &mut ratatui::Frame) {
        let size = frame.area();

        // Check terminal size
        if size.width < MIN_TERMINAL_WIDTH || size.height < MIN_TERMINAL_HEIGHT {
            self.render_size_warning(frame, size);
            return;
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Header
                Constraint::Length(5),  // Connected devices
                Constraint::Length(7),  // Current request
                Constraint::Length(12), // Decision panel
                Constraint::Min(5),     // History
                Constraint::Length(1),  // Help line
            ])
            .split(size);

        // Render header
        self.render_header(frame, chunks[0]);

        // Render components
        self.devices_panel.render(frame, chunks[1]);
        self.request_panel.render(frame, chunks[2]);
        self.decision_panel.render(frame, chunks[3]);
        self.history_panel.render(frame, chunks[4]);

        // Render help line
        self.render_help(frame, chunks[5]);
    }

    fn render_header(&self, frame: &mut ratatui::Frame, area: Rect) {
        use ratatui::style::{Color, Modifier, Style};

        let status_indicator = match &self.server_state {
            ServerState::Listening => ("LISTENING", Color::Green),
            ServerState::Error(_) => ("ERROR", Color::Red),
        };

        let mut text_parts = vec![
            Span::raw("CLIENT EMULATOR | "),
            Span::styled(
                status_indicator.0,
                Style::default()
                    .fg(status_indicator.1)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!(" | {}", self.bind_addr)),
        ];

        // Add error message if present
        if let Some(ref error) = self.last_error {
            text_parts.push(Span::raw(" | "));
            text_parts.push(Span::styled(error, Style::default().fg(Color::Red)));
        }

        let line = Line::from(text_parts);
        let paragraph = Paragraph::new(line).style(header_style());
        frame.render_widget(paragraph, area);
    }

    fn render_help(&self, frame: &mut ratatui::Frame, area: Rect) {
        let help = Line::from(
            " [E/X/B/D] Decision | [Tab] Edit | [Enter] Send | [↑↓/PgUp/PgDn] Scroll | [Q] Quit",
        );
        let paragraph = Paragraph::new(help);
        frame.render_widget(paragraph, area);
    }

    fn render_size_warning(&self, frame: &mut ratatui::Frame, area: Rect) {
        let text = format!(
            "Terminal too small! Required: {}x{}, Current: {}x{}",
            MIN_TERMINAL_WIDTH, MIN_TERMINAL_HEIGHT, area.width, area.height
        );
        let paragraph = Paragraph::new(text).block(Block::bordered().title("⚠ Warning"));
        frame.render_widget(paragraph, area);
    }
}

/// Setup terminal for TUI.
fn setup_terminal() -> io::Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend)
}

/// Cleanup terminal after TUI exits.
fn cleanup_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

/// Run the client-emulator TUI.
/// Run the client-emulator TUI application.
///
/// Sets up terminal with panic handler to ensure proper cleanup
/// even if the application panics.
pub async fn run_client_tui(bind_addr: SocketAddr) -> anyhow::Result<()> {
    // Set up panic hook to restore terminal on panic
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        // Attempt to restore terminal
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);

        // Call original panic hook
        original_hook(panic_info);
    }));

    let mut app = App::new(bind_addr).await?;
    let result = app.run().await;

    // Restore original panic hook
    let _ = std::panic::take_hook();

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants() {
        // Verify terminal size constants are reasonable
        const _: () = assert!(MIN_TERMINAL_WIDTH >= 80);
        const _: () = assert!(MIN_TERMINAL_HEIGHT >= 24);
        assert_eq!(FRAME_DURATION_MS, 16);
    }
}
