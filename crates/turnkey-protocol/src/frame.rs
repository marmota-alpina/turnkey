use crate::{commands::CommandCode, message::Message};
use bytes::{BufMut, Bytes, BytesMut};
use std::fmt;
use turnkey_core::{DeviceId, Error, Result, constants::*};

/// Frame represents the byte-level wire protocol format for Henry protocol messages
///
/// A Frame contains the raw bytes of a message including framing markers (STX/ETX),
/// field separators, and all message components in their wire format.
///
/// # Wire Format
/// The Henry protocol uses ASCII-based framing with specific delimiters:
/// - Device separator: `+`
/// - Field separator: `]`
/// - Subfield separator: `[`
/// - Protocol identifier: `REON`
/// - Optional STX (0x02) at start, ETX (0x03) at end
///
/// # Protocol Flow Example
/// A complete turnstile access cycle follows this sequence:
///
/// **1. Turnstile Requests Access (Card Read)**
/// ```text
/// 15+REON+000+0]00000000000011912322]10/05/2025 12:46:06]1]0]
///   ^^      ^^^^^ ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
///   ID      CMD   Card | Timestamp | Direction | Indicator
/// ```
///
/// **2. Server Grants Access**
/// ```text
/// 15+REON+00+6]5]Acesso liberado]
///   ^^      ^^^^ ^^^^^^^^^^^^^^^^^^
///   ID      CMD  Seconds | Message
/// ```
///
/// **3. Turnstile Waits for Rotation**
/// ```text
/// 15+REON+000+80]]10/05/2025 12:46:06]0]0]
///   ^^      ^^^^^^ ^^^^^^^^^^^^^^^^^^^^^^^^^^
///   ID      CMD    Empty | Timestamp | ...
/// ```
///
/// **4. Turnstile Confirms Rotation**
/// ```text
/// 15+REON+000+81]]10/05/2025 12:46:08]1]0]
///   ^^      ^^^^^^ ^^^^^^^^^^^^^^^^^^^^^^^^^^
///   ID      CMD    Empty | Timestamp | ...
/// ```
///
/// # Basic Usage
/// ```
/// use turnkey_protocol::{Frame, Message, CommandCode};
/// use turnkey_core::DeviceId;
///
/// // Create a message
/// let device_id = DeviceId::new(15).unwrap();
/// let msg = Message::new(
///     device_id,
///     CommandCode::AccessRequest,
///     vec![
///         "12345678".to_string(),
///         "10/05/2025 12:46:06".to_string(),
///         "1".to_string(),
///         "0".to_string(),
///     ],
/// )
/// .unwrap();
///
/// // Convert to wire format
/// let frame = Frame::from(msg);
/// let bytes = frame.as_bytes();
///
/// // Add framing for transmission
/// let framed = frame.with_framing();
/// assert_eq!(framed.as_bytes()[0], 0x02); // STX
/// ```
#[derive(Debug, Clone)]
pub struct Frame {
    /// Raw bytes of the frame including all delimiters
    data: Bytes,

    /// Size of the frame in bytes
    size: usize,

    /// Whether this frame includes STX/ETX framing bytes
    has_framing: bool,

    /// Optional checksum for the frame (stored separately from data)
    checksum: Option<String>,
}

impl Frame {
    /// Create a new Frame from raw bytes
    ///
    /// # Arguments
    /// * `data` - Raw bytes containing the frame data
    /// * `has_framing` - Whether the data includes STX/ETX bytes
    pub fn new(data: Bytes, has_framing: bool) -> Self {
        let size = data.len();
        Frame {
            data,
            size,
            has_framing,
            checksum: None,
        }
    }

    /// Get the content bytes of the frame, excluding framing bytes if present
    ///
    /// Returns a slice to the actual message content:
    /// - If frame has STX/ETX framing: returns bytes between STX and ETX
    /// - If frame has no framing: returns all bytes
    ///
    /// This is used internally for operations on the message payload.
    fn content_bytes(&self) -> &[u8] {
        if self.has_framing && self.size >= FRAME_OVERHEAD {
            // Skip STX (first byte) and ETX (last byte)
            &self.data[1..self.size - 1]
        } else {
            &self.data[..]
        }
    }

