//! Decision panel component.
//!
//! Provides operator interface for making access control decisions with
//! buttons (G/E/B/D) and editable fields for display message and timeout.

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, Paragraph},
};
use turnkey_core::constants::{
    DEFAULT_DENY_TIMEOUT_SECONDS, DEFAULT_GRANT_TIMEOUT_SECONDS, MAX_DISPLAY_MESSAGE_LENGTH,
};
use turnkey_protocol::commands::access::{AccessDecision, AccessResponse};
use turnkey_protocol::is_protocol_delimiter;

use super::theme::{
    decision_button_normal_style, decision_button_selected_style, input_field_focused_style,
    input_field_style, panel_border_style,
};

/// Represents which decision button is selected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecisionButton {
    GrantEntry,
    GrantExit,
    GrantBoth,
    Deny,
}

/// Represents which input field is focused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputField {
    Message,
    Timeout,
}

/// Component for operator decision input.
#[derive(Debug)]
pub struct DecisionPanel {
    /// Currently selected decision button.
    selected_button: DecisionButton,
    /// Display message input.
    message: String,
    /// Timeout input (in seconds).
    timeout: String,
    /// Currently focused input field.
    focused_field: Option<InputField>,
}

impl DecisionPanel {
    /// Create a new decision panel with default values.
    pub fn new() -> Self {
        Self {
            selected_button: DecisionButton::GrantExit,
            message: "Acesso liberado".to_string(),
            timeout: DEFAULT_GRANT_TIMEOUT_SECONDS.to_string(),
            focused_field: None,
        }
    }

    /// Select a decision button.
    pub fn select_button(&mut self, button: DecisionButton) {
        self.selected_button = button;

        // Update default timeout based on decision
        self.timeout = match button {
            DecisionButton::Deny => DEFAULT_DENY_TIMEOUT_SECONDS.to_string(),
            _ => DEFAULT_GRANT_TIMEOUT_SECONDS.to_string(),
        };
    }

    /// Get the currently selected button.
    pub fn selected_button(&self) -> DecisionButton {
        self.selected_button
    }

    /// Focus an input field for editing.
    pub fn focus_field(&mut self, field: InputField) {
        self.focused_field = Some(field);
    }

    /// Unfocus all input fields.
    pub fn unfocus(&mut self) {
        self.focused_field = None;
    }

    /// Check if a field is focused.
    pub fn is_focused(&self, field: InputField) -> bool {
        self.focused_field == Some(field)
    }

    /// Append character to the focused field.
    ///
    /// For message field, rejects protocol delimiter characters using centralized
    /// validation to prevent protocol injection vulnerabilities.
    ///
    /// Uses `is_protocol_delimiter()` from turnkey-protocol to ensure consistency
    /// across all input validation points in the codebase.
    pub fn append_char(&mut self, c: char) {
        match self.focused_field {
            Some(InputField::Message) => {
                // Reject protocol delimiters to ensure FieldData construction succeeds
                if !is_protocol_delimiter(c) && self.message.len() < MAX_DISPLAY_MESSAGE_LENGTH {
                    self.message.push(c);
                }
            }
            Some(InputField::Timeout) => {
                if self.timeout.len() < 3 && c.is_ascii_digit() {
                    self.timeout.push(c);
                }
            }
            None => {}
        }
    }

    /// Remove last character from the focused field.
    pub fn backspace(&mut self) {
        match self.focused_field {
            Some(InputField::Message) => {
                self.message.pop();
            }
            Some(InputField::Timeout) => {
                self.timeout.pop();
            }
            None => {}
        }
    }

    /// Get the display message.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Get the timeout value.
    pub fn timeout(&self) -> u8 {
        self.timeout
            .parse()
            .unwrap_or(DEFAULT_GRANT_TIMEOUT_SECONDS)
    }

    /// Build an AccessResponse from the current decision and inputs.
    pub fn build_response(&self) -> AccessResponse {
        let decision = match self.selected_button {
            DecisionButton::GrantEntry => AccessDecision::GrantEntry,
            DecisionButton::GrantExit => AccessDecision::GrantExit,
            DecisionButton::GrantBoth => AccessDecision::GrantBoth,
            DecisionButton::Deny => AccessDecision::Deny,
        };

        AccessResponse::new(decision, self.timeout(), self.message.clone())
    }

    /// Reset to default values.
    pub fn reset(&mut self) {
        self.selected_button = DecisionButton::GrantExit;
        self.message = "Acesso liberado".to_string();
        self.timeout = DEFAULT_GRANT_TIMEOUT_SECONDS.to_string();
        self.focused_field = None;
    }

