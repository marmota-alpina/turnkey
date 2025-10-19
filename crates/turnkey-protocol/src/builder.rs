use crate::{commands::CommandCode, message::Message};
use turnkey_core::{DeviceId, constants::*};

pub struct MessageBuilder {
    device_id: DeviceId,
    command: CommandCode,
    fields: Vec<String>,
}

impl MessageBuilder {
    pub fn new(device_id: DeviceId, command: CommandCode) -> Self {
        MessageBuilder {
            device_id,
            command,
            fields: Vec::new(),
        }
    }

    pub fn field(mut self, value: impl Into<String>) -> Self {
        self.fields.push(value.into());
        self
    }

    pub fn fields(mut self, values: Vec<String>) -> Self {
        self.fields.extend(values);
        self
    }

    pub fn build(self) -> Message {
        Message::new(self.device_id, self.command, self.fields)
    }

    /// Build and format as protocol string
    pub fn build_string(self) -> String {
        let msg = self.build();
        format_message(&msg)
    }
}

/// Format message to protocol string
pub fn format_message(msg: &Message) -> String {
    let mut output = format!(
        "{}{}{}{}{}",
        msg.device_id,
        DELIMITER_DEVICE,
        PROTOCOL_ID,
        DELIMITER_DEVICE,
        msg.command.as_str()
    );

    if !msg.fields.is_empty() {
        // All fields are separated by ] after command
        for field in &msg.fields {
            output.push_str(DELIMITER_FIELD);
            output.push_str(field);
        }
        output.push_str(DELIMITER_FIELD);
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_access_request() {
        let msg = MessageBuilder::new(DeviceId::new(15).unwrap(), CommandCode::AccessRequest)
            .field("12345678")
            .field("10/05/2025 12:46:06")
            .field("1")
            .field("0")
            .build_string();

        assert_eq!(msg, "15+REON+000+0]12345678]10/05/2025 12:46:06]1]0]");
    }

    #[test]
    fn test_build_grant_response() {
        let msg = MessageBuilder::new(DeviceId::new(15).unwrap(), CommandCode::GrantExit)
            .field("5")
            .field("Acesso liberado")
            .build_string();

        assert_eq!(msg, "15+REON+00+6]5]Acesso liberado]");
    }

    #[test]
    fn test_build_no_fields() {
        let msg =
            MessageBuilder::new(DeviceId::new(1).unwrap(), CommandCode::QueryStatus).build_string();

        assert_eq!(msg, "01+REON+RQ");
    }

    #[test]
    fn test_build_with_fields_vec() {
        let fields = vec!["field1".to_string(), "field2".to_string()];
        let msg = MessageBuilder::new(DeviceId::new(10).unwrap(), CommandCode::SendCards)
            .fields(fields)
            .build_string();

        assert_eq!(msg, "10+REON+ECAR]field1]field2]");
    }

    #[test]
    fn test_build_message_object() {
        let msg = MessageBuilder::new(DeviceId::new(5).unwrap(), CommandCode::AccessRequest)
            .field("12345678")
            .build();

        assert_eq!(msg.device_id.as_u8(), 5);
        assert_eq!(msg.command, CommandCode::AccessRequest);
        assert_eq!(msg.field_count(), 1);
        assert_eq!(msg.field(0), Some("12345678"));
    }
}
