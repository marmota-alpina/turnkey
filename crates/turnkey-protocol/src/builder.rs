use crate::{commands::CommandCode, frame::Frame, message::Message, validation::validate_field};
use turnkey_core::{DeviceId, HenryTimestamp, Result};

/// Builder for constructing Henry protocol messages with a fluent API
///
/// Provides a convenient way to build messages with optional components
/// like checksum and timestamp.
///
/// # Example
/// ```
/// use turnkey_protocol::{MessageBuilder, CommandCode};
/// use turnkey_core::DeviceId;
///
/// let device_id = DeviceId::new(15).unwrap();
/// let msg = MessageBuilder::new(device_id, CommandCode::AccessRequest)
///     .field("12345678").unwrap()
///     .field("10/05/2025 12:46:06").unwrap()
///     .with_auto_checksum()
///     .build()
///     .unwrap();
/// ```
pub struct MessageBuilder {
    device_id: DeviceId,
    command: CommandCode,
    fields: Vec<String>,
    checksum: Option<String>,
    timestamp: Option<HenryTimestamp>,
    auto_checksum: bool,
}

impl MessageBuilder {
    /// Create a new message builder with device ID and command
    pub fn new(device_id: DeviceId, command: CommandCode) -> Self {
        MessageBuilder {
            device_id,
            command,
            fields: Vec::new(),
            checksum: None,
            timestamp: None,
            auto_checksum: false,
        }
    }

    /// Add a single field to the message
    ///
    /// Fields are added in order and will appear in the wire format in the same order.
    ///
    /// # Errors
    /// Returns error if the field contains protocol delimiters (], +, or [).
    pub fn field(mut self, value: impl Into<String>) -> Result<Self> {
        let field = value.into();
        validate_field(&field)?;
        self.fields.push(field);
        Ok(self)
    }

    /// Add multiple fields to the message
    ///
    /// # Errors
    /// Returns error if any field contains protocol delimiters (], +, or [).
    pub fn fields(mut self, values: Vec<String>) -> Result<Self> {
        for field in &values {
            validate_field(field)?;
        }
        self.fields.extend(values);
        Ok(self)
    }

    /// Set a specific checksum value
    ///
    /// If both `checksum()` and `with_auto_checksum()` are called,
    /// the explicit checksum takes precedence.
    pub fn checksum(mut self, checksum: impl Into<String>) -> Self {
        self.checksum = Some(checksum.into());
        self.auto_checksum = false;
        self
    }

    /// Enable automatic checksum calculation
    ///
    /// The checksum will be calculated when `build()` is called.
    pub fn with_auto_checksum(mut self) -> Self {
        self.auto_checksum = true;
        self
    }

    /// Set the timestamp for the message
    pub fn timestamp(mut self, timestamp: HenryTimestamp) -> Self {
        self.timestamp = Some(timestamp);
        self
    }

    /// Set the timestamp to the current time
    pub fn with_current_timestamp(mut self) -> Self {
        self.timestamp = Some(HenryTimestamp::now());
        self
    }

    /// Validate the message fields
    ///
    /// Returns Ok(()) if validation passes, or an error describing the problem.
    fn validate(&self) -> Result<()> {
        // Add validation rules here as needed
        // For now, basic validation is done by the types themselves
        Ok(())
    }

    /// Build the message
    ///
    /// # Errors
    /// Returns error if validation fails
    pub fn build(self) -> Result<Message> {
        self.validate()?;

        // Fields are already validated by field()/fields() methods,
        // so we can use unchecked here for better performance
        let msg = Message::with_metadata_unchecked(
            self.device_id,
            self.command,
            self.fields,
            self.checksum,
            self.timestamp,
        );

        Ok(msg)
    }

    /// Build the message without validation (for testing or trusted inputs)
    ///
    /// This is faster than `build()` but skips validation checks.
    pub fn build_unchecked(self) -> Message {
        Message::with_metadata_unchecked(
            self.device_id,
            self.command,
            self.fields,
            self.checksum,
            self.timestamp,
        )
    }

    /// Build and format as protocol string
    pub fn build_string(self) -> Result<String> {
        let msg = self.build()?;
        Ok(format_message(&msg))
    }

