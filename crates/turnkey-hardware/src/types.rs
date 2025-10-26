//! Common types shared across hardware device implementations.
//!
//! This module defines types used by multiple device traits, such as
//! device information, LED colors, and protocol-specific data structures.

use serde::{Deserialize, Serialize};

/// Generic device information.
///
/// Contains metadata about a hardware device such as name, model,
/// serial number, and firmware version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceInfo {
    /// Device name (e.g., "ACR122U", "MockKeypad").
    pub name: String,

    /// Device model identifier.
    pub model: String,

    /// Optional device serial number.
    pub serial_number: Option<String>,

    /// Optional firmware version string.
    pub firmware_version: Option<String>,
}

impl DeviceInfo {
    /// Create a new DeviceInfo with required fields.
    pub fn new(name: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            model: model.into(),
            serial_number: None,
            firmware_version: None,
        }
    }

    /// Set the serial number.
    pub fn with_serial_number(mut self, serial_number: impl Into<String>) -> Self {
        self.serial_number = Some(serial_number.into());
        self
    }

    /// Set the firmware version.
    pub fn with_firmware_version(mut self, firmware_version: impl Into<String>) -> Self {
        self.firmware_version = Some(firmware_version.into());
        self
    }
}

/// RFID reader information.
///
/// Contains reader-specific metadata such as supported protocols
/// and maximum baud rate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReaderInfo {
    /// Reader name (e.g., "ACR122U NFC Reader").
    pub name: String,

    /// List of supported protocols (e.g., ["ISO14443A", "ISO14443B"]).
    pub protocols: Vec<String>,

    /// Maximum supported baud rate in bits per second.
    pub max_baud_rate: Option<u32>,
}

impl ReaderInfo {
    /// Create a new ReaderInfo.
    pub fn new(name: impl Into<String>, protocols: Vec<String>) -> Self {
        Self {
            name: name.into(),
            protocols,
            max_baud_rate: None,
        }
    }

    /// Set the maximum baud rate.
    pub fn with_max_baud_rate(mut self, max_baud_rate: u32) -> Self {
        self.max_baud_rate = Some(max_baud_rate);
        self
    }
}

/// LED colors for visual feedback on devices.
///
/// Many hardware devices have status LEDs that can be controlled
/// to provide visual feedback to users.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum LedColor {
    /// LED off.
    Off,

    /// Red LED.
    Red,

    /// Green LED.
    Green,

    /// Blue LED.
    Blue,

    /// Yellow LED.
    Yellow,

    /// Orange LED.
    Orange,

    /// Cyan LED.
    Cyan,

    /// Magenta LED.
    Magenta,

    /// White LED.
    White,

    /// Custom RGB color (red, green, blue).
    Custom(u8, u8, u8),
}

impl LedColor {
    /// Create a custom RGB LED color.
    pub fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self::Custom(r, g, b)
    }

    /// Get the RGB components of the LED color.
    pub fn as_rgb(&self) -> (u8, u8, u8) {
        match self {
            Self::Off => (0, 0, 0),
            Self::Red => (255, 0, 0),
            Self::Green => (0, 255, 0),
            Self::Blue => (0, 0, 255),
            Self::Yellow => (255, 255, 0),
            Self::Orange => (255, 165, 0),
            Self::Cyan => (0, 255, 255),
            Self::Magenta => (255, 0, 255),
            Self::White => (255, 255, 255),
            Self::Custom(r, g, b) => (*r, *g, *b),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_info_builder() {
        let info = DeviceInfo::new("ACR122U", "USB NFC Reader")
            .with_serial_number("123456789")
            .with_firmware_version("v2.0.1");

        assert_eq!(info.name, "ACR122U");
        assert_eq!(info.model, "USB NFC Reader");
        assert_eq!(info.serial_number, Some("123456789".to_string()));
        assert_eq!(info.firmware_version, Some("v2.0.1".to_string()));
    }

    #[test]
    fn test_device_info_minimal() {
        let info = DeviceInfo::new("MockKeypad", "Mock");

        assert_eq!(info.name, "MockKeypad");
        assert_eq!(info.model, "Mock");
        assert_eq!(info.serial_number, None);
        assert_eq!(info.firmware_version, None);
    }

    #[test]
    fn test_reader_info() {
        let info =
            ReaderInfo::new("ACR122U", vec!["ISO14443A".to_string()]).with_max_baud_rate(424000);

        assert_eq!(info.name, "ACR122U");
        assert_eq!(info.protocols, vec!["ISO14443A"]);
        assert_eq!(info.max_baud_rate, Some(424000));
    }

    #[test]
    fn test_led_color_rgb() {
        assert_eq!(LedColor::Red.as_rgb(), (255, 0, 0));
        assert_eq!(LedColor::Green.as_rgb(), (0, 255, 0));
        assert_eq!(LedColor::Blue.as_rgb(), (0, 0, 255));
        assert_eq!(LedColor::Off.as_rgb(), (0, 0, 0));
    }

    #[test]
    fn test_led_color_custom() {
        let custom = LedColor::rgb(128, 64, 32);
        assert_eq!(custom.as_rgb(), (128, 64, 32));
    }

    #[test]
    fn test_led_color_serialization() {
        let color = LedColor::Green;
        let json = serde_json::to_string(&color).unwrap();
        let deserialized: LedColor = serde_json::from_str(&json).unwrap();
        assert_eq!(color, deserialized);
    }
}