    /// Create a Frame from a byte slice
    pub fn from_bytes(bytes: &[u8], has_framing: bool) -> Self {
        Self::new(Bytes::copy_from_slice(bytes), has_framing)
    }

    /// Create a Frame from an ASCII string (most common case)
    pub fn from_string(s: &str, has_framing: bool) -> Self {
        Self::from_bytes(s.as_bytes(), has_framing)
    }

    /// Get the raw bytes of the frame
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    /// Get the frame size in bytes
    pub fn size(&self) -> usize {
        self.size
    }

    /// Check if frame has STX/ETX framing
    pub fn has_framing(&self) -> bool {
        self.has_framing
    }

    /// Set the checksum for this frame
    pub fn set_checksum(&mut self, checksum: String) {
        self.checksum = Some(checksum);
    }

    /// Get the checksum if present
    pub fn checksum(&self) -> Option<&str> {
        self.checksum.as_deref()
    }

    /// Add STX/ETX framing bytes to the frame
    ///
    /// Returns a new Frame with framing added. If framing already exists, returns self.
    pub fn with_framing(self) -> Self {
        if self.has_framing {
            return self;
        }

        let mut buf = BytesMut::with_capacity(self.size + FRAME_OVERHEAD);
        buf.put_u8(START_BYTE);
        buf.put_slice(&self.data);
        buf.put_u8(END_BYTE);

        let size = buf.len();

        Frame {
            data: buf.freeze(),
            size,
            has_framing: true,
            checksum: self.checksum,
        }
    }

    /// Remove STX/ETX framing bytes from the frame
    ///
    /// Returns a new Frame with framing removed. If no framing exists, returns self.
    pub fn without_framing(self) -> Self {
        if !self.has_framing {
            return self;
        }

        let data = &self.data;
        if data.len() < FRAME_OVERHEAD {
            return self;
        }

        // Check if first and last bytes are STX/ETX
        if data[0] == START_BYTE && data[data.len() - 1] == END_BYTE {
            let inner = &data[1..data.len() - 1];
            Frame {
                data: Bytes::copy_from_slice(inner),
                size: inner.len(),
                has_framing: false,
                checksum: self.checksum,
            }
        } else {
            self
        }
    }

    /// Convert the frame to a UTF-8 string (without STX/ETX if present)
    ///
    /// # Errors
    /// Returns error if the frame contains invalid UTF-8
    pub fn to_string(&self) -> Result<String> {
        let bytes = self.content_bytes();

        String::from_utf8(bytes.to_vec()).map_err(|e| Error::InvalidMessageFormat {
            message: format!("Invalid UTF-8: {}", e),
        })
    }

    /// Calculate XOR checksum for the frame content (excluding STX/ETX)
    pub fn calculate_checksum(&self) -> String {
        let bytes = self.content_bytes();

        let checksum: u8 = bytes.iter().fold(0u8, |acc, &b| acc ^ b);
        format!("{:02X}", checksum)
    }

    /// Verify the stored checksum against calculated checksum
    ///
    /// # Returns
    /// - `Ok(true)` if checksum is present and matches
    /// - `Ok(false)` if no checksum is present (nothing to verify)
    /// - `Err(ChecksumMismatch)` if checksum is present but does not match
    ///
    /// # Errors
    /// Returns `Error::ChecksumMismatch` if the stored checksum does not match
    /// the calculated checksum.
    pub fn verify_checksum(&self) -> Result<bool> {
        match &self.checksum {
            None => Ok(false), // No checksum to verify
            Some(stored) => {
                let calculated = self.calculate_checksum();
                if stored == &calculated {
                    Ok(true)
                } else {
                    Err(Error::ChecksumMismatch {
                        expected: calculated, // Fix: calculated is expected
                        actual: stored.clone(),
                    })
                }
            }
        }
    }
}

