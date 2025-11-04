//! Status bar component.
//!
//! Displays reader status and event count at the bottom of the emulator column.

use ratatui::{
    Frame,
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use turnkey_core::ReaderType;

use super::theme::status_bar_style;

/// Status bar state.
#[derive(Debug, Clone)]
pub struct StatusBar {
    /// RFID reader enabled.
    pub rfid_enabled: bool,
    /// Biometric reader enabled.
    pub biometric_enabled: bool,
    /// Keypad enabled.
    pub keypad_enabled: bool,
    /// Event count.
    pub event_count: usize,
}

impl Default for StatusBar {
    fn default() -> Self {
        Self {
            rfid_enabled: true,
            keypad_enabled: true,
            biometric_enabled: false,
            event_count: 0,
        }
    }
}

impl StatusBar {
    /// Create a new status bar.
    pub fn new() -> Self {
        Self::default()
    }

    /// Update reader status based on reader type.
    #[allow(dead_code)]
    pub fn enable_reader(&mut self, reader_type: ReaderType) {
        match reader_type {
            ReaderType::Rfid => self.rfid_enabled = true,
            ReaderType::Biometric => self.biometric_enabled = true,
        }
    }

    /// Increment event count.
    pub fn increment_event_count(&mut self) {
        self.event_count = self.event_count.saturating_add(1);
    }

    /// Render the status bar.
    pub fn render(&self, f: &mut Frame, area: Rect) {
        let rfid_status = if self.rfid_enabled {
            "\u{2713}"
        } else {
            "\u{2717}"
        };
        let bio_status = if self.biometric_enabled {
            "\u{2713}"
        } else {
            "\u{2717}"
        };
        let keypad_status = if self.keypad_enabled {
            "\u{2713}"
        } else {
            "\u{2717}"
        };

        let line1 = Line::from(vec![
            Span::raw("Leitoras: RFID"),
            Span::raw(rfid_status),
            Span::raw(" BIO"),
            Span::raw(bio_status),
            Span::raw(" KEYPAD"),
            Span::raw(keypad_status),
        ]);

        let line2 = Line::from(vec![Span::raw(format!("Eventos: {}", self.event_count))]);

        let paragraph = Paragraph::new(vec![line1, line2])
            .block(Block::default().borders(Borders::ALL))
            .style(status_bar_style());

        f.render_widget(paragraph, area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_bar_new() {
        let status = StatusBar::new();
        assert!(status.rfid_enabled);
        assert!(status.keypad_enabled);
        assert!(!status.biometric_enabled);
        assert_eq!(status.event_count, 0);
    }

    #[test]
    fn test_enable_reader() {
        let mut status = StatusBar::new();
        status.biometric_enabled = false;
        status.enable_reader(ReaderType::Biometric);
        assert!(status.biometric_enabled);
    }

    #[test]
    fn test_increment_event_count() {
        let mut status = StatusBar::new();
        status.increment_event_count();
        assert_eq!(status.event_count, 1);
        status.increment_event_count();
        assert_eq!(status.event_count, 2);
    }
}
