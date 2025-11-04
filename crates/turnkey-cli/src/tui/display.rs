//! LCD display component.
//!
//! Renders a 2-line x 40-column LCD display with state-aware styling.

use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};
use turnkey_emulator::{DisplayState, OperationMode, TurnstileState};

use super::theme::Theme;

/// LCD display component.
pub struct DisplayComponent {
    theme: Theme,
}

impl Default for DisplayComponent {
    fn default() -> Self {
        Self::new()
    }
}

impl DisplayComponent {
    /// Create a new display component.
    pub fn new() -> Self {
        Self {
            theme: Theme::Default,
        }
    }

    /// Set the theme.
    #[allow(dead_code)]
    pub fn set_theme(&mut self, theme: Theme) {
        self.theme = theme;
    }

    /// Render the LCD display.
    pub fn render(
        &self,
        f: &mut Frame,
        area: Rect,
        display_state: &DisplayState,
        turnstile_state: TurnstileState,
    ) {
        let style = match turnstile_state {
            TurnstileState::Granted => self.theme.access_granted_style(),
            TurnstileState::Denied => self.theme.access_denied_style(),
            _ => {
                if display_state.mode == OperationMode::Offline {
                    self.theme.offline_mode_style()
                } else {
                    self.theme.lcd_display_style()
                }
            }
        };

        let mode_indicator = match display_state.mode {
            OperationMode::Online => "ONLINE",
            OperationMode::Offline => "OFFLINE",
        };

        let ip_info = if let Some(ref ip) = display_state.ip_address {
            format!(" | IP: {}", ip)
        } else {
            String::new()
        };

        let status_line = format!("{} | MOCK{}", mode_indicator, ip_info);

        let line1 = Line::from(Span::raw(display_state.line1.clone()));
        let line2 = Line::from(Span::raw(display_state.line2.clone()));
        let line3 = Line::from(Span::raw(""));
        let line4 = Line::from(Span::raw(status_line));

        let paragraph = Paragraph::new(vec![line1, line2, line3, line4])
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Double)
                    .title("DISPLAY LCD"),
            )
            .style(style)
            .alignment(Alignment::Center);

        f.render_widget(paragraph, area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_component_new() {
        let display = DisplayComponent::new();
        assert_eq!(display.theme, Theme::Default);
    }

    #[test]
    fn test_set_theme() {
        let mut display = DisplayComponent::new();
        display.set_theme(Theme::Dark);
        assert_eq!(display.theme, Theme::Dark);
    }
}
