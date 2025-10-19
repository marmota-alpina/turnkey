//! Command code definitions for the Henry protocol.
//!
//! This module defines all command codes used in the Henry protocol for
//! access control communication between turnstiles and servers. Each command
//! code represents a specific action or message type in the protocol flow.
//!
//! # Protocol Format
//!
//! Command codes appear in the protocol format after the device ID and
//! protocol identifier:
//!
//! ```text
//! <ID>+REON+<COMMAND>+<DATA_FIELDS>
//!            ^^^^^^^^
//!            Command code position
//! ```
//!
//! # Command Categories
//!
//! Commands are organized into three main categories:
//!
//! ## Access Control
//!
//! Commands for managing access requests and responses:
//! - `AccessRequest` (000+0): Turnstile requests access validation
//! - `GrantBoth` (00+1): Server grants access in both directions
//! - `GrantManual` (00+4): Server grants manual access (requires operator intervention)
//! - `GrantEntry` (00+5): Server grants entry access only
//! - `GrantExit` (00+6): Server grants exit access only
//! - `DenyAccess` (00+30): Server denies access
//!
//! ## Turnstile Status
//!
//! Commands for tracking turnstile state machine:
//! - `WaitingRotation` (000+80): Turnstile waiting for user to pass
//! - `RotationCompleted` (000+81): User successfully passed through
//! - `RotationTimeout` (000+82): User did not pass within timeout period
//!
//! ## Device Management
//!
//! Commands for configuration and data synchronization:
//! - `SendConfig` (EC): Send device configuration parameters
//! - `SendCards` (ECAR): Send/update card database
//! - `SendUsers` (EU): Send/update user database (Primme Acesso only)
//! - `SendBiometrics` (ED): Send/update biometric templates
//! - `SendDateTime` (EH): Synchronize device date/time
//! - `ReceiveLogs` (ER): Retrieve access logs from device
//! - `QueryStatus` (RQ): Query device status and counters
//! - `ReceiveConfig` (RC): Request current device configuration
//!
//! # Wire Format Examples
//!
//! ## Access Request
//! ```text
//! 15+REON+000+0]12345678]10/05/2025 12:46:06]1]0]
//!         ^^^^^ Command: AccessRequest
//! ```
//!
//! ## Grant Access
//! ```text
//! 15+REON+00+6]5]Acesso liberado]
//!         ^^^^ Command: GrantExit
//! ```
//!
//! ## Rotation Status
//! ```text
//! 15+REON+000+81]]10/05/2025 12:46:08]1]0]
//!         ^^^^^^ Command: RotationCompleted
//! ```
//!
//! # Usage Examples
//!
//! ## Parsing Command Codes
//!
//! ```
//! use turnkey_protocol::CommandCode;
//!
//! // Parse from protocol string
//! let cmd = CommandCode::parse("000+0").unwrap();
//! assert_eq!(cmd, CommandCode::AccessRequest);
//!
//! // Convert back to string
//! assert_eq!(cmd.as_str(), "000+0");
//! ```
//!
//! ## Round-trip Conversion
//!
//! ```
//! use turnkey_protocol::CommandCode;
//!
//! let original = CommandCode::GrantExit;
//! let wire_format = original.as_str();
//! let parsed = CommandCode::parse(wire_format).unwrap();
//!
//! assert_eq!(parsed, original);
//! assert_eq!(wire_format, "00+6");
//! ```
//!
//! ## Error Handling
//!
//! ```
//! use turnkey_protocol::CommandCode;
//!
//! // Invalid command codes return errors
//! let result = CommandCode::parse("INVALID");
//! assert!(result.is_err());
//! ```
//!
//! # Protocol Compliance
//!
//! All command codes are defined according to the Henry protocol specification
//! and are compatible with:
//! - Primme Acesso (all commands)
//! - Argos (access control and status commands)
//! - Primme SF (access control and basic management)
//!
//! Note: Some commands like `SendUsers` (EU) are specific to certain equipment
//! models and may not be supported by all devices.

use serde::{Deserialize, Serialize};
use std::fmt;
use turnkey_core::{Error, Result};

/// Command codes for Henry protocol messages.
///
/// Represents all supported command types in the Henry access control protocol.
/// Each variant corresponds to a specific wire format code used in protocol messages.
///
/// # Wire Format
///
/// Command codes use a compact string representation:
/// - Access control: Numeric codes (e.g., "000+0", "00+6")
/// - Management: Letter codes (e.g., "EC", "ECAR", "RQ")
///
/// # Examples
///
/// ```
/// use turnkey_protocol::CommandCode;
///
/// let cmd = CommandCode::AccessRequest;
/// assert_eq!(cmd.as_str(), "000+0");
///
/// let parsed = CommandCode::parse("00+6").unwrap();
/// assert_eq!(parsed, CommandCode::GrantExit);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommandCode {
    // Access control
    AccessRequest, // 000+0
    GrantBoth,     // 00+1
    GrantManual,   // 00+4
    GrantEntry,    // 00+5
    GrantExit,     // 00+6
    DenyAccess,    // 00+30

    // Turnstile status
    WaitingRotation,   // 000+80
    RotationCompleted, // 000+81
    RotationTimeout,   // 000+82

    // Management
    SendConfig,     // EC
    SendCards,      // ECAR
    SendUsers,      // EU
    SendBiometrics, // ED
    SendDateTime,   // EH
    ReceiveLogs,    // ER
    QueryStatus,    // RQ
    ReceiveConfig,  // RC
}

