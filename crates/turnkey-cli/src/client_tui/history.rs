//! History panel component.
//!
//! Displays a scrollable log of all validation decisions made by the operator.

use std::collections::VecDeque;

use chrono::{DateTime, Local};
use ratatui::{
    Frame,
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
};
use turnkey_core::DeviceId;
use turnkey_protocol::commands::access::AccessDecision;

use super::theme::{LogLevel, log_style, panel_border_style};

/// Maximum number of history entries to keep.
const MAX_HISTORY_ENTRIES: usize = 1000;

/// Represents a single validation decision in history.
#[derive(Debug, Clone)]
pub struct HistoryEntry {
    /// Timestamp when decision was made.
    timestamp: DateTime<Local>,
    /// Device ID that made the request.
    device_id: DeviceId,
    /// Card number from the request.
    card_number: String,
    /// Decision made by operator.
    decision: AccessDecision,
    /// Timeout sent with response.
    timeout_seconds: u8,
}

impl HistoryEntry {
    /// Create a new history entry.
    pub fn new(
        device_id: DeviceId,
        card_number: String,
        decision: AccessDecision,
        timeout_seconds: u8,
    ) -> Self {
        Self {
            timestamp: Local::now(),
            device_id,
            card_number,
            decision,
            timeout_seconds,
        }
    }

    /// Format entry as display string.
    fn format(&self) -> String {
        let time = self.timestamp.format("%H:%M:%S");
        let icon = if self.decision.is_grant() {
            "✅"
        } else {
            "❌"
        };
        let decision_str = match self.decision {
            AccessDecision::GrantEntry => "GRANT ENTRY",
            AccessDecision::GrantExit => "GRANT EXIT",
            AccessDecision::GrantBoth => "GRANT BOTH",
            AccessDecision::Deny => "DENY ACCESS",
        };

        format!(
            "[{}] Dev:{:02} {} → {} {} ({}s)",
            time,
            self.device_id.as_u8(),
            self.card_number,
            icon,
            decision_str,
            self.timeout_seconds
        )
    }

    /// Get log level for styling based on decision.
    fn log_level(&self) -> LogLevel {
        if self.decision.is_grant() {
            LogLevel::Success
        } else {
            LogLevel::Warning
        }
    }
}

/// Component for displaying validation history.
#[derive(Debug)]
pub struct HistoryPanel {
    /// Ring buffer of history entries.
    entries: VecDeque<HistoryEntry>,
    /// Current scroll offset.
    scroll_offset: usize,
}

impl HistoryPanel {
    /// Create a new history panel.
    pub fn new() -> Self {
        Self {
            entries: VecDeque::with_capacity(MAX_HISTORY_ENTRIES),
            scroll_offset: 0,
        }
    }

    /// Add a new entry to history.
    pub fn add_entry(
        &mut self,
        device_id: DeviceId,
        card_number: String,
        decision: AccessDecision,
        timeout_seconds: u8,
    ) {
        let entry = HistoryEntry::new(device_id, card_number, decision, timeout_seconds);

        if self.entries.len() >= MAX_HISTORY_ENTRIES {
            self.entries.pop_front();
        }

        self.entries.push_back(entry);

        // Auto-scroll to bottom when new entry is added
        self.scroll_to_bottom();
    }

    /// Get total number of entries.
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// Scroll up by one line.
    pub fn scroll_up(&mut self) {
        if self.scroll_offset > 0 {
            self.scroll_offset -= 1;
        }
    }

    /// Scroll down by one line.
    pub fn scroll_down(&mut self) {
        let max_offset = self.entries.len().saturating_sub(1);
        if self.scroll_offset < max_offset {
            self.scroll_offset += 1;
        }
    }

    /// Scroll to the top.
    pub fn scroll_to_top(&mut self) {
        self.scroll_offset = 0;
    }

    /// Scroll to the bottom.
    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = self.entries.len().saturating_sub(1);
    }

    /// Render the history panel.
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self
            .entries
            .iter()
            .map(|entry| {
                let line = Line::from(Span::styled(entry.format(), log_style(entry.log_level())));
                ListItem::new(line)
            })
            .collect();

        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(panel_border_style())
                .title(format!("HISTORY ({} entries)", self.entries.len())),
        );

        frame.render_widget(list, area);
    }
}

impl Default for HistoryPanel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_panel_creation() {
        let panel = HistoryPanel::new();
        assert_eq!(panel.entry_count(), 0);
    }

    #[test]
    fn test_add_entry() {
        let mut panel = HistoryPanel::new();
        let device_id = DeviceId::new(1).unwrap();

        panel.add_entry(device_id, "1234".to_string(), AccessDecision::GrantExit, 5);

        assert_eq!(panel.entry_count(), 1);
    }

    #[test]
    fn test_max_entries_limit() {
        let mut panel = HistoryPanel::new();
        let device_id = DeviceId::new(1).unwrap();

        // Add more than max
        for i in 0..MAX_HISTORY_ENTRIES + 10 {
            panel.add_entry(device_id, format!("{}", i), AccessDecision::GrantExit, 5);
        }

        assert_eq!(panel.entry_count(), MAX_HISTORY_ENTRIES);
    }

    #[test]
    fn test_scroll_up_down() {
        let mut panel = HistoryPanel::new();
        let device_id = DeviceId::new(1).unwrap();

        for i in 0..10 {
            panel.add_entry(device_id, format!("{}", i), AccessDecision::GrantExit, 5);
        }

        panel.scroll_to_top();
        assert_eq!(panel.scroll_offset, 0);

        panel.scroll_down();
        assert_eq!(panel.scroll_offset, 1);

        panel.scroll_up();
        assert_eq!(panel.scroll_offset, 0);
    }

    #[test]
    fn test_scroll_to_bottom() {
        let mut panel = HistoryPanel::new();
        let device_id = DeviceId::new(1).unwrap();

        for i in 0..10 {
            panel.add_entry(device_id, format!("{}", i), AccessDecision::GrantExit, 5);
        }

        panel.scroll_to_bottom();
        assert_eq!(panel.scroll_offset, 9);
    }

    #[test]
    fn test_history_entry_format_grant() {
        let device_id = DeviceId::new(15).unwrap();
        let entry = HistoryEntry::new(device_id, "1234".to_string(), AccessDecision::GrantExit, 5);

        let formatted = entry.format();
        assert!(formatted.contains("Dev:15"));
        assert!(formatted.contains("1234"));
        assert!(formatted.contains("GRANT EXIT"));
        assert!(formatted.contains("5s"));
    }

    #[test]
    fn test_history_entry_format_deny() {
        let device_id = DeviceId::new(1).unwrap();
        let entry = HistoryEntry::new(device_id, "9999".to_string(), AccessDecision::Deny, 0);

        let formatted = entry.format();
        assert!(formatted.contains("DENY ACCESS"));
    }
}
