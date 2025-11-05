//! Virtual keypad component.
//!
//! Renders a 4x3 numeric keypad with action buttons.

use std::time::{Duration, Instant};

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use super::theme::{keypad_action_style, keypad_normal_style, keypad_pressed_style};

/// Keypad layout: 4 rows x 3 columns.
const KEYPAD_LAYOUT: [[char; 3]; 4] = [
    ['1', '2', '3'],
    ['4', '5', '6'],
    ['7', '8', '9'],
    ['*', '0', '#'],
];

/// Duration to show pressed key highlight (milliseconds).
const PRESS_HIGHLIGHT_DURATION_MS: u64 = 200;

/// Virtual keypad component.
pub struct KeypadComponent {
    /// Last pressed key with timestamp for visual feedback.
    last_pressed: Option<(char, Instant)>,
}

impl Default for KeypadComponent {
    fn default() -> Self {
        Self::new()
    }
}

impl KeypadComponent {
    /// Create a new keypad component.
    pub fn new() -> Self {
        Self { last_pressed: None }
    }

    /// Set the last pressed key for visual feedback.
    pub fn set_last_pressed(&mut self, key: char) {
        self.last_pressed = Some((key, Instant::now()));
    }

    /// Check if a key is currently highlighted.
    fn is_key_pressed(&self, key: char) -> bool {
        self.last_pressed
            .map(|(k, instant)| {
                k == key && instant.elapsed() < Duration::from_millis(PRESS_HIGHLIGHT_DURATION_MS)
            })
            .unwrap_or(false)
    }

    /// Render the keypad.
    pub fn render(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title("TECLADO NUMERICO");

        let inner = block.inner(area);
        f.render_widget(block, area);

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(inner);

        self.render_digit_grid(f, rows[0]);
        self.render_action_buttons(f, rows[1]);
    }

    /// Render the 4x3 digit grid.
    fn render_digit_grid(&self, f: &mut Frame, area: Rect) {
        let row_constraints = vec![Constraint::Percentage(25); 4];
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints(row_constraints)
            .split(area);

        for (row_idx, row_keys) in KEYPAD_LAYOUT.iter().enumerate() {
            let col_constraints = vec![Constraint::Percentage(33); 3];
            let cols = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(col_constraints)
                .split(rows[row_idx]);

            for (col_idx, &key) in row_keys.iter().enumerate() {
                self.render_key(f, cols[col_idx], key);
            }
        }
    }

    /// Render a single key.
    fn render_key(&self, f: &mut Frame, area: Rect, key: char) {
        let style = if self.is_key_pressed(key) {
            keypad_pressed_style()
        } else {
            keypad_normal_style()
        };

        let text = format!(" {} ", key);
        let paragraph = Paragraph::new(Line::from(Span::styled(text, style)))
            .block(Block::default().borders(Borders::ALL))
            .alignment(Alignment::Center);

        f.render_widget(paragraph, area);
    }

    /// Render action buttons (ENTER, CANCEL, CLEAR).
    fn render_action_buttons(&self, f: &mut Frame, area: Rect) {
        let constraints = vec![Constraint::Percentage(33); 3];
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(constraints)
            .split(area);

        self.render_action_button(f, cols[0], "ENTER");
        self.render_action_button(f, cols[1], "CANCEL");
        self.render_action_button(f, cols[2], "CLEAR");
    }

    /// Render a single action button.
    fn render_action_button(&self, f: &mut Frame, area: Rect, label: &str) {
        let style = keypad_action_style();
        let paragraph = Paragraph::new(Line::from(Span::styled(label, style)))
            .block(Block::default().borders(Borders::ALL))
            .alignment(Alignment::Center);

        f.render_widget(paragraph, area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keypad_component_new() {
        let keypad = KeypadComponent::new();
        assert!(keypad.last_pressed.is_none());
    }

    #[test]
    fn test_set_last_pressed() {
        let mut keypad = KeypadComponent::new();
        keypad.set_last_pressed('5');
        assert!(keypad.last_pressed.is_some());
        let (key, _) = keypad.last_pressed.unwrap();
        assert_eq!(key, '5');
    }

    #[test]
    fn test_is_key_pressed() {
        let mut keypad = KeypadComponent::new();
        assert!(!keypad.is_key_pressed('5'));

        keypad.set_last_pressed('5');
        assert!(keypad.is_key_pressed('5'));
        assert!(!keypad.is_key_pressed('3'));

        // Wait for highlight duration to expire
        std::thread::sleep(Duration::from_millis(PRESS_HIGHLIGHT_DURATION_MS + 10));
        assert!(!keypad.is_key_pressed('5'));
    }

    #[test]
    fn test_keypad_layout() {
        assert_eq!(KEYPAD_LAYOUT[0], ['1', '2', '3']);
        assert_eq!(KEYPAD_LAYOUT[1], ['4', '5', '6']);
        assert_eq!(KEYPAD_LAYOUT[2], ['7', '8', '9']);
        assert_eq!(KEYPAD_LAYOUT[3], ['*', '0', '#']);
    }
}
