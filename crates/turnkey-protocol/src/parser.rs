use crate::{commands::CommandCode, message::Message};
use turnkey_core::{DeviceId, Error, Result, constants::*};

pub struct MessageParser;

impl MessageParser {
    /// Parse a Henry protocol message
    /// Format: ID+REON+COMMAND+FIELD1]FIELD2]...
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
        let fields = if !rest_fields.is_empty() {
            rest_fields
                .split(DELIMITER_FIELD)
                .map(|s| s.to_string())
                .filter(|s| !s.is_empty())
                .collect()
        } else {
            Vec::new()
        };

        Message::new(device_id, command, fields)
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
        // First field is empty (between two ]])
        assert_eq!(msg.field(0), Some("10/05/2025 12:46:06"));
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
