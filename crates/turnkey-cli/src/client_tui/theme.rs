//! Color themes and styling for client-emulator TUI components.
//!
//! Reuses theme system from turnstile TUI with additional styles
//! for validation server specific components.

pub use crate::tui::theme::{
    LogLevel, Theme, keypad_action_style, keypad_normal_style, log_style, status_bar_style,
};
use ratatui::style::{Color, Modifier, Style};

/// Get style for decision button normal state.
pub fn decision_button_normal_style() -> Style {
    Style::default()
        .bg(Color::DarkGray)
        .fg(Color::White)
        .add_modifier(Modifier::BOLD)
}

/// Get style for decision button selected state.
pub fn decision_button_selected_style() -> Style {
    Style::default()
        .bg(Color::Yellow)
        .fg(Color::Black)
        .add_modifier(Modifier::BOLD)
}

/// Get style for connected device indicator.
pub fn device_connected_style() -> Style {
    Style::default()
        .fg(Color::Green)
        .add_modifier(Modifier::BOLD)
}

/// Get style for disconnected device indicator.
pub fn device_disconnected_style() -> Style {
    Style::default().fg(Color::Red)
}

/// Get style for header bar.
pub fn header_style() -> Style {
    Style::default()
        .bg(Color::Blue)
        .fg(Color::White)
        .add_modifier(Modifier::BOLD)
}

/// Get style for panel borders.
pub fn panel_border_style() -> Style {
    Style::default().fg(Color::White)
}

/// Get style for editable input field.
pub fn input_field_style() -> Style {
    Style::default().bg(Color::DarkGray).fg(Color::White)
}

/// Get style for input field focus.
pub fn input_field_focused_style() -> Style {
    Style::default()
        .bg(Color::Blue)
        .fg(Color::White)
        .add_modifier(Modifier::BOLD)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decision_button_styles() {
        let normal = decision_button_normal_style();
        assert_eq!(normal.bg, Some(Color::DarkGray));

        let selected = decision_button_selected_style();
        assert_eq!(selected.bg, Some(Color::Yellow));
    }

    #[test]
    fn test_device_styles() {
        let connected = device_connected_style();
        assert_eq!(connected.fg, Some(Color::Green));

        let disconnected = device_disconnected_style();
        assert_eq!(disconnected.fg, Some(Color::Red));
    }

    #[test]
    fn test_input_styles() {
        let normal = input_field_style();
        assert_eq!(normal.bg, Some(Color::DarkGray));

        let focused = input_field_focused_style();
        assert_eq!(focused.bg, Some(Color::Blue));
    }
}
