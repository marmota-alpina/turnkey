use crate::commands::CommandCode;
use crate::validation::validate_field;
use serde::{Deserialize, Serialize};
use std::fmt;
use turnkey_core::{DeviceId, Error, HenryTimestamp, Result};

/// Parsed Henry protocol message
///
/// Represents a complete Henry protocol message with all components including
/// optional checksum and timestamp for enhanced protocol support.
///
/// # Example
/// ```
/// use turnkey_protocol::{Message, CommandCode};
/// use turnkey_core::DeviceId;
///
/// let device_id = DeviceId::new(15).unwrap();
/// let msg = Message::new(
///     device_id,
///     CommandCode::AccessRequest,
///     vec!["12345678".to_string(), "10/05/2025 12:46:06".to_string()],
/// )
/// .unwrap();
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Device identifier (1-99, zero-padded to 2 digits)
    pub device_id: DeviceId,

    /// Command code identifying the message type
    pub command: CommandCode,

    /// Vector of data fields (can be empty for some commands)
    pub fields: Vec<String>,

    /// Optional checksum for message integrity verification
    pub checksum: Option<String>,

    /// Optional timestamp when the message was created/received
    pub timestamp: Option<HenryTimestamp>,
}

impl Message {
    /// Create a new message with validation
    ///
    /// This is the preferred constructor that returns errors instead of panicking.
    ///
    /// # Errors
    /// Returns `Error::InvalidFieldFormat` if any field contains protocol delimiters
    pub fn new(device_id: DeviceId, command: CommandCode, fields: Vec<String>) -> Result<Self> {
        // Validate all fields
        for field in &fields {
            validate_field(field)?;
        }

        Ok(Message {
            device_id,
            command,
            fields,
            checksum: None,
            timestamp: None,
        })
    }

    /// Create a new message without validation (for testing or trusted inputs)
    ///
    /// Panics if any field contains protocol delimiters.
    ///
    /// Safety Only use this when you are certain the fields are valid. Prefer `new()` for
    /// untrusted input.
    pub fn new_unchecked(device_id: DeviceId, command: CommandCode, fields: Vec<String>) -> Self {
        // Validate fields for protocol safety
        for field in &fields {
            validate_field(field).expect("Field validation failed");
        }

        Message {
            device_id,
            command,
            fields,
            checksum: None,
            timestamp: None,
        }
    }

    /// Create a new message with validation (deprecated alias for `new()`)
    ///
    /// # Errors
    /// Returns `Error::InvalidFieldFormat` if any field contains protocol delimiters
    #[deprecated(since = "0.1.0", note = "Use `new()` instead - it now returns Result")]
    pub fn try_new(device_id: DeviceId, command: CommandCode, fields: Vec<String>) -> Result<Self> {
        Self::new(device_id, command, fields)
    }

    /// Create a new message with all fields including checksum and timestamp
    ///
    /// # Errors
    /// Returns `Error::InvalidFieldFormat` if any field contains protocol delimiters
    pub fn with_metadata(
        device_id: DeviceId,
        command: CommandCode,
        fields: Vec<String>,
        checksum: Option<String>,
        timestamp: Option<HenryTimestamp>,
    ) -> Result<Self> {
        // Validate fields for protocol safety
        for field in &fields {
            validate_field(field)?;
        }

        Ok(Message {
            device_id,
            command,
            fields,
            checksum,
            timestamp,
        })
    }

    /// Create a new message with all fields without validation (for testing or trusted inputs)
    ///
    /// Panics if any field contains protocol delimiters.
    ///
    /// Safety Only use this when you are certain the fields are valid. Prefer `with_metadata()` for
    /// untrusted input.
    pub fn with_metadata_unchecked(
        device_id: DeviceId,
        command: CommandCode,
        fields: Vec<String>,
        checksum: Option<String>,
        timestamp: Option<HenryTimestamp>,
    ) -> Self {
        // Validate fields for protocol safety
        for field in &fields {
            validate_field(field).expect("Field validation failed");
        }

        Message {
            device_id,
            command,
            fields,
            checksum,
            timestamp,
        }
    }

