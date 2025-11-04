//! Emulator configuration structures and loading.
//!
//! This module provides configuration structures for the turnstile emulator,
//! supporting both TOML file deserialization and programmatic construction.
//!
//! # Configuration File Format
//!
//! The emulator expects a TOML configuration file with the following structure:
//!
//! ```toml
//! [device]
//! id = 1
//! default_display_message = "DIGITE SEU CODIGO"
//!
//! [mode]
//! online = true
//!
//! [timeouts]
//! validation_ms = 3000
//! rotation_secs = 10
//! ```
//!
//! # Examples
//!
//! ## Load from file
//!
//! ```no_run
//! use turnkey_emulator::EmulatorConfig;
//!
//! let config = EmulatorConfig::from_file("config/default.toml")?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ## Create programmatically
//!
//! ```
//! use turnkey_emulator::{EmulatorConfig, OperationMode};
//! use turnkey_core::AccessDirection;
//!
//! let config = EmulatorConfig {
//!     device_id: 1,
//!     mode: OperationMode::Online,
//!     validation_timeout_ms: 3000,
//!     rotation_timeout_secs: 10,
//!     default_display_message: "DIGITE SEU CODIGO".to_string(),
//!     default_direction: AccessDirection::Entry,
//!     idle_timeout_secs: 30,
//!     rotation_duration_secs: 2,
//!     network: None,
//! };
//! ```

use serde::{Deserialize, Serialize};
use std::fs;
use std::net::IpAddr;
use std::path::Path;
use turnkey_core::{AccessDirection, Error, Result};

/// Minimum valid device ID per Henry protocol.
const MIN_DEVICE_ID: u8 = 1;

/// Maximum valid device ID per Henry protocol.
const MAX_DEVICE_ID: u8 = 99;

/// Maximum display message length (LCD specification: 2 lines × 40 columns).
const MAX_DISPLAY_MESSAGE_LENGTH: usize = 40;

/// Network configuration for ONLINE mode.
///
/// Contains network settings required for TCP communication with
/// the validation server in ONLINE mode.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// IP address for TCP connection in ONLINE mode.
    ///
    /// The server address to connect to for access validation.
    /// Can be an IPv4 or IPv6 address.
    pub ip_address: IpAddr,

    /// Port for TCP connection.
    ///
    /// The server port to connect to for access validation.
    /// Standard Henry protocol port is 3000.
    pub port: u16,
}

impl NetworkConfig {
    /// Create a new network configuration.
    ///
    /// # Arguments
    ///
    /// * `ip_address` - Server IP address
    /// * `port` - Server port number
    ///
    /// # Examples
    ///
    /// ```
    /// use turnkey_emulator::config::NetworkConfig;
    /// use std::net::IpAddr;
    ///
    /// let config = NetworkConfig::new(
    ///     "192.168.1.100".parse::<IpAddr>().unwrap(),
    ///     3000
    /// );
    /// ```
    pub fn new(ip_address: IpAddr, port: u16) -> Self {
        Self { ip_address, port }
    }

    /// Get the IP address as a string.
    pub fn ip_address_string(&self) -> String {
        self.ip_address.to_string()
    }
}

/// Operation mode for the emulator.
///
/// Determines whether the emulator validates access requests using
/// a remote server (Online) or local database (Offline).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperationMode {
    /// Online mode - validates via TCP connection to server.
    ///
    /// Access requests are sent to a validation server and the emulator
    /// waits for responses. Requires network connectivity and a running
    /// validation server.
    Online,

    /// Offline mode - validates using local SQLite database.
    ///
    /// Access requests are validated against a local database of users
    /// and cards. No network connection required.
    Offline,
}

impl Default for OperationMode {
    fn default() -> Self {
        Self::Online
    }
}