    /// Render the decision panel.
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(panel_border_style())
            .title("DECISION");

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Buttons
                Constraint::Length(1), // Spacing
                Constraint::Length(3), // Message input
                Constraint::Length(3), // Timeout input
            ])
            .split(inner);

        // Render decision buttons
        self.render_buttons(frame, chunks[0]);

        // Render message input
        self.render_message_input(frame, chunks[2]);

        // Render timeout input
        self.render_timeout_input(frame, chunks[3]);
    }

    fn render_buttons(&self, frame: &mut Frame, area: Rect) {
        let button_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
            ])
            .split(area);

        let buttons = [
            ("E", "Grant Entry", DecisionButton::GrantEntry),
            ("X", "Grant Exit", DecisionButton::GrantExit),
            ("B", "Grant Both", DecisionButton::GrantBoth),
            ("D", "Deny Access", DecisionButton::Deny),
        ];

        for (i, (key, label, button)) in buttons.iter().enumerate() {
            let style = if self.selected_button == *button {
                decision_button_selected_style()
            } else {
                decision_button_normal_style()
            };

            let text = format!("[{}] {}", key, label);
            let paragraph = Paragraph::new(text).style(style);

            frame.render_widget(paragraph, button_chunks[i]);
        }
    }

    fn render_message_input(&self, frame: &mut Frame, area: Rect) {
        let style = if self.is_focused(InputField::Message) {
            input_field_focused_style()
        } else {
            input_field_style()
        };

        let remaining = MAX_DISPLAY_MESSAGE_LENGTH - self.message.len();
        let text = format!(
            "Message: [{}{}] ({})",
            self.message,
            "_".repeat(remaining),
            MAX_DISPLAY_MESSAGE_LENGTH
        );

        let paragraph = Paragraph::new(text).style(style);
        frame.render_widget(paragraph, area);
    }

    fn render_timeout_input(&self, frame: &mut Frame, area: Rect) {
        let style = if self.is_focused(InputField::Timeout) {
            input_field_focused_style()
        } else {
            input_field_style()
        };

        let text = format!("Timeout: [{}] seconds", self.timeout);
        let paragraph = Paragraph::new(text).style(style);
        frame.render_widget(paragraph, area);
    }
}

impl Default for DecisionPanel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_panel_creation() {
        let panel = DecisionPanel::new();
        assert_eq!(panel.selected_button(), DecisionButton::GrantExit);
        assert!(!panel.message().is_empty());
    }

    #[test]
    fn test_select_button() {
        let mut panel = DecisionPanel::new();
        panel.select_button(DecisionButton::Deny);
        assert_eq!(panel.selected_button(), DecisionButton::Deny);
    }

    #[test]
    fn test_focus_field() {
        let mut panel = DecisionPanel::new();
        panel.focus_field(InputField::Message);
        assert!(panel.is_focused(InputField::Message));
    }

    #[test]
    fn test_append_char_to_message() {
        let mut panel = DecisionPanel::new();
        panel.focus_field(InputField::Message);
        panel.message = String::new();

        panel.append_char('T');
        panel.append_char('e');
        panel.append_char('s');
        panel.append_char('t');

        assert_eq!(panel.message(), "Test");
    }

    #[test]
    fn test_append_char_to_timeout() {
        let mut panel = DecisionPanel::new();
        panel.focus_field(InputField::Timeout);
        panel.timeout = String::new();

        panel.append_char('1');
        panel.append_char('0');

        assert_eq!(panel.timeout(), 10);
    }

    #[test]
    fn test_backspace() {
        let mut panel = DecisionPanel::new();
        panel.focus_field(InputField::Message);
        panel.message = "Test".to_string();

        panel.backspace();
        assert_eq!(panel.message(), "Tes");
    }

    #[test]
    fn test_build_response() {
        let panel = DecisionPanel::new();
        let response = panel.build_response();

        assert_eq!(response.decision(), AccessDecision::GrantExit);
        assert_eq!(response.timeout_seconds(), DEFAULT_GRANT_TIMEOUT_SECONDS);
    }

    #[test]
    fn test_reset() {
        let mut panel = DecisionPanel::new();
        panel.select_button(DecisionButton::Deny);
        panel.message = "Custom message".to_string();

        panel.reset();

        assert_eq!(panel.selected_button(), DecisionButton::GrantExit);
        assert_eq!(panel.message(), "Acesso liberado");
    }

    #[test]
    fn test_message_length_limit() {
        let mut panel = DecisionPanel::new();
        panel.focus_field(InputField::Message);
        panel.message = "A".repeat(MAX_DISPLAY_MESSAGE_LENGTH);

        panel.append_char('X');
        assert_eq!(panel.message().len(), MAX_DISPLAY_MESSAGE_LENGTH);
    }

    #[test]
    fn test_timeout_only_accepts_digits() {
        let mut panel = DecisionPanel::new();
        panel.focus_field(InputField::Timeout);
        panel.timeout = String::new();

        panel.append_char('a');
        assert_eq!(panel.timeout.len(), 0);

        panel.append_char('5');
        assert_eq!(panel.timeout.len(), 1);
    }
}