    /// Get field by index
    ///
    /// Returns None if the index is out of bounds.
    pub fn field(&self, index: usize) -> Option<&str> {
        self.fields.get(index).map(|s| s.as_str())
    }

    /// Get required field or return error
    ///
    /// # Errors
    /// Returns `Error::MissingField` if the field at the given index doesn't exist.
    pub fn required_field(&self, index: usize, name: &str) -> Result<&str> {
        self.field(index)
            .ok_or_else(|| Error::MissingField(name.to_string()))
    }

    /// Number of fields in the message
    pub fn field_count(&self) -> usize {
        self.fields.len()
    }

    /// Check if message has a checksum
    pub fn has_checksum(&self) -> bool {
        self.checksum.is_some()
    }

    /// Check if message has a timestamp
    pub fn has_timestamp(&self) -> bool {
        self.timestamp.is_some()
    }

    /// Set the checksum for this message
    pub fn set_checksum(&mut self, checksum: String) {
        self.checksum = Some(checksum);
    }

    /// Set the timestamp for this message
    pub fn set_timestamp(&mut self, timestamp: HenryTimestamp) {
        self.timestamp = Some(timestamp);
    }
}

/// Message type enum for pattern matching
///
/// Provides a higher-level categorization of messages based on their command codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    /// Access request from turnstile (card read)
    AccessRequest,
    /// Access response from server (grant/deny)
    AccessResponse,
    /// Turnstile waiting for user to pass through
    WaitingForRotation,
    /// Turnstile rotation completed
    RotationCompleted,
    /// Rotation timeout or cancelled
    RotationTimeout,
    /// Configuration message
    Configuration,
    /// Status query
    StatusQuery,
    /// Other/unknown message type
    Other,
}

impl Message {
    /// Get the message type based on the command code
    pub fn message_type(&self) -> MessageType {
        match self.command {
            CommandCode::AccessRequest => MessageType::AccessRequest,
            CommandCode::GrantEntry
            | CommandCode::GrantExit
            | CommandCode::GrantBoth
            | CommandCode::DenyAccess => MessageType::AccessResponse,
            CommandCode::WaitingRotation => MessageType::WaitingForRotation,
            CommandCode::RotationCompleted => MessageType::RotationCompleted,
            CommandCode::RotationTimeout => MessageType::RotationTimeout,
            CommandCode::SendConfig | CommandCode::ReceiveConfig => MessageType::Configuration,
            CommandCode::QueryStatus => MessageType::StatusQuery,
            _ => MessageType::Other,
        }
    }
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Message[device={}, cmd={}, fields={}, checksum={}, timestamp={}]",
            self.device_id,
            self.command.as_str(),
            self.field_count(),
            self.checksum.as_deref().unwrap_or("none"),
            self.timestamp
                .as_ref()
                .map(|t| t.to_string())
                .unwrap_or_else(|| "none".to_string())
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let device_id = DeviceId::new(15).unwrap();
        let msg = Message::new(
            device_id,
            CommandCode::AccessRequest,
            vec!["12345678".to_string(), "10/05/2025 12:46:06".to_string()],
        )
        .unwrap();