/// Configuration for the turnstile emulator.
///
/// This struct contains all configuration parameters needed to run
/// the emulator, including device identity, operation mode, and
/// timeout settings.
///
/// # Examples
///
/// ```
/// use turnkey_emulator::{EmulatorConfig, OperationMode};
/// use turnkey_core::AccessDirection;
///
/// let config = EmulatorConfig {
///     device_id: 15,
///     mode: OperationMode::Online,
///     validation_timeout_ms: 3000,
///     rotation_timeout_secs: 10,
///     default_display_message: "DIGITE SEU CODIGO".to_string(),
///     default_direction: AccessDirection::Entry,
///     idle_timeout_secs: 30,
///     rotation_duration_secs: 2,
///     network: None,
/// };
///
/// assert_eq!(config.device_id, 15);
/// assert_eq!(config.mode, OperationMode::Online);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmulatorConfig {
    /// Device ID (1-99).
    ///
    /// This ID is used in protocol messages to identify this emulator
    /// instance. Must be unique within the access control system.
    pub device_id: u8,

    /// Operation mode (Online or Offline).
    ///
    /// Determines how access requests are validated. In Online mode,
    /// requests are sent to a validation server. In Offline mode,
    /// requests are validated against a local database.
    pub mode: OperationMode,

    /// Validation timeout in milliseconds.
    ///
    /// Maximum time to wait for a validation response when in Online mode.
    /// After this timeout, the request is considered failed.
    ///
    /// Default: 3000ms (3 seconds)
    pub validation_timeout_ms: u64,

    /// Rotation timeout in seconds.
    ///
    /// Maximum time to wait for the user to pass through the turnstile
    /// after access is granted. If the user does not pass within this
    /// time, the grant expires and the turnstile returns to idle.
    ///
    /// Default: 10 seconds
    pub rotation_timeout_secs: u64,

    /// Default message to display when idle.
    ///
    /// This message is shown on the LCD display when the turnstile
    /// is waiting for user input. Must be ASCII-only and no longer
    /// than 40 characters.
    pub default_display_message: String,

    /// Default access direction for requests.
    ///
    /// Determines the direction field in access request messages:
    /// - Entry (1): User entering the facility
    /// - Exit (2): User exiting the facility
    /// - Undefined (0): Direction not specified
    ///
    /// Default: Entry
    pub default_direction: AccessDirection,

    /// Idle timeout in seconds.
    ///
    /// If no user activity (keypad input, card read) occurs within this
    /// period, the emulator will return to idle state, clearing any
    /// partial input.
    ///
    /// Set to 0 to disable idle timeout.
    ///
    /// Default: 30 seconds
    pub idle_timeout_secs: u64,

    /// Physical rotation duration in seconds.
    ///
    /// Time it takes for the turnstile to complete a physical rotation.
    /// Used in emulation mode to simulate realistic hardware behavior.
    /// Adjust this value to match your specific turnstile hardware
    /// characteristics for accurate testing.
    ///
    /// Default: 2 seconds
    pub rotation_duration_secs: u64,

    /// Network configuration for ONLINE mode.
    ///
    /// Contains IP address and port for TCP connection to validation server.
    /// Required when `mode` is `OperationMode::Online`.
    /// Optional (can be `None`) when in `OperationMode::Offline`.
    pub network: Option<NetworkConfig>,
}

impl Default for EmulatorConfig {
    fn default() -> Self {
        Self {
            device_id: 1,
            mode: OperationMode::Online,
            validation_timeout_ms: 3000,
            rotation_timeout_secs: 10,
            default_display_message: "DIGITE SEU CODIGO".to_string(),
            default_direction: AccessDirection::Entry,
            idle_timeout_secs: 30,
            rotation_duration_secs: 2,
            network: None,
        }
    }
}

impl EmulatorConfig {
    /// Load configuration from a TOML file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the TOML configuration file
    ///
    /// # Returns
    ///
    /// Returns `Ok(EmulatorConfig)` if the file was successfully read
    /// and parsed.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The file does not exist or cannot be read
    /// - The file contains invalid TOML syntax
    /// - Required configuration fields are missing
    /// - Configuration values are out of valid range
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use turnkey_emulator::EmulatorConfig;
    ///
    /// let config = EmulatorConfig::from_file("config/default.toml")?;
    /// println!("Loaded config for device {}", config.device_id);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let contents = fs::read_to_string(path.as_ref())
            .map_err(|e| Error::Config(format!("Failed to read config file: {}", e)))?;

        let config: Self = toml::from_str(&contents)
            .map_err(|e| Error::Config(format!("Failed to parse config file: {}", e)))?;

        config.validate()?;

