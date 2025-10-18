use crate::commands::CommandCode;
use serde::{Deserialize, Serialize};
use turnkey_core::{DeviceId, Error, Result};

/// Parsed Henry protocol message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub device_id: DeviceId,
    pub command: CommandCode,
    pub fields: Vec<String>,
}

impl Message {
    pub fn new(device_id: DeviceId, command: CommandCode, fields: Vec<String>) -> Self {
        Message {
            device_id,
            command,
            fields,
        }
    }

    /// Get field by index
    pub fn field(&self, index: usize) -> Option<&str> {
        self.fields.get(index).map(|s| s.as_str())
    }

    /// Get required field or error
    pub fn required_field(&self, index: usize, name: &str) -> Result<&str> {
        self.field(index)
            .ok_or_else(|| Error::MissingField(name.to_string()))
    }

    /// Number of fields in the message
    pub fn field_count(&self) -> usize {
        self.fields.len()
    }
}

/// Message type enum for pattern matching
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    AccessRequest,
    AccessResponse,
    WaitingForRotation,
    RotationCompleted,
    RotationTimeout,
    Configuration,
    StatusQuery,
    Other,
}

impl Message {
    pub fn message_type(&self) -> MessageType {
        match self.command {
            CommandCode::AccessRequest => MessageType::AccessRequest,
            CommandCode::GrantEntry | CommandCode::GrantExit | CommandCode::GrantBoth | CommandCode::DenyAccess => {
                MessageType::AccessResponse
            }
            CommandCode::WaitingRotation => MessageType::WaitingForRotation,
            CommandCode::RotationCompleted => MessageType::RotationCompleted,
            CommandCode::RotationTimeout => MessageType::RotationTimeout,
            CommandCode::SendConfig | CommandCode::ReceiveConfig => MessageType::Configuration,
            CommandCode::QueryStatus => MessageType::StatusQuery,
            _ => MessageType::Other,
        }
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
        );

        assert_eq!(msg.device_id.as_u8(), 15);
        assert_eq!(msg.command, CommandCode::AccessRequest);
        assert_eq!(msg.field_count(), 2);
        assert_eq!(msg.field(0), Some("12345678"));
    }

    #[test]
    fn test_required_field() {
        let device_id = DeviceId::new(1).unwrap();
        let msg = Message::new(
            device_id,
            CommandCode::AccessRequest,
            vec!["12345678".to_string()],
        );

        assert_eq!(msg.required_field(0, "card").unwrap(), "12345678");
        assert!(msg.required_field(1, "timestamp").is_err());
    }

    #[test]
    fn test_message_type() {
        let device_id = DeviceId::new(1).unwrap();

        let msg = Message::new(device_id, CommandCode::AccessRequest, vec![]);
        assert_eq!(msg.message_type(), MessageType::AccessRequest);

        let msg = Message::new(device_id, CommandCode::GrantExit, vec![]);
        assert_eq!(msg.message_type(), MessageType::AccessResponse);

        let msg = Message::new(device_id, CommandCode::WaitingRotation, vec![]);
        assert_eq!(msg.message_type(), MessageType::WaitingForRotation);
    }
}
