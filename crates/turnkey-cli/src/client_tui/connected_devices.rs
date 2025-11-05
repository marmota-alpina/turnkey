//! Connected devices panel component.
//!
//! Displays list of all turnstiles currently connected to the server,
//! showing their device ID, IP address, and connection uptime.

use ratatui::{
    Frame,
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
};
use turnkey_network::ConnectionInfo;

use super::theme::{device_connected_style, panel_border_style};

/// Component for rendering connected devices list.
#[derive(Debug)]
pub struct ConnectedDevicesPanel {
    /// List of connected devices.
    devices: Vec<ConnectionInfo>,
}

impl ConnectedDevicesPanel {
    /// Create a new connected devices panel.
    pub fn new() -> Self {
        Self {
            devices: Vec::new(),
        }
    }

    /// Update the list of connected devices.
    pub fn update_devices(&mut self, devices: Vec<ConnectionInfo>) {
        self.devices = devices;
    }

    /// Get count of connected devices.
    pub fn device_count(&self) -> usize {
        self.devices.len()
    }

    /// Render the connected devices panel.
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self
            .devices
            .iter()
            .map(|conn| {
                let uptime_secs = conn.uptime.num_seconds();
                let hours = uptime_secs / 3600;
                let minutes = (uptime_secs % 3600) / 60;
                let seconds = uptime_secs % 60;

                let uptime_str = format!("{:02}:{:02}:{:02}", hours, minutes, seconds);

                let line = Line::from(vec![
                    Span::styled("● ", device_connected_style()),
                    Span::raw(format!(
                        "Device {:02} | {} | Connected {}",
                        conn.device_id.as_u8(),
                        conn.remote_addr,
                        uptime_str
                    )),
                ]);

                ListItem::new(line)
            })
            .collect();

        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(panel_border_style())
                .title("CONNECTED DEVICES"),
        );

        frame.render_widget(list, area);
    }
}

impl Default for ConnectedDevicesPanel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    use turnkey_core::DeviceId;

    fn create_test_connection(device_id: u8, uptime_secs: i64) -> ConnectionInfo {
        ConnectionInfo {
            device_id: DeviceId::new(device_id).unwrap(),
            remote_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 54321),
            connected_at: Utc::now() - Duration::try_seconds(uptime_secs).unwrap(),
            uptime: Duration::try_seconds(uptime_secs).unwrap(),
        }
    }

    #[test]
    fn test_panel_creation() {
        let panel = ConnectedDevicesPanel::new();
        assert_eq!(panel.device_count(), 0);
    }

    #[test]
    fn test_update_devices() {
        let mut panel = ConnectedDevicesPanel::new();

        let devices = vec![
            create_test_connection(1, 100),
            create_test_connection(2, 200),
        ];

        panel.update_devices(devices);
        assert_eq!(panel.device_count(), 2);
    }

    #[test]
    fn test_device_count() {
        let mut panel = ConnectedDevicesPanel::new();
        assert_eq!(panel.device_count(), 0);

        panel.update_devices(vec![create_test_connection(1, 50)]);
        assert_eq!(panel.device_count(), 1);
    }
}
