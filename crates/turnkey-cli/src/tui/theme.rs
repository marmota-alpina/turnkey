//! Color themes and styling for TUI components.
//!
//! Provides predefined color schemes and style utilities for consistent
//! visual appearance across all TUI components.

use ratatui::style::{Color, Modifier, Style};

/// Theme for TUI components.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    /// Default theme: Blue background, white text (Primme SF style)
    Default,
    /// Dark theme: Black background, green text
    #[allow(dead_code)]
    Dark,
    /// Light theme: White background, black text
    #[allow(dead_code)]
    Light,
    /// LCD-style theme: Black background, green text
    #[allow(dead_code)]
    Lcd,
}

impl Default for Theme {
    fn default() -> Self {
        Self::Default
    }
}

impl Theme {
    /// Get the LCD display style for this theme.
    pub fn lcd_display_style(self) -> Style {
        match self {
            Self::Default => Style::default()
                .bg(Color::Blue)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
            Self::Dark => Style::default()
                .bg(Color::Black)
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
            Self::Light => Style::default().bg(Color::White).fg(Color::Black),
            Self::Lcd => Style::default()
                .bg(Color::Black)
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        }
    }

    /// Get the granted access display style (green background).
    pub fn access_granted_style(self) -> Style {
        Style::default()
            .bg(Color::Green)
            .fg(Color::White)
            .add_modifier(Modifier::BOLD)
    }

    /// Get the denied access display style (red background).
    pub fn access_denied_style(self) -> Style {
        Style::default()
            .bg(Color::Red)
            .fg(Color::White)
            .add_modifier(Modifier::BOLD)
    }

    /// Get the offline mode display style (yellow background).
    pub fn offline_mode_style(self) -> Style {
        Style::default()
            .bg(Color::Yellow)
            .fg(Color::Black)
            .add_modifier(Modifier::BOLD)
    }
}

/// Get keypad button normal style.
pub fn keypad_normal_style() -> Style {
    Style::default().bg(Color::Gray).fg(Color::Black)
}

/// Get keypad button pressed style.
#[allow(dead_code)]
pub fn keypad_pressed_style() -> Style {
    Style::default()
        .bg(Color::Yellow)
        .fg(Color::Black)
        .add_modifier(Modifier::BOLD)
}

/// Get keypad action button style (ENTER, CANCEL, CLEAR).
pub fn keypad_action_style() -> Style {
    Style::default()
        .bg(Color::DarkGray)
        .fg(Color::White)
        .add_modifier(Modifier::BOLD)
}

/// Get status bar style.
pub fn status_bar_style() -> Style {
    Style::default().bg(Color::DarkGray).fg(Color::White)
}

/// Get log entry style based on level.
pub fn log_style(level: LogLevel) -> Style {
    match level {
        LogLevel::Info => Style::default().fg(Color::White),
        LogLevel::Success => Style::default().fg(Color::Green),
        LogLevel::Warning => Style::default().fg(Color::Yellow),
        LogLevel::Error => Style::default().fg(Color::Red),
    }
}

/// Log level for styling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Info,
    Success,
    Warning,
    Error,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_default() {
        assert_eq!(Theme::default(), Theme::Default);
    }

    #[test]
    fn test_lcd_display_styles() {
        let default_style = Theme::Default.lcd_display_style();
        assert_eq!(default_style.bg, Some(Color::Blue));
        assert_eq!(default_style.fg, Some(Color::White));

        let dark_style = Theme::Dark.lcd_display_style();
        assert_eq!(dark_style.bg, Some(Color::Black));
        assert_eq!(dark_style.fg, Some(Color::Green));
    }

    #[test]
    fn test_access_styles() {
        let granted = Theme::Default.access_granted_style();
        assert_eq!(granted.bg, Some(Color::Green));

        let denied = Theme::Default.access_denied_style();
        assert_eq!(denied.bg, Some(Color::Red));
    }

    #[test]
    fn test_log_styles() {
        let info = log_style(LogLevel::Info);
        assert_eq!(info.fg, Some(Color::White));

        let error = log_style(LogLevel::Error);
        assert_eq!(error.fg, Some(Color::Red));
    }
}