/// Convert Message to Frame (wire format)
impl From<Message> for Frame {
    fn from(msg: Message) -> Self {
        // Build the protocol string: <ID>+REON+<COMMAND>+<DATA_FIELDS>
        // Calculate approximate capacity:
        // - Device ID: 2 bytes
        // - Delimiters: 2 bytes (+ +)
        // - Protocol ID: 4 bytes (REON)
        // - Command: ~5 bytes average
        // - Fields: sum of field lengths + delimiters
        let fields_size: usize = msg.fields.iter().map(|f| f.len() + 1).sum(); // +1 for delimiter
        let capacity = 13 + fields_size + 1; // +1 for trailing delimiter if fields exist

        let mut buffer = String::with_capacity(capacity);

        // Device ID (zero-padded to 2 digits)
        buffer.push_str(&msg.device_id.to_string_padded());
        buffer.push_str(DELIMITER_DEVICE);

        // Protocol ID
        buffer.push_str(PROTOCOL_ID);
        buffer.push_str(DELIMITER_DEVICE);

        // Command code
        buffer.push_str(msg.command.as_str());

        // Data fields
        if !msg.fields.is_empty() {
            for field in &msg.fields {
                buffer.push_str(DELIMITER_FIELD);
                buffer.push_str(field);
            }
            buffer.push_str(DELIMITER_FIELD);
        }

        let mut frame = Frame::from_string(&buffer, false);

        // Preserve explicit checksum from message if present
        if let Some(checksum) = msg.checksum {
            frame.set_checksum(checksum);
        }

        frame
    }
}

/// Parse device ID from string part
fn parse_device_id(part: &str) -> Result<DeviceId> {
    part.parse().map_err(|e| Error::InvalidMessageFormat {
        message: format!("Invalid device ID '{}': {}", part, e),
    })
}

/// Validate protocol ID matches expected value
fn validate_protocol_id(part: &str) -> Result<()> {
    if part != PROTOCOL_ID {
        return Err(Error::InvalidMessageFormat {
            message: format!("Expected protocol ID '{}', got '{}'", PROTOCOL_ID, part),
        });
    }
    Ok(())
}

/// Parse command and fields from the remaining protocol parts
fn parse_command_and_fields(parts: &[&str]) -> Result<(CommandCode, Vec<String>)> {
    if parts.is_empty() {
        return Err(Error::InvalidMessageFormat {
            message: "Missing command part".to_string(),
        });
    }

    // Join remaining parts and split by field delimiter
    let command_and_fields = parts.join(DELIMITER_DEVICE);
    let field_parts: Vec<&str> = command_and_fields.split(DELIMITER_FIELD).collect();

    // First part is the command
    let command = CommandCode::parse(field_parts[0])?;

    // Remaining parts are data fields (filter out empty strings)
    let fields: Vec<String> = field_parts[1..]
        .iter()
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();

    Ok((command, fields))
}

/// Convert Frame to Message with validation
impl TryFrom<Frame> for Message {
    type Error = Error;

    fn try_from(frame: Frame) -> Result<Self> {
        // Get the string representation (automatically handles STX/ETX removal)
        let frame_str = frame.to_string()?;

        // Parse the protocol string: <ID>+REON+<COMMAND>+<DATA_FIELDS>
        let parts: Vec<&str> = frame_str.split(DELIMITER_DEVICE).collect();

        if parts.len() < 3 {
            return Err(Error::InvalidMessageFormat {
                message: format!(
                    "Expected at least 3 parts (ID+REON+CMD), got {}",
                    parts.len()
                ),
            });
        }

        // Parse device ID
        let device_id = parse_device_id(parts[0])?;

        // Verify protocol ID
        validate_protocol_id(parts[1])?;

        // Parse command and fields
        let (command, fields) = parse_command_and_fields(&parts[2..])?;

        // Create message with checksum from frame
        let mut msg = Message::new(device_id, command, fields)?;
        if let Some(checksum) = frame.checksum {
            msg.set_checksum(checksum);
        }

        Ok(msg)
    }
}

