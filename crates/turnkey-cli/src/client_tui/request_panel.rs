//! Current request display panel component.
//!
//! Shows details of the access request currently being validated by the operator.

use ratatui::{
    Frame,
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use turnkey_core::{AccessDirection, DeviceId};
use turnkey_protocol::commands::access::AccessRequest;

use super::theme::panel_border_style;

/// Component for displaying current access request.
#[derive(Debug)]
pub struct RequestPanel {
    /// Current request being displayed.
    current_request: Option<RequestData>,
}

/// Data extracted from access request for display.
#[derive(Debug, Clone)]
struct RequestData {
    device_id: DeviceId,
    card_number: String,
    timestamp: String,
    direction: AccessDirection,
}

impl RequestPanel {
    /// Create a new request panel.
    pub fn new() -> Self {
        Self {
            current_request: None,
        }
    }

    /// Set the current request to display.
    pub fn set_request(&mut self, device_id: DeviceId, request: &AccessRequest) {
        self.current_request = Some(RequestData {
            device_id,
            card_number: request.card_number().to_string(),
            timestamp: request.timestamp().to_string(),
            direction: request.direction(),
        });
    }

    /// Clear the current request.
    pub fn clear(&mut self) {
        self.current_request = None;
    }

    /// Check if there is a current request.
    pub fn has_request(&self) -> bool {
        self.current_request.is_some()
    }

    /// Get the device ID of the current request.
    pub fn current_device_id(&self) -> Option<DeviceId> {
        self.current_request.as_ref().map(|req| req.device_id)
    }

    /// Get the card number of the current request.
    pub fn current_card_number(&self) -> Option<&str> {
        self.current_request
            .as_ref()
            .map(|req| req.card_number.as_str())
    }

    /// Render the request panel.
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let text = if let Some(ref req) = self.current_request {
            let direction_str = match req.direction {
                AccessDirection::Entry => "Entry",
                AccessDirection::Exit => "Exit",
                AccessDirection::Undefined => "Undefined",
            };

            vec![
                Line::from(vec![
                    Span::raw("Device ID: "),
                    Span::raw(format!("{:02}", req.device_id.as_u8())),
                ]),
                Line::from(vec![
                    Span::raw("Card/Code: "),
                    Span::raw(req.card_number.clone()),
                ]),
                Line::from(vec![
                    Span::raw("Timestamp: "),
                    Span::raw(req.timestamp.clone()),
                ]),
                Line::from(vec![Span::raw("Direction: "), Span::raw(direction_str)]),
            ]
        } else {
            vec![Line::from(Span::raw("No pending requests"))]
        };

        let paragraph = Paragraph::new(text).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(panel_border_style())
                .title("CURRENT REQUEST"),
        );

        frame.render_widget(paragraph, area);
    }
}

impl Default for RequestPanel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use turnkey_protocol::commands::access::AccessRequest;

    fn create_test_request() -> AccessRequest {
        AccessRequest::parse(&[
            "1234".to_string(),
            "25/10/2025 14:30:05".to_string(),
            "1".to_string(),
            "1".to_string(),
        ])
        .unwrap()
    }

    #[test]
    fn test_panel_creation() {
        let panel = RequestPanel::new();
        assert!(!panel.has_request());
    }

    #[test]
    fn test_set_request() {
        let mut panel = RequestPanel::new();
        let device_id = DeviceId::new(1).unwrap();
        let request = create_test_request();

        panel.set_request(device_id, &request);
        assert!(panel.has_request());
    }

    #[test]
    fn test_clear_request() {
        let mut panel = RequestPanel::new();
        let device_id = DeviceId::new(1).unwrap();
        let request = create_test_request();

        panel.set_request(device_id, &request);
        assert!(panel.has_request());

        panel.clear();
        assert!(!panel.has_request());
    }
}
