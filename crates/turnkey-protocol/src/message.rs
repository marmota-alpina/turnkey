use crate::commands::CommandCode;
use crate::field::FieldData;
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
/// use turnkey_protocol::{Message, CommandCode, FieldData};
/// use turnkey_core::DeviceId;
///
/// let device_id = DeviceId::new(15).unwrap();
/// let msg = Message::new(
///     device_id,
///     CommandCode::AccessRequest,
///     vec![
///         FieldData::new("12345678".to_string()).unwrap(),
///         FieldData::new("10/05/2025 12:46:06".to_string()).unwrap(),
///     ],
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
    pub fields: Vec<FieldData>,

    /// Optional checksum for message integrity verification
    pub checksum: Option<String>,

    /// Optional timestamp when the message was created/received
    pub timestamp: Option<HenryTimestamp>,
}

impl Message {
    /// Create a new message with validation
    ///
    /// This is the preferred constructor that returns errors instead of panicking.
    /// Fields are validated at construction through the FieldData type.
    ///
    /// # Errors
    /// Returns error if device_id or command are invalid.
    /// Field validation is enforced by the FieldData type itself.
    ///
    /// # Example
    /// ```
    /// use turnkey_protocol::{Message, CommandCode, FieldData};
    /// use turnkey_core::DeviceId;
    ///
    /// let device_id = DeviceId::new(15).unwrap();
    /// let msg = Message::new(
    ///     device_id,
    ///     CommandCode::AccessRequest,
    ///     vec![FieldData::new("12345678".to_string()).unwrap()],
    /// ).unwrap();
    /// ```
    pub fn new(device_id: DeviceId, command: CommandCode, fields: Vec<FieldData>) -> Result<Self> {
        // Fields are already validated by FieldData type
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
    /// Since fields are already validated by FieldData type, this is now
    /// equivalent to `new()` but kept for API compatibility.
    ///
    /// # Example
    /// ```
    /// use turnkey_protocol::{Message, CommandCode, FieldData};
    /// use turnkey_core::DeviceId;
    ///
    /// let device_id = DeviceId::new(15).unwrap();
    /// let msg = Message::new_unchecked(
    ///     device_id,
    ///     CommandCode::AccessRequest,
    ///     vec![FieldData::new("12345678".to_string()).unwrap()],
    /// );
    /// ```
    pub fn new_unchecked(
        device_id: DeviceId,
        command: CommandCode,
        fields: Vec<FieldData>,
    ) -> Self {
        // Fields are already validated by FieldData type
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
    /// Returns error if construction fails
    #[deprecated(since = "0.1.0", note = "Use `new()` instead - it now returns Result")]
    pub fn try_new(
        device_id: DeviceId,
        command: CommandCode,
        fields: Vec<FieldData>,
    ) -> Result<Self> {
        Self::new(device_id, command, fields)
    }

    /// Create a new message with all fields including checksum and timestamp
    ///
    /// Fields are validated at construction through the FieldData type.
    ///
    /// # Errors
    /// Returns error if construction fails
    pub fn with_metadata(
        device_id: DeviceId,
        command: CommandCode,
        fields: Vec<FieldData>,
        checksum: Option<String>,
        timestamp: Option<HenryTimestamp>,
    ) -> Result<Self> {
        // Fields are already validated by FieldData type
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
    /// Since fields are already validated by FieldData type, this is now
    /// equivalent to `with_metadata()` but kept for API compatibility.
    pub fn with_metadata_unchecked(
        device_id: DeviceId,
        command: CommandCode,
        fields: Vec<FieldData>,
        checksum: Option<String>,
        timestamp: Option<HenryTimestamp>,
    ) -> Self {
        // Fields are already validated by FieldData type
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
        self.fields.get(index).map(|f| f.as_str())
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

    /// Returns the total length of all fields in bytes (content only).
    ///
    /// This is useful for capacity calculations when encoding messages.
    /// Returns the sum of all field content lengths, **excluding** field
    /// delimiters which must be calculated separately based on field count.
    ///
    /// For complete frame size calculation, see `From<Message> for Frame`.
    ///
    /// # Example
    /// ```
    /// use turnkey_protocol::{Message, CommandCode, FieldData};
    /// use turnkey_core::DeviceId;
    ///
    /// let device_id = DeviceId::new(15).unwrap();
    /// let msg = Message::new(
    ///     device_id,
    ///     CommandCode::AccessRequest,
    ///     vec![
    ///         FieldData::new("12345678".to_string()).unwrap(),  // 8 bytes
    ///         FieldData::new("data".to_string()).unwrap(),       // 4 bytes
    ///     ],
    /// ).unwrap();
    ///
    /// assert_eq!(msg.fields_len(), 12); // 8 + 4
    /// ```
    #[inline]
    pub fn fields_len(&self) -> usize {
        self.fields.iter().map(|f| f.len()).sum()
    }

    /// Calculates the exact frame capacity needed to encode this message.
    ///
    /// Returns the total number of bytes required for the wire format,
    /// allowing pre-allocation without reallocations during frame construction.
    ///
    /// # Frame Structure
    ///
    /// The capacity includes all components of the Henry protocol message:
    /// - Device ID: 2 bytes (zero-padded, e.g., "01", "15", "99")
    /// - First delimiter: 1 byte (+)
    /// - Protocol ID: 4 bytes (REON)
    /// - Second delimiter: 1 byte (+)
    /// - Command: variable length (2-6 bytes depending on command)
    /// - Field delimiters: (fields.len() + 1) bytes if fields exist, 0 otherwise
    /// - Field content: sum of all field lengths
    ///
    /// # Examples
    ///
    /// ```
    /// use turnkey_protocol::{Message, CommandCode, FieldData};
    /// use turnkey_core::DeviceId;
    ///
    /// // Message with fields
    /// let device_id = DeviceId::new(15).unwrap();
    /// let msg = Message::new(
    ///     device_id,
    ///     CommandCode::AccessRequest,
    ///     vec![
    ///         FieldData::new("12345678".to_string()).unwrap(),
    ///         FieldData::new("data".to_string()).unwrap(),
    ///     ],
    /// ).unwrap();
    ///
    /// let capacity = msg.frame_capacity();
    /// // Capacity: 2 (ID) + 1 (+) + 4 (REON) + 1 (+) + 5 (cmd) + 3 (delims) + 12 (fields) = 28
    /// assert_eq!(capacity, 28);
    ///
    /// // Message without fields
    /// let msg = Message::new(
    ///     DeviceId::new(1).unwrap(),
    ///     CommandCode::QueryStatus,
    ///     vec![],
    /// ).unwrap();
    ///
    /// let capacity = msg.frame_capacity();
    /// // Capacity: 2 (ID) + 1 (+) + 4 (REON) + 1 (+) + 2 (RQ) + 0 (no delims) = 10
    /// assert_eq!(capacity, 10);
    /// ```
    #[inline]
    pub fn frame_capacity(&self) -> usize {
        use turnkey_core::constants::{BASE_DELIMITER_COUNT, DEVICE_ID_LENGTH, PROTOCOL_ID_LENGTH};

        let cmd_size = self.command.len();
        let fields_size = self.fields_len();
        let field_delimiters = if self.fields.is_empty() {
            0
        } else {
            self.fields.len() + 1 // ']' before each field + trailing ']'
        };

        DEVICE_ID_LENGTH
            + BASE_DELIMITER_COUNT
            + PROTOCOL_ID_LENGTH
            + cmd_size
            + field_delimiters
            + fields_size
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
            vec![
                FieldData::new("12345678".to_string()).unwrap(),
                FieldData::new("10/05/2025 12:46:06".to_string()).unwrap(),
            ],
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
            vec![FieldData::new("12345678".to_string()).unwrap()],
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
            vec![FieldData::new("12345678".to_string()).unwrap()],
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
            vec![
                FieldData::new("12345678".to_string()).unwrap(),
                FieldData::new("data2".to_string()).unwrap(),
            ],
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
    fn test_field_with_field_delimiter_rejected() {
        // FieldData construction should fail for invalid fields
        let result = FieldData::new("field]with]delimiters".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_field_with_device_delimiter_rejected() {
        // FieldData construction should fail for invalid fields
        let result = FieldData::new("field+with+plus".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_field_with_subfield_delimiter_rejected() {
        // FieldData construction should fail for invalid fields
        let result = FieldData::new("field[with[brackets".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_try_new_with_invalid_field() {
        // FieldData construction should fail for invalid fields
        let field_result = FieldData::new("field]invalid".to_string());
        assert!(field_result.is_err());

        if let Err(Error::InvalidFieldFormat { message }) = field_result {
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
            vec![
                FieldData::new("12345678".to_string()).unwrap(),
                FieldData::new("10/05/2025 12:46:06".to_string()).unwrap(),
            ],
        );

        assert!(result.is_ok());
        let msg = result.unwrap();
        assert_eq!(msg.field_count(), 2);
    }

    #[test]
    fn test_fields_len_with_multiple_fields() {
        // Test total fields length calculation
        let device_id = DeviceId::new(15).unwrap();
        let msg = Message::new(
            device_id,
            CommandCode::AccessRequest,
            vec![
                FieldData::new("12345678".to_string()).unwrap(), // 8 bytes
                FieldData::new("10/05/2025 12:46:06".to_string()).unwrap(), // 19 bytes
                FieldData::new("1".to_string()).unwrap(),        // 1 byte
                FieldData::new("0".to_string()).unwrap(),        // 1 byte
            ],
        )
        .unwrap();

        // Total: 8 + 19 + 1 + 1 = 29 bytes
        assert_eq!(msg.fields_len(), 29);
    }

    #[test]
    fn test_fields_len_with_no_fields() {
        // Test with empty fields vector
        let device_id = DeviceId::new(1).unwrap();
        let msg = Message::new(device_id, CommandCode::QueryStatus, vec![]).unwrap();

        assert_eq!(msg.fields_len(), 0);
        assert_eq!(msg.field_count(), 0);
    }

    #[test]
    fn test_fields_len_with_empty_fields() {
        // Test with fields that have empty content
        let device_id = DeviceId::new(15).unwrap();
        let msg = Message::new(
            device_id,
            CommandCode::WaitingRotation,
            vec![
                FieldData::new("".to_string()).unwrap(), // 0 bytes
                FieldData::new("10/05/2025 12:46:06".to_string()).unwrap(), // 19 bytes
            ],
        )
        .unwrap();

        // Total: 0 + 19 = 19 bytes
        assert_eq!(msg.fields_len(), 19);
    }

    #[test]
    fn test_fields_len_consistency() {
        // Verify that fields_len() matches manual calculation
        let device_id = DeviceId::new(15).unwrap();
        let fields = vec![
            FieldData::new("field1".to_string()).unwrap(),
            FieldData::new("field2".to_string()).unwrap(),
            FieldData::new("field3".to_string()).unwrap(),
        ];

        let manual_sum: usize = fields.iter().map(|f| f.len()).sum();

        let msg = Message::new(device_id, CommandCode::SendCards, fields).unwrap();

        assert_eq!(msg.fields_len(), manual_sum);
    }

    #[test]
    fn test_frame_capacity_with_fields() {
        // Test capacity calculation with multiple fields
        let device_id = DeviceId::new(15).unwrap();
        let msg = Message::new(
            device_id,
            CommandCode::AccessRequest,
            vec![
                FieldData::new("12345678".to_string()).unwrap(), // 8 bytes
                FieldData::new("10/05/2025 12:46:06".to_string()).unwrap(), // 19 bytes
                FieldData::new("1".to_string()).unwrap(),        // 1 byte
                FieldData::new("0".to_string()).unwrap(),        // 1 byte
            ],
        )
        .unwrap();

        // Expected: 2 (ID) + 1 (+) + 4 (REON) + 1 (+) + 5 (000+0) + 5 (delims) + 29 (fields) = 47
        assert_eq!(msg.frame_capacity(), 47);

        // Verify it matches actual frame size
        let frame = crate::Frame::from(msg);
        let frame_str = frame.to_string().unwrap();
        assert_eq!(frame_str.len(), 47);
    }

    #[test]
    fn test_frame_capacity_without_fields() {
        // Test capacity calculation with no fields
        let device_id = DeviceId::new(1).unwrap();
        let msg = Message::new(device_id, CommandCode::QueryStatus, vec![]).unwrap();

        // Expected: 2 (01) + 1 (+) + 4 (REON) + 1 (+) + 2 (RQ) + 0 (no delims) = 10
        assert_eq!(msg.frame_capacity(), 10);

        // Verify it matches actual frame size
        let frame = crate::Frame::from(msg);
        let frame_str = frame.to_string().unwrap();
        assert_eq!(frame_str.len(), 10);
    }

    #[test]
    fn test_frame_capacity_with_long_command() {
        // Test capacity with longer command code
        let device_id = DeviceId::new(15).unwrap();
        let msg = Message::new(
            device_id,
            CommandCode::GrantExit,
            vec![
                FieldData::new("5".to_string()).unwrap(),
                FieldData::new("Acesso liberado".to_string()).unwrap(),
            ],
        )
        .unwrap();

        // Expected: 2 (15) + 1 (+) + 4 (REON) + 1 (+) + 4 (00+6) + 3 (delims) + 16 (fields) = 31
        assert_eq!(msg.frame_capacity(), 31);

        // Verify it matches actual frame size
        let frame = crate::Frame::from(msg);
        let frame_str = frame.to_string().unwrap();
        assert_eq!(frame_str.len(), 31);
    }

    #[test]
    fn test_frame_capacity_matches_actual_frame() {
        // Comprehensive test ensuring capacity always matches actual frame length
        let test_cases = vec![
            (DeviceId::new(1).unwrap(), CommandCode::QueryStatus, vec![]),
            (
                DeviceId::new(15).unwrap(),
                CommandCode::AccessRequest,
                vec![FieldData::new("12345678".to_string()).unwrap()],
            ),
            (
                DeviceId::new(99).unwrap(),
                CommandCode::WaitingRotation,
                vec![
                    FieldData::new("".to_string()).unwrap(),
                    FieldData::new("10/05/2025 12:46:06".to_string()).unwrap(),
                ],
            ),
            (
                DeviceId::new(50).unwrap(),
                CommandCode::SendCards,
                vec![
                    FieldData::new("field1".to_string()).unwrap(),
                    FieldData::new("field2".to_string()).unwrap(),
                    FieldData::new("field3".to_string()).unwrap(),
                ],
            ),
        ];

        for (device_id, command, fields) in test_cases {
            let msg = Message::new(device_id, command, fields).unwrap();
            let capacity = msg.frame_capacity();
            let frame = crate::Frame::from(msg);
            let frame_str = frame.to_string().unwrap();

            assert_eq!(
                capacity,
                frame_str.len(),
                "Capacity mismatch for command {:?}",
                command
            );
        }
    }

    #[test]
    fn test_capacity_constants_are_correct() {
        use turnkey_core::constants::{
            BASE_DELIMITER_COUNT, DEVICE_ID_LENGTH, PROTOCOL_ID, PROTOCOL_ID_LENGTH,
        };

        // Verify constants match actual values
        assert_eq!(
            DEVICE_ID_LENGTH, 2,
            "Device IDs are zero-padded to 2 digits (01-99)"
        );
        assert_eq!(
            PROTOCOL_ID_LENGTH,
            PROTOCOL_ID.len(),
            "PROTOCOL_ID_LENGTH must match PROTOCOL_ID string length"
        );
        assert_eq!(
            BASE_DELIMITER_COUNT, 2,
            "Two '+' delimiters separate ID, REON, and command"
        );
    }
}