impl fmt::Display for Frame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let content = match self.to_string() {
            Ok(s) => s,
            Err(_) => {
                // Include hex representation for debugging invalid UTF-8
                let bytes = self.content_bytes();
                let hex: String = bytes
                    .iter()
                    .map(|b| format!("{:02X}", b))
                    .collect::<Vec<_>>()
                    .join(" ");
                format!("<invalid UTF-8: {}>", hex)
            }
        };
        write!(
            f,
            "Frame[size={}, framing={}, content='{}']",
            self.size, self.has_framing, content
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_creation() {
        let data = b"15+REON+000+0]12345678]";
        let frame = Frame::from_bytes(data, false);

        assert_eq!(frame.size(), data.len());
        assert!(!frame.has_framing());
        assert_eq!(frame.as_bytes(), data);
    }

    #[test]
    fn test_frame_from_string() {
        let msg_str = "15+REON+000+0]12345678]";
        let frame = Frame::from_string(msg_str, false);

        assert_eq!(frame.to_string().unwrap(), msg_str);
    }

    #[test]
    fn test_frame_with_framing() {
        let frame = Frame::from_string("15+REON+RQ", false);
        let framed = frame.with_framing();

        assert!(framed.has_framing());
        assert_eq!(framed.as_bytes()[0], START_BYTE);
        assert_eq!(framed.as_bytes()[framed.size() - 1], END_BYTE);
        assert_eq!(framed.to_string().unwrap(), "15+REON+RQ");
    }

    #[test]
    fn test_frame_without_framing() {
        let mut data = BytesMut::new();
        data.put_u8(START_BYTE);
        data.put_slice(b"15+REON+RQ");
        data.put_u8(END_BYTE);

        let frame = Frame::new(data.freeze(), true);
        let unframed = frame.without_framing();

        assert!(!unframed.has_framing());
        assert_eq!(unframed.to_string().unwrap(), "15+REON+RQ");
    }

    #[test]
    fn test_frame_checksum() {
        let frame = Frame::from_string("15+REON+000+0", false);
        let checksum = frame.calculate_checksum();

        assert!(!checksum.is_empty());
        assert_eq!(checksum.len(), 2); // Hex format
    }

    #[test]
    fn test_message_to_frame_conversion() {
        let device_id = DeviceId::new(15).unwrap();
        let msg = Message::new(
            device_id,
            CommandCode::AccessRequest,
            vec!["12345678".to_string(), "10/05/2025 12:46:06".to_string()],
        )
        .unwrap();

        let frame = Frame::from(msg);
        let frame_str = frame.to_string().unwrap();

        assert_eq!(frame_str, "15+REON+000+0]12345678]10/05/2025 12:46:06]");
    }

    #[test]
    fn test_message_to_frame_no_fields() {
        let device_id = DeviceId::new(1).unwrap();
        let msg = Message::new(device_id, CommandCode::QueryStatus, vec![]).unwrap();

        let frame = Frame::from(msg);
        let frame_str = frame.to_string().unwrap();

        assert_eq!(frame_str, "01+REON+RQ");
    }

    #[test]
    fn test_frame_to_message_conversion() {
        let frame = Frame::from_string("15+REON+000+0]12345678]10/05/2025 12:46:06]", false);
        let msg = Message::try_from(frame).unwrap();

        assert_eq!(msg.device_id.as_u8(), 15);
        assert_eq!(msg.command, CommandCode::AccessRequest);
        assert_eq!(msg.field_count(), 2);
        assert_eq!(msg.field(0), Some("12345678"));
        assert_eq!(msg.field(1), Some("10/05/2025 12:46:06"));
    }

    #[test]
    fn test_frame_to_message_no_fields() {
        let frame = Frame::from_string("01+REON+RQ", false);
        let msg = Message::try_from(frame).unwrap();

        assert_eq!(msg.device_id.as_u8(), 1);
        assert_eq!(msg.command, CommandCode::QueryStatus);
        assert_eq!(msg.field_count(), 0);
    }

    #[test]
    fn test_frame_to_message_invalid_protocol() {
        let frame = Frame::from_string("15+WRONG+000+0", false);
        let result = Message::try_from(frame);

        assert!(result.is_err());
    }

    #[test]
    fn test_frame_to_message_invalid_command() {
        let frame = Frame::from_string("15+REON+INVALID", false);
        let result = Message::try_from(frame);

        assert!(result.is_err());
    }

    #[test]
    fn test_round_trip_conversion() {
        let device_id = DeviceId::new(15).unwrap();
        let original_msg = Message::new(
            device_id,
            CommandCode::GrantExit,
            vec!["5".to_string(), "Acesso liberado".to_string()],
        )
        .unwrap();

        let frame = Frame::from(original_msg.clone());
        let recovered_msg = Message::try_from(frame).unwrap();

        assert_eq!(recovered_msg.device_id, original_msg.device_id);
        assert_eq!(recovered_msg.command, original_msg.command);
        assert_eq!(recovered_msg.fields, original_msg.fields);
    }

    #[test]
    fn test_frame_display() {
        let frame = Frame::from_string("15+REON+RQ", false);
        let display = format!("{}", frame);

        assert!(display.contains("size=10"));
        assert!(display.contains("framing=false"));
        assert!(display.contains("15+REON+RQ"));
    }

    #[test]
    fn test_frame_checksum_verification() {
        let mut frame = Frame::from_string("15+REON+000+0", false);

        // No checksum set
        assert_eq!(frame.verify_checksum().unwrap(), false);

        // Set correct checksum
        let checksum = frame.calculate_checksum();
        frame.set_checksum(checksum);
        assert_eq!(frame.verify_checksum().unwrap(), true);

        // Set wrong checksum
        frame.set_checksum("FF".to_string());
        assert!(frame.verify_checksum().is_err());
    }

    #[test]
    fn test_frame_with_special_characters_fails() {
        // Fields containing delimiters should cause an error
        let device_id = DeviceId::new(15).unwrap();
        let result = Message::new(
            device_id,
            CommandCode::GrantExit,
            vec!["Field]with]delimiters".to_string()],
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_frame_with_safe_special_characters() {
        // Safe special characters that are not protocol delimiters
        let device_id = DeviceId::new(15).unwrap();
        let msg = Message::new(
            device_id,
            CommandCode::GrantExit,
            vec!["Field with spaces & symbols!".to_string()],
        )
        .unwrap();

        let frame = Frame::from(msg);
        let frame_str = frame.to_string().unwrap();

        assert!(frame_str.contains("Field with spaces & symbols!"));
    }

    #[test]
    fn test_message_with_checksum_to_frame() {
        let device_id = DeviceId::new(15).unwrap();
        let mut msg = Message::new(
            device_id,
            CommandCode::AccessRequest,
            vec!["12345678".to_string()],
        )
        .unwrap();

        // Set explicit checksum on message
        msg.set_checksum("AB".to_string());

        let frame = Frame::from(msg);
        assert_eq!(frame.checksum(), Some("AB"));
    }

    #[test]
    fn test_frame_empty_fields_handling() {
        // Frame with empty fields between delimiters
        let frame = Frame::from_string("15+REON+000+0]]12345678]", false);
        let msg = Message::try_from(frame).unwrap();

        // Empty fields should be filtered out
        assert_eq!(msg.field_count(), 1);
        assert_eq!(msg.field(0), Some("12345678"));
    }

    // Edge case tests

    #[test]
    fn test_device_id_boundary_minimum() {
        // DeviceId minimum is 1, not 0
        let device_id = DeviceId::new(1).unwrap();
        let msg = Message::new(device_id, CommandCode::QueryStatus, vec![]).unwrap();
        let frame = Frame::from(msg);

        assert_eq!(frame.to_string().unwrap(), "01+REON+RQ");
    }

    #[test]
    fn test_device_id_boundary_one() {
        let device_id = DeviceId::new(1).unwrap();
        let msg = Message::new(device_id, CommandCode::QueryStatus, vec![]).unwrap();
        let frame = Frame::from(msg);

        assert_eq!(frame.to_string().unwrap(), "01+REON+RQ");
    }

    #[test]
    fn test_device_id_boundary_99() {
        let device_id = DeviceId::new(99).unwrap();
        let msg = Message::new(device_id, CommandCode::QueryStatus, vec![]).unwrap();
        let frame = Frame::from(msg);

        assert_eq!(frame.to_string().unwrap(), "99+REON+RQ");
    }

    #[test]
    fn test_maximum_fields() {
        let device_id = DeviceId::new(15).unwrap();
        let fields: Vec<String> = (0..100).map(|i| format!("field{}", i)).collect();
        let msg = Message::new(device_id, CommandCode::SendCards, fields.clone()).unwrap();
        let frame = Frame::from(msg);

        let recovered = Message::try_from(frame).unwrap();
        assert_eq!(recovered.field_count(), 100);
        assert_eq!(recovered.field(0), Some("field0"));
        assert_eq!(recovered.field(99), Some("field99"));
    }

    #[test]
    fn test_empty_message_components() {
        // Message with no fields
        let device_id = DeviceId::new(1).unwrap();
        let msg = Message::new(device_id, CommandCode::QueryStatus, vec![]).unwrap();
        let frame = Frame::from(msg);

        let recovered = Message::try_from(frame).unwrap();
        assert_eq!(recovered.field_count(), 0);
        assert_eq!(recovered.device_id.as_u8(), 1);
    }

    #[test]
    fn test_frame_display_with_invalid_utf8() {
        // Create frame with invalid UTF-8 bytes
        let invalid_bytes = vec![0x15, 0x2B, 0x52, 0x45, 0x4F, 0x4E, 0xFF, 0xFE]; // Contains 0xFF, 0xFE
        let frame = Frame::from_bytes(&invalid_bytes, false);

        let display = format!("{}", frame);
        assert!(display.contains("invalid UTF-8"));
        assert!(display.contains("FF FE"));
    }

    #[test]
    fn test_frame_to_message_malformed_no_protocol() {
        let frame = Frame::from_string("15+WRONGPROTO+RQ", false);
        let result = Message::try_from(frame);

        assert!(result.is_err());
        if let Err(Error::InvalidMessageFormat { message }) = result {
            assert!(message.contains("Expected protocol ID"));
        }
    }

    #[test]
    fn test_frame_to_message_malformed_no_command() {
        let frame = Frame::from_string("15+REON", false);
        let result = Message::try_from(frame);

        assert!(result.is_err());
        if let Err(Error::InvalidMessageFormat { message }) = result {
            assert!(message.contains("at least 3 parts"));
        }
    }

    #[test]
    fn test_frame_to_message_malformed_invalid_device_id() {
        let frame = Frame::from_string("ABC+REON+RQ", false);
        let result = Message::try_from(frame);

        assert!(result.is_err());
    }

    #[test]
    fn test_frame_checksum_with_empty_frame() {
        let frame = Frame::from_bytes(&[], false);
        let checksum = frame.calculate_checksum();

        // XOR of empty should be 0
        assert_eq!(checksum, "00");
    }

    #[test]
    fn test_frame_with_only_stx_etx() {
        let mut data = BytesMut::new();
        data.put_u8(START_BYTE);
        data.put_u8(END_BYTE);

        let frame = Frame::new(data.freeze(), true);
        let unframed = frame.without_framing();

        // Should result in empty frame
        assert_eq!(unframed.size(), 0);
    }
}