impl CommandCode {
    pub fn parse(s: &str) -> Result<Self> {
        match s {
            "000+0" => Ok(CommandCode::AccessRequest),
            "00+1" => Ok(CommandCode::GrantBoth),
            "00+4" => Ok(CommandCode::GrantManual),
            "00+5" => Ok(CommandCode::GrantEntry),
            "00+6" => Ok(CommandCode::GrantExit),
            "00+30" => Ok(CommandCode::DenyAccess),
            "000+80" => Ok(CommandCode::WaitingRotation),
            "000+81" => Ok(CommandCode::RotationCompleted),
            "000+82" => Ok(CommandCode::RotationTimeout),
            "EC" => Ok(CommandCode::SendConfig),
            "ECAR" => Ok(CommandCode::SendCards),
            "EU" => Ok(CommandCode::SendUsers),
            "ED" => Ok(CommandCode::SendBiometrics),
            "EH" => Ok(CommandCode::SendDateTime),
            "ER" => Ok(CommandCode::ReceiveLogs),
            "RQ" => Ok(CommandCode::QueryStatus),
            "RC" => Ok(CommandCode::ReceiveConfig),
            _ => Err(Error::InvalidCommandCode {
                code: s.to_string(),
            }),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            CommandCode::AccessRequest => "000+0",
            CommandCode::GrantBoth => "00+1",
            CommandCode::GrantManual => "00+4",
            CommandCode::GrantEntry => "00+5",
            CommandCode::GrantExit => "00+6",
            CommandCode::DenyAccess => "00+30",
            CommandCode::WaitingRotation => "000+80",
            CommandCode::RotationCompleted => "000+81",
            CommandCode::RotationTimeout => "000+82",
            CommandCode::SendConfig => "EC",
            CommandCode::SendCards => "ECAR",
            CommandCode::SendUsers => "EU",
            CommandCode::SendBiometrics => "ED",
            CommandCode::SendDateTime => "EH",
            CommandCode::ReceiveLogs => "ER",
            CommandCode::QueryStatus => "RQ",
            CommandCode::ReceiveConfig => "RC",
        }
    }
}

impl fmt::Display for CommandCode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_code_parse() {
        assert_eq!(
            CommandCode::parse("000+0").unwrap(),
            CommandCode::AccessRequest
        );
        assert_eq!(CommandCode::parse("00+6").unwrap(), CommandCode::GrantExit);
        assert_eq!(
            CommandCode::parse("000+81").unwrap(),
            CommandCode::RotationCompleted
        );
        assert_eq!(CommandCode::parse("ECAR").unwrap(), CommandCode::SendCards);
    }

    #[test]
    fn test_command_code_invalid() {
        assert!(CommandCode::parse("INVALID").is_err());
    }

    #[test]
    fn test_command_code_round_trip() {
        let commands = vec![
            CommandCode::AccessRequest,
            CommandCode::GrantExit,
            CommandCode::WaitingRotation,
            CommandCode::SendCards,
        ];

        for cmd in commands {
            let str_repr = cmd.as_str();
            let parsed = CommandCode::parse(str_repr).unwrap();
            assert_eq!(parsed, cmd);
        }
    }

    #[test]
    fn test_command_code_display() {
        // Access control commands
        assert_eq!(format!("{}", CommandCode::AccessRequest), "000+0");
        assert_eq!(format!("{}", CommandCode::GrantBoth), "00+1");
        assert_eq!(format!("{}", CommandCode::GrantManual), "00+4");
        assert_eq!(format!("{}", CommandCode::GrantEntry), "00+5");
        assert_eq!(format!("{}", CommandCode::GrantExit), "00+6");
        assert_eq!(format!("{}", CommandCode::DenyAccess), "00+30");

        // Turnstile status commands
        assert_eq!(format!("{}", CommandCode::WaitingRotation), "000+80");
        assert_eq!(format!("{}", CommandCode::RotationCompleted), "000+81");
        assert_eq!(format!("{}", CommandCode::RotationTimeout), "000+82");

        // Management commands
        assert_eq!(format!("{}", CommandCode::SendConfig), "EC");
        assert_eq!(format!("{}", CommandCode::SendCards), "ECAR");
        assert_eq!(format!("{}", CommandCode::SendUsers), "EU");
        assert_eq!(format!("{}", CommandCode::SendBiometrics), "ED");
        assert_eq!(format!("{}", CommandCode::SendDateTime), "EH");
        assert_eq!(format!("{}", CommandCode::ReceiveLogs), "ER");
        assert_eq!(format!("{}", CommandCode::QueryStatus), "RQ");
        assert_eq!(format!("{}", CommandCode::ReceiveConfig), "RC");
    }

    #[test]
    fn test_command_code_display_consistency() {
        // Verify that Display trait produces same output as as_str()
        let all_commands = vec![
            CommandCode::AccessRequest,
            CommandCode::GrantBoth,
            CommandCode::GrantManual,
            CommandCode::GrantEntry,
            CommandCode::GrantExit,
            CommandCode::DenyAccess,
            CommandCode::WaitingRotation,
            CommandCode::RotationCompleted,
            CommandCode::RotationTimeout,
            CommandCode::SendConfig,
            CommandCode::SendCards,
            CommandCode::SendUsers,
            CommandCode::SendBiometrics,
            CommandCode::SendDateTime,
            CommandCode::ReceiveLogs,
            CommandCode::QueryStatus,
            CommandCode::ReceiveConfig,
        ];

        for cmd in all_commands {
            assert_eq!(format!("{}", cmd), cmd.as_str());
        }
    }

    #[test]
    fn test_command_code_display_in_strings() {
        // Test that Display can be used in string formatting
        let cmd = CommandCode::AccessRequest;
        let message = format!("Processing command: {}", cmd);
        assert_eq!(message, "Processing command: 000+0");

        let cmd = CommandCode::QueryStatus;
        let log_entry = format!("[{}] Device query initiated", cmd);
        assert_eq!(log_entry, "[RQ] Device query initiated");
    }
}