        assert_eq!(msg.device_id.as_u8(), 15);
        assert_eq!(msg.command, CommandCode::AccessRequest);
        assert_eq!(msg.field_count(), 2);
        assert_eq!(msg.field(0), Some("12345678"));
        assert!(!msg.has_checksum());
        assert!(!msg.has_timestamp());
    }

    #[test]
    fn test_message_with_metadata() {
        let device_id = DeviceId::new(15).unwrap();
        let timestamp = HenryTimestamp::parse("10/05/2025 12:46:06").unwrap();
        let msg = Message::with_metadata(
            device_id,
            CommandCode::AccessRequest,
            vec!["12345678".to_string()],
            Some("AB".to_string()),
            Some(timestamp),
        )
        .unwrap();

        assert!(msg.has_checksum());
        assert!(msg.has_timestamp());
        assert_eq!(msg.checksum, Some("AB".to_string()));
    }

    #[test]
    fn test_required_field() {
        let device_id = DeviceId::new(1).unwrap();
        let msg = Message::new(
            device_id,
            CommandCode::AccessRequest,
            vec!["12345678".to_string()],
        )
        .unwrap();

        assert_eq!(msg.required_field(0, "card").unwrap(), "12345678");
        assert!(msg.required_field(1, "timestamp").is_err());
    }

    #[test]
    fn test_message_type() {
        let device_id = DeviceId::new(1).unwrap();

        let msg = Message::new(device_id, CommandCode::AccessRequest, vec![]).unwrap();
        assert_eq!(msg.message_type(), MessageType::AccessRequest);

        let msg = Message::new(device_id, CommandCode::GrantExit, vec![]).unwrap();
        assert_eq!(msg.message_type(), MessageType::AccessResponse);

        let msg = Message::new(device_id, CommandCode::WaitingRotation, vec![]).unwrap();
        assert_eq!(msg.message_type(), MessageType::WaitingForRotation);
    }

    #[test]
    fn test_set_timestamp() {
        let device_id = DeviceId::new(15).unwrap();
        let mut msg = Message::new(device_id, CommandCode::AccessRequest, vec![]).unwrap();

        assert!(!msg.has_timestamp());

        let timestamp = HenryTimestamp::parse("10/05/2025 12:46:06").unwrap();
        msg.set_timestamp(timestamp);

        assert!(msg.has_timestamp());
    }

    #[test]
    fn test_display() {
        let device_id = DeviceId::new(15).unwrap();
        let msg = Message::new(
            device_id,
            CommandCode::AccessRequest,
            vec!["12345678".to_string(), "data2".to_string()],
        )
        .unwrap();

        let display = format!("{}", msg);
        assert!(display.contains("device=15"));
        assert!(display.contains("cmd=000+0"));
        assert!(display.contains("fields=2"));
        assert!(display.contains("checksum=none"));
    }

    #[test]
    fn test_empty_fields() {
        let device_id = DeviceId::new(1).unwrap();
        let msg = Message::new(device_id, CommandCode::QueryStatus, vec![]).unwrap();

        assert_eq!(msg.field_count(), 0);
        assert_eq!(msg.field(0), None);
    }

    #[test]
    #[should_panic(expected = "Field validation failed")]
    fn test_field_with_field_delimiter_panics() {
        let device_id = DeviceId::new(15).unwrap();
        let _msg = Message::new_unchecked(
            device_id,
            CommandCode::AccessRequest,
            vec!["field]with]delimiters".to_string()],
        );
    }

    #[test]
    #[should_panic(expected = "Field validation failed")]
    fn test_field_with_device_delimiter_panics() {
        let device_id = DeviceId::new(15).unwrap();
        let _msg = Message::new_unchecked(
            device_id,
            CommandCode::AccessRequest,
            vec!["field+with+plus".to_string()],
        );
    }

    #[test]
    #[should_panic(expected = "Field validation failed")]
    fn test_field_with_subfield_delimiter_panics() {
        let device_id = DeviceId::new(15).unwrap();
        let _msg = Message::new_unchecked(
            device_id,
            CommandCode::AccessRequest,
            vec!["field[with[brackets".to_string()],
        );
    }

    #[test]
    fn test_try_new_with_invalid_field() {
        let device_id = DeviceId::new(15).unwrap();
        #[allow(deprecated)]
        let result = Message::try_new(
            device_id,
            CommandCode::AccessRequest,
            vec!["field]invalid".to_string()],
        );

        assert!(result.is_err());
        if let Err(Error::InvalidFieldFormat { message }) = result {
            assert!(message.contains("reserved protocol delimiters"));
        } else {
            panic!("Expected InvalidFieldFormat error");
        }
    }

    #[test]
    fn test_try_new_with_valid_fields() {
        let device_id = DeviceId::new(15).unwrap();
        #[allow(deprecated)]
        let result = Message::try_new(
            device_id,
            CommandCode::AccessRequest,
            vec!["12345678".to_string(), "10/05/2025 12:46:06".to_string()],
        );

        assert!(result.is_ok());
        let msg = result.unwrap();
        assert_eq!(msg.field_count(), 2);
    }
}
