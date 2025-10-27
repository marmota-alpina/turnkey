//! Henry protocol message parser.
//!
//! This module provides parsing functionality for Henry protocol messages,
//! converting raw ASCII text into structured [`Message`] objects.
//!
//! # Protocol Format
//!
//! The Henry protocol uses the following message structure:
//!
//! ```text
//! ID+REON+COMMAND+FIELD1]FIELD2]FIELD3]...
//! ```
//!
//! Where:
//! - `ID`: Device identifier (01-99)
//! - `REON`: Protocol identifier constant
//! - `COMMAND`: Command code (e.g., "000+0", "00+6", "000+80")
//! - `FIELD1]FIELD2]...`: Variable number of data fields separated by `]`
//!
//! # Delimiter Semantics
//!
//! The parser uses the following delimiters:
//! - `+` - Separates device ID, protocol ID, and command components
//! - `]` - Separates data fields (IMPORTANT: empty fields are preserved)
//! - `[` - Subfield delimiter (used in nested data structures)
//! - `{` and `}` - Array delimiters
//!
//! # Empty Field Handling
//!
//! **CRITICAL**: Empty fields have semantic meaning in the Henry protocol.
//! The parser preserves empty fields to maintain protocol compliance.
//!
//! Example: `15+REON+000+80]]10/05/2025 12:46:06]0]0]`
//!
//! The double `]]` creates an empty first field (empty card number), which
//! indicates "waiting for rotation" without a specific card. This is NOT
//! an error - it's the correct protocol format.
//!
//! # Examples
//!
//! ## Parse Access Request
//!
//! ```
//! use turnkey_protocol::parser::MessageParser;
//! use turnkey_protocol::commands::CommandCode;
//!
//! let input = "15+REON+000+0]12345678]10/05/2025 12:46:06]1]0]";
//! let msg = MessageParser::parse(input).unwrap();
//!
//! assert_eq!(msg.device_id.as_u8(), 15);
//! assert_eq!(msg.command, CommandCode::AccessRequest);
//! assert_eq!(msg.field_count(), 4);
//! assert_eq!(msg.field(0), Some("12345678")); // Card number
//! ```
//!
//! ## Parse Message with Empty Field
//!
//! ```
//! use turnkey_protocol::parser::MessageParser;
//! use turnkey_protocol::commands::CommandCode;
//!
//! // Double ]] creates empty field (valid protocol)
//! let input = "15+REON+000+80]]10/05/2025 12:46:06]0]0]";
//! let msg = MessageParser::parse(input).unwrap();
//!
//! assert_eq!(msg.command, CommandCode::WaitingRotation);
//! assert_eq!(msg.field(0), Some("")); // Empty card number field
//! assert_eq!(msg.field(1), Some("10/05/2025 12:46:06"));
//! ```
//!
//! ## Error Handling
//!
//! ```
//! use turnkey_protocol::parser::MessageParser;
//!
//! // Invalid device ID
//! let result = MessageParser::parse("999+REON+000+0]12345678]");
//! assert!(result.is_err());
//!
//! // Missing REON
//! let result = MessageParser::parse("01+XXXX+000+0]12345678]");
//! assert!(result.is_err());
//! ```
//!
//! # Protocol Reference
//!
//! See Henry protocol specification section 2.1 for message format details.

use crate::{commands::CommandCode, field::FieldData, message::Message};
use turnkey_core::{DeviceId, Error, Result, constants::*};

/// Parser for Henry protocol messages.
///
/// This struct provides a single static method for parsing raw protocol
/// messages into structured [`Message`] objects.
pub struct MessageParser;