    /// Build and convert to Frame for wire transmission
    ///
    /// If auto_checksum is enabled and no explicit checksum was set,
    /// the checksum will be calculated at the Frame level (on wire format).
    pub fn build_frame(self) -> Result<Frame> {
        let should_auto_checksum = self.auto_checksum && self.checksum.is_none();
        let msg = self.build()?;
        let mut frame = Frame::from(msg);

        // Auto-calculate checksum at Frame level for wire format integrity
        if should_auto_checksum {
            let checksum = frame.calculate_checksum();
            frame.set_checksum(checksum);
        }

        Ok(frame)
    }
}

/// Format message to protocol string
///
/// Converts a Message into the Henry protocol wire format:
/// `<ID>+REON+<COMMAND>+<DATA_FIELDS>`
///
/// This function uses the Frame conversion internally to ensure consistency
/// with wire protocol encoding and eliminate code duplication.
///
/// # Example
/// ```
/// use turnkey_protocol::{Message, CommandCode, format_message};
/// use turnkey_core::DeviceId;
///
/// let device_id = DeviceId::new(15).unwrap();
/// let msg = Message::new(
///     device_id,
///     CommandCode::AccessRequest,
///     vec!["12345678".to_string()],
/// )
/// .unwrap();
///
/// let formatted = format_message(&msg);
/// assert_eq!(formatted, "15+REON+000+0]12345678]");
/// ```
pub fn format_message(msg: &Message) -> String {
    // Use Frame conversion to avoid code duplication
    // Frame::from() handles all the protocol formatting logic
    let frame = Frame::from(msg.clone());

    // Frame::to_string() returns Result, but we know it will succeed
    // because we just created the frame from a valid Message
    frame.to_string().expect("Frame from valid Message should always convert to string")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_access_request() {
        let msg = MessageBuilder::new(DeviceId::new(15).unwrap(), CommandCode::AccessRequest)
            .field("12345678")
            .unwrap()
            .field("10/05/2025 12:46:06")
            .unwrap()
            .field("1")
            .unwrap()
            .field("0")
            .unwrap()
            .build_string()
            .unwrap();

        assert_eq!(msg, "15+REON+000+0]12345678]10/05/2025 12:46:06]1]0]");
    }

    #[test]
    fn test_build_grant_response() {
        let msg = MessageBuilder::new(DeviceId::new(15).unwrap(), CommandCode::GrantExit)
            .field("5")
            .unwrap()
            .field("Acesso liberado")
            .unwrap()
            .build_string()
            .unwrap();

        assert_eq!(msg, "15+REON+00+6]5]Acesso liberado]");
    }

    #[test]
    fn test_build_no_fields() {
        let msg = MessageBuilder::new(DeviceId::new(1).unwrap(), CommandCode::QueryStatus)
            .build_string()
            .unwrap();

        assert_eq!(msg, "01+REON+RQ");
    }

    #[test]
    fn test_build_with_fields_vec() {
        let fields = vec!["field1".to_string(), "field2".to_string()];
        let msg = MessageBuilder::new(DeviceId::new(10).unwrap(), CommandCode::SendCards)
            .fields(fields)
            .unwrap()
            .build_string()
            .unwrap();

        assert_eq!(msg, "10+REON+ECAR]field1]field2]");
    }

    #[test]
    fn test_build_message_object() {
        let msg = MessageBuilder::new(DeviceId::new(5).unwrap(), CommandCode::AccessRequest)
            .field("12345678")
            .unwrap()
            .build()
            .unwrap();

        assert_eq!(msg.device_id.as_u8(), 5);
        assert_eq!(msg.command, CommandCode::AccessRequest);
        assert_eq!(msg.field_count(), 1);
        assert_eq!(msg.field(0), Some("12345678"));
    }

    #[test]
    fn test_build_with_checksum() {
        let msg = MessageBuilder::new(DeviceId::new(15).unwrap(), CommandCode::AccessRequest)
            .field("12345678")
            .unwrap()
            .checksum("AB")
            .build()
            .unwrap();

        assert!(msg.has_checksum());
        assert_eq!(msg.checksum, Some("AB".to_string()));
    }

    #[test]
    fn test_build_with_auto_checksum() {
        // Auto-checksum is applied at Frame level, not Message level
        let frame = MessageBuilder::new(DeviceId::new(15).unwrap(), CommandCode::AccessRequest)
            .field("12345678")
            .unwrap()
            .with_auto_checksum()
            .build_frame()
            .unwrap();

        // Frame should have checksum
        assert!(frame.checksum().is_some());
        // Verify checksum is valid
        assert_eq!(frame.verify_checksum().unwrap(), true);
    }

    #[test]
    fn test_explicit_checksum_overrides_auto() {
        let msg = MessageBuilder::new(DeviceId::new(15).unwrap(), CommandCode::AccessRequest)
            .field("12345678")
            .unwrap()
            .checksum("FF")
            .with_auto_checksum()
            .build()
            .unwrap();

        // Explicit checksum should be used, not auto-calculated
        assert_eq!(msg.checksum, Some("FF".to_string()));
    }

    #[test]
    fn test_build_with_timestamp() {
        let timestamp = HenryTimestamp::parse("10/05/2025 12:46:06").unwrap();
        let msg = MessageBuilder::new(DeviceId::new(15).unwrap(), CommandCode::AccessRequest)
            .field("12345678")
            .unwrap()
            .timestamp(timestamp)
            .build()
            .unwrap();

        assert!(msg.has_timestamp());
    }

    #[test]
    fn test_build_with_current_timestamp() {
        let msg = MessageBuilder::new(DeviceId::new(15).unwrap(), CommandCode::AccessRequest)
            .field("12345678")
            .unwrap()
            .with_current_timestamp()
            .build()
            .unwrap();

        assert!(msg.has_timestamp());
    }

    #[test]
    fn test_build_frame() {
        let frame = MessageBuilder::new(DeviceId::new(15).unwrap(), CommandCode::AccessRequest)
            .field("12345678")
            .unwrap()
            .build_frame()
            .unwrap();

        let frame_str = frame.to_string().unwrap();
        assert_eq!(frame_str, "15+REON+000+0]12345678]");
    }

    #[test]
    fn test_build_unchecked() {
        let msg = MessageBuilder::new(DeviceId::new(15).unwrap(), CommandCode::AccessRequest)
            .field("12345678")
            .unwrap()
            .build_unchecked();

        assert_eq!(msg.device_id.as_u8(), 15);
        assert_eq!(msg.field_count(), 1);
    }

    #[test]
    fn test_format_message() {
        let msg = Message::new(
            DeviceId::new(15).unwrap(),
            CommandCode::GrantExit,
            vec!["5".to_string(), "Acesso liberado".to_string()],
        )
        .unwrap();

        let formatted = format_message(&msg);
        assert_eq!(formatted, "15+REON+00+6]5]Acesso liberado]");
    }

    #[test]
    fn test_format_message_empty_fields() {
        let msg =
            Message::new(DeviceId::new(1).unwrap(), CommandCode::QueryStatus, vec![]).unwrap();

        let formatted = format_message(&msg);
        assert_eq!(formatted, "01+REON+RQ");
    }

    #[test]
    fn test_builder_fluent_api() {
        // Test that builder methods can be chained
        let msg = MessageBuilder::new(DeviceId::new(15).unwrap(), CommandCode::AccessRequest)
            .field("12345678")
            .unwrap()
            .field("10/05/2025 12:46:06")
            .unwrap()
            .checksum("AB")
            .with_current_timestamp()
            .build()
            .unwrap();

        assert_eq!(msg.field_count(), 2);
        assert!(msg.has_checksum());
        assert!(msg.has_timestamp());
    }

    #[test]
    fn test_builder_reusability() {
        // Same builder pattern can create different messages
        let device_id = DeviceId::new(15).unwrap();

        let msg1 = MessageBuilder::new(device_id, CommandCode::AccessRequest)
            .field("12345678")
            .unwrap()
            .build()
            .unwrap();

        let msg2 = MessageBuilder::new(device_id, CommandCode::GrantExit)
            .field("5")
            .unwrap()
            .build()
            .unwrap();

        assert_eq!(msg1.command, CommandCode::AccessRequest);
        assert_eq!(msg2.command, CommandCode::GrantExit);
    }
}
