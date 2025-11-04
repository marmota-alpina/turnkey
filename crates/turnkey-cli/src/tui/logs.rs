//! Scrollable logs panel component.
//!
//! Displays real-time event logs with timestamps and color coding.

use chrono::{DateTime, Local};
use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
};

use super::theme::{LogLevel, log_style};

/// Maximum number of log entries to keep in memory.
const MAX_LOG_ENTRIES: usize = 1000;

/// A single log entry with timestamp, level, and message.
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: DateTime<Local>,
    pub level: LogLevel,
    pub message: String,
}

impl LogEntry {
    /// Create a new log entry.
    pub fn new(level: LogLevel, message: String) -> Self {
        Self {
            timestamp: Local::now(),
            level,
            message,
        }
    }

    /// Format the log entry as a line with timestamp and colored text.
    pub fn to_line(&self) -> Line<'static> {
        let timestamp = format!("[{}]", self.timestamp.format("%H:%M:%S"));
        let style = log_style(self.level);

        Line::from(vec![
            Span::raw(timestamp),
            Span::raw(" "),
            Span::styled(self.message.clone(), style),
        ])
    }
}

/// Logs panel state.
pub struct LogsPanel {
    /// Log entries (ring buffer).
    entries: Vec<LogEntry>,
    /// List state for scrolling.
    state: ListState,
    /// Auto-scroll to bottom when new entries arrive.
    auto_scroll: bool,
}

impl Default for LogsPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl LogsPanel {
    /// Create a new logs panel.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            state: ListState::default(),
            auto_scroll: true,
        }
    }

    /// Add a log entry.
    pub fn add_entry(&mut self, level: LogLevel, message: String) {
        self.entries.push(LogEntry::new(level, message));

        // Limit entries to MAX_LOG_ENTRIES
        if self.entries.len() > MAX_LOG_ENTRIES {
            self.entries
                .drain(0..(self.entries.len() - MAX_LOG_ENTRIES));
        }

        // Auto-scroll to bottom if enabled
        if self.auto_scroll && !self.entries.is_empty() {
            self.state.select(Some(self.entries.len() - 1));
        }
    }

    /// Add an info log entry.
    pub fn info(&mut self, message: String) {
        self.add_entry(LogLevel::Info, message);
    }

    /// Add a success log entry.
    pub fn success(&mut self, message: String) {
        self.add_entry(LogLevel::Success, message);
    }

    /// Add a warning log entry.
    pub fn warning(&mut self, message: String) {
        self.add_entry(LogLevel::Warning, message);
    }

    /// Add an error log entry.
    pub fn error(&mut self, message: String) {
        self.add_entry(LogLevel::Error, message);
    }

    /// Scroll up.
    pub fn scroll_up(&mut self) {
        self.auto_scroll = false;
        if let Some(selected) = self.state.selected() {
            if selected > 0 {
                self.state.select(Some(selected - 1));
            }
        } else if !self.entries.is_empty() {
            self.state.select(Some(self.entries.len() - 1));
        }
    }

    /// Scroll down.
    pub fn scroll_down(&mut self) {
        if let Some(selected) = self.state.selected() {
            if selected < self.entries.len().saturating_sub(1) {
                self.state.select(Some(selected + 1));
            } else {
                self.auto_scroll = true;
            }
        } else if !self.entries.is_empty() {
            self.state.select(Some(0));
        }
    }

    /// Scroll to top.
    pub fn scroll_to_top(&mut self) {
        self.auto_scroll = false;
        if !self.entries.is_empty() {
            self.state.select(Some(0));
        }
    }

    /// Scroll to bottom.
    pub fn scroll_to_bottom(&mut self) {
        self.auto_scroll = true;
        if !self.entries.is_empty() {
            self.state.select(Some(self.entries.len() - 1));
        }
    }

    /// Render the logs panel.
    pub fn render(&mut self, f: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self
            .entries
            .iter()
            .map(|e| ListItem::new(e.to_line()))
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("LOGS")
                    .title_bottom("Tab: Focus | \u{2191}\u{2193}: Nav"),
            )
            .style(Style::default());

        f.render_stateful_widget(list, area, &mut self.state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logs_panel_new() {
        let panel = LogsPanel::new();
        assert_eq!(panel.entries.len(), 0);
        assert!(panel.auto_scroll);
    }

    #[test]
    fn test_add_entry() {
        let mut panel = LogsPanel::new();
        panel.add_entry(LogLevel::Info, "test message".to_string());
        assert_eq!(panel.entries.len(), 1);
        assert_eq!(panel.entries[0].message, "test message");
    }

    #[test]
    fn test_max_entries() {
        let mut panel = LogsPanel::new();
        for i in 0..1100 {
            panel.add_entry(LogLevel::Info, format!("message {}", i));
        }
        assert_eq!(panel.entries.len(), MAX_LOG_ENTRIES);
    }

    #[test]
    fn test_scroll_up() {
        let mut panel = LogsPanel::new();
        for i in 0..10 {
            panel.add_entry(LogLevel::Info, format!("message {}", i));
        }
        panel.scroll_up();
        assert_eq!(panel.state.selected(), Some(8));
    }

    #[test]
    fn test_scroll_down() {
        let mut panel = LogsPanel::new();
        for i in 0..10 {
            panel.add_entry(LogLevel::Info, format!("message {}", i));
        }
        panel.state.select(Some(0));
        panel.auto_scroll = false;
        panel.scroll_down();
        assert_eq!(panel.state.selected(), Some(1));
    }

    #[test]
    fn test_auto_scroll() {
        let mut panel = LogsPanel::new();
        panel.add_entry(LogLevel::Info, "first".to_string());
        assert_eq!(panel.state.selected(), Some(0));
        panel.add_entry(LogLevel::Info, "second".to_string());
        assert_eq!(panel.state.selected(), Some(1));
    }
}