impl MessageParser {
    /// Parse a Henry protocol message from raw ASCII text.
    ///
    /// Converts a raw protocol message string into a structured [`Message`]
    /// object, validating format and extracting components.
    ///
    /// # Message Structure
    ///
    /// ```text
    /// ID+REON+COMMAND+FIELD1]FIELD2]...
    /// ```
    ///
    /// # Arguments
    ///
    /// * `input` - Raw protocol message (leading/trailing whitespace is trimmed)
    ///
    /// # Returns
    ///
    /// Returns `Ok(Message)` if parsing succeeds, containing:
    /// - Device ID (validated 01-99 range)
    /// - Command code (validated against known commands)
    /// - Data fields (empty fields preserved)
    ///
    /// # Errors
    ///
    /// Returns `Err` if:
    /// - Device ID is missing or out of range (01-99)
    /// - Protocol identifier is not "REON"
    /// - Command code is invalid or unrecognized
    /// - Message format is malformed (incorrect delimiters)
    ///
    /// # Empty Field Handling
    ///
    /// **IMPORTANT**: Empty fields (consecutive `]]`) are preserved as they
    /// have semantic meaning in the protocol. For example, waiting rotation
    /// messages use an empty card number field:
    ///
    /// ```text
    /// 15+REON+000+80]]10/05/2025 12:46:06]0]0]
    ///                ^^ Empty field (valid)
    /// ```
    ///
    /// # Examples
    ///
    /// ## Basic Parsing
    ///
    /// ```
    /// use turnkey_protocol::parser::MessageParser;
    /// use turnkey_protocol::commands::CommandCode;
    ///
    /// let input = "15+REON+000+0]12345678]10/05/2025 12:46:06]1]0]";
    /// let msg = MessageParser::parse(input).unwrap();
    ///
    /// assert_eq!(msg.device_id.as_u8(), 15);
    /// assert_eq!(msg.command, CommandCode::AccessRequest);
    /// assert_eq!(msg.field_count(), 4);
    /// ```
    ///
    /// ## Whitespace Handling
    ///
    /// ```
    /// use turnkey_protocol::parser::MessageParser;
    ///
    /// // Leading/trailing whitespace is automatically trimmed
    /// let input = "  15+REON+RQ  \n";
    /// let msg = MessageParser::parse(input).unwrap();
    /// assert_eq!(msg.device_id.as_u8(), 15);
    /// ```
    ///
    /// ## Error Cases
    ///
    /// ```
    /// use turnkey_protocol::parser::MessageParser;
    ///
    /// // Invalid device ID (out of range)
    /// assert!(MessageParser::parse("999+REON+RQ").is_err());
    ///
    /// // Missing REON identifier
    /// assert!(MessageParser::parse("01+XXXX+RQ").is_err());
    ///
    /// // Invalid command code
    /// assert!(MessageParser::parse("01+REON+INVALID").is_err());
    /// ```
    ///
    /// # Protocol Compliance
    ///
    /// This parser strictly validates:
    /// - Device ID range (01-99)
    /// - Protocol identifier ("REON")
    /// - Delimiter placement (`+` and `]`)
    /// - Command code validity
    ///
    /// # Performance
    ///
    /// The parser performs minimal allocations:
    /// - One allocation for field vector
    /// - Individual field data allocations
    /// - No intermediate string copies for validation
    pub fn parse(input: &str) -> Result<Message> {
        let input = input.trim();

        // Split by first '+'
        let mut parts = input.splitn(2, DELIMITER_DEVICE);

        // Parse device ID
        let device_id_str = parts.next().ok_or_else(|| Error::InvalidMessageFormat {
            message: "Missing device ID".to_string(),
        })?;
        let device_id: DeviceId = device_id_str.parse()?;

        // Get rest of message
        let rest = parts.next().ok_or_else(|| Error::InvalidMessageFormat {
            message: "Missing protocol ID".to_string(),
        })?;

        // Check for REON
        if !rest.starts_with(PROTOCOL_ID) {
            return Err(Error::InvalidMessageFormat {
                message: format!("Expected '{}' protocol identifier", PROTOCOL_ID),
            });
        }

        // Remove REON+
        let rest = &rest[PROTOCOL_ID.len()..];
        if !rest.starts_with(DELIMITER_DEVICE) {
            return Err(Error::InvalidMessageFormat {
                message: "Missing delimiter after REON".to_string(),
            });
        }
        let rest = &rest[1..]; // Skip '+'

        // Split by ] to separate command from fields
        // The command can contain + (like "000+0" or "00+6")
        let split_pos = rest.find(DELIMITER_FIELD);

        let (command_str, rest_fields) = if let Some(pos) = split_pos {
            (&rest[..pos], &rest[pos + 1..])
        } else {
            (rest, "")
        };

        let command = CommandCode::parse(command_str)?;

        // Parse fields (separated by ])
        // IMPORTANT: Do not filter empty fields - they are semantically meaningful
        // in the protocol (e.g., empty card number in waiting rotation messages)
        let fields: Result<Vec<FieldData>> = if !rest_fields.is_empty() {
            let split_fields: Vec<&str> = rest_fields.split(DELIMITER_FIELD).collect();
            // Remove only the trailing empty string from the final delimiter
            // (if the message ends with ], there will be an empty string at the end)
            let fields_to_process = if split_fields.last() == Some(&"") {
                &split_fields[..split_fields.len() - 1]
            } else {
                &split_fields[..]
            };

            fields_to_process
                .iter()
                .map(|s| FieldData::new(s.to_string()))
                .collect()
        } else {
            Ok(Vec::new())
        };

        Message::new(device_id, command, fields?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_access_request() {
        let input = "15+REON+000+0]00000000000011912322]10/05/2025 12:46:06]1]0]";
        let msg = MessageParser::parse(input).unwrap();

        assert_eq!(msg.device_id.as_u8(), 15);
        assert_eq!(msg.command, CommandCode::AccessRequest);
        assert_eq!(msg.field_count(), 4);
        assert_eq!(msg.field(0), Some("00000000000011912322"));
        assert_eq!(msg.field(1), Some("10/05/2025 12:46:06"));
        assert_eq!(msg.field(2), Some("1"));
        assert_eq!(msg.field(3), Some("0"));
    }

    #[test]
    fn test_parse_grant_response() {
        let input = "15+REON+00+6]5]Acesso liberado]";
        let msg = MessageParser::parse(input).unwrap();

        assert_eq!(msg.device_id.as_u8(), 15);
        assert_eq!(msg.command, CommandCode::GrantExit);
        assert_eq!(msg.field_count(), 2);
        assert_eq!(msg.field(0), Some("5"));
        assert_eq!(msg.field(1), Some("Acesso liberado"));
    }

    #[test]
    fn test_parse_waiting_rotation() {
        let input = "15+REON+000+80]]10/05/2025 12:46:06]0]0]";
        let msg = MessageParser::parse(input).unwrap();

        assert_eq!(msg.device_id.as_u8(), 15);
        assert_eq!(msg.command, CommandCode::WaitingRotation);
        // First field is empty (between two ]]), which is preserved
        assert_eq!(msg.field_count(), 4);
        assert_eq!(msg.field(0), Some("")); // Empty card number field
        assert_eq!(msg.field(1), Some("10/05/2025 12:46:06"));
        assert_eq!(msg.field(2), Some("0"));
        assert_eq!(msg.field(3), Some("0"));
    }

    #[test]
    fn test_parse_rotation_completed() {
        let input = "01+REON+000+81]]10/05/2025 12:46:08]1]0]";
        let msg = MessageParser::parse(input).unwrap();

        assert_eq!(msg.device_id.as_u8(), 1);
        assert_eq!(msg.command, CommandCode::RotationCompleted);
    }

    #[test]
    fn test_parse_no_fields() {
        let input = "01+REON+RQ";
        let msg = MessageParser::parse(input).unwrap();

        assert_eq!(msg.device_id.as_u8(), 1);
        assert_eq!(msg.command, CommandCode::QueryStatus);
        assert_eq!(msg.field_count(), 0);
    }

    #[test]
    fn test_parse_missing_device_id() {
        let input = "+REON+000+0]12345678]";
        let result = MessageParser::parse(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_device_id() {
        let input = "999+REON+000+0]12345678]";
        let result = MessageParser::parse(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_missing_reon() {
        let input = "01+XXXX+000+0]12345678]";
        let result = MessageParser::parse(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_command() {
        let input = "01+REON+INVALID]12345678]";
        let result = MessageParser::parse(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_with_whitespace() {
        let input = "  15+REON+000+0]12345678]  \n";
        let msg = MessageParser::parse(input).unwrap();
        assert_eq!(msg.device_id.as_u8(), 15);
    }
}