        Ok(config)
    }

    /// Validate configuration values.
    ///
    /// Checks that all configuration values are within valid ranges
    /// and constraints.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if all values are valid.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - `device_id` is 0 or greater than 99
    /// - `default_display_message` is empty or longer than 40 characters
    /// - `default_display_message` contains non-ASCII characters
    fn validate(&self) -> Result<()> {
        self.validate_device_id()?;
        self.validate_display_message()?;
        Ok(())
    }

    /// Validate device ID is within valid range (1-99).
    fn validate_device_id(&self) -> Result<()> {
        if !(MIN_DEVICE_ID..=MAX_DEVICE_ID).contains(&self.device_id) {
            return Err(Error::Config(format!(
                "Device ID must be between {} and {}, got {}",
                MIN_DEVICE_ID, MAX_DEVICE_ID, self.device_id
            )));
        }
        Ok(())
    }

    /// Validate display message meets requirements.
    fn validate_display_message(&self) -> Result<()> {
        if self.default_display_message.is_empty() {
            return Err(Error::Config(
                "Default display message cannot be empty".to_string(),
            ));
        }

        if self.default_display_message.len() > MAX_DISPLAY_MESSAGE_LENGTH {
            return Err(Error::Config(format!(
                "Default display message cannot exceed {} characters, got {}",
                MAX_DISPLAY_MESSAGE_LENGTH,
                self.default_display_message.len()
            )));
        }

        if !self.default_display_message.is_ascii() {
            return Err(Error::Config(
                "Default display message must contain only ASCII characters".to_string(),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operation_mode_default() {
        assert_eq!(OperationMode::default(), OperationMode::Online);
    }

    #[test]
    fn test_emulator_config_default() {
        let config = EmulatorConfig::default();

        assert_eq!(config.device_id, 1);
        assert_eq!(config.mode, OperationMode::Online);
        assert_eq!(config.validation_timeout_ms, 3000);
        assert_eq!(config.rotation_timeout_secs, 10);
        assert_eq!(config.default_display_message, "DIGITE SEU CODIGO");
    }

    #[test]
    fn test_validate_valid_config() {
        let config = EmulatorConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_device_id_zero() {
        let config = EmulatorConfig {
            device_id: 0,
            ..EmulatorConfig::default()
        };

        let result = config.validate();
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Device ID must be between 1 and 99")
        );
    }

    #[test]
    fn test_validate_device_id_too_high() {
        let config = EmulatorConfig {
            device_id: 100,
            ..EmulatorConfig::default()
        };

        let result = config.validate();
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Device ID must be between 1 and 99")
        );
    }

    #[test]
    fn test_validate_empty_display_message() {
        let config = EmulatorConfig {
            default_display_message: String::new(),
            ..EmulatorConfig::default()
        };

        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be empty"));
    }

    #[test]
    fn test_validate_display_message_too_long() {
        let config = EmulatorConfig {
            default_display_message: "A".repeat(41),
            ..EmulatorConfig::default()
        };

        let result = config.validate();
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("cannot exceed 40 characters")
        );
    }

    #[test]
    fn test_validate_display_message_non_ascii() {
        let config = EmulatorConfig {
            default_display_message: "CÓDIGO".to_string(),
            ..EmulatorConfig::default()
        };

        let result = config.validate();
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("must contain only ASCII characters")
        );
    }

    #[test]
    fn test_serialize_deserialize() {
        use std::net::IpAddr;
        use turnkey_core::AccessDirection;

        let config = EmulatorConfig {
            device_id: 15,
            mode: OperationMode::Offline,
            validation_timeout_ms: 5000,
            rotation_timeout_secs: 15,
            default_display_message: "WELCOME".to_string(),
            default_direction: AccessDirection::Exit,
            idle_timeout_secs: 60,
            rotation_duration_secs: 3,
            network: Some(NetworkConfig::new(
                "192.168.1.100".parse::<IpAddr>().unwrap(),
                3000,
            )),
        };

        let toml_str = toml::to_string(&config).unwrap();
        let deserialized: EmulatorConfig = toml::from_str(&toml_str).unwrap();

        assert_eq!(config, deserialized);
    }

    #[test]
    fn test_network_config_creation() {
        use std::net::IpAddr;

        let config = NetworkConfig::new("192.168.1.100".parse::<IpAddr>().unwrap(), 3000);

        assert_eq!(config.ip_address_string(), "192.168.1.100");
        assert_eq!(config.port, 3000);
    }

    #[test]
    fn test_network_config_ipv6() {
        use std::net::IpAddr;

        let config = NetworkConfig::new("::1".parse::<IpAddr>().unwrap(), 8080);

        assert_eq!(config.ip_address_string(), "::1");
        assert_eq!(config.port, 8080);
    }
}
