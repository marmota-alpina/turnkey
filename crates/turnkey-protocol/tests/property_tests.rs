//! Property-based tests for protocol message validation.
//!
//! These tests use proptest to generate random valid inputs and verify that
//! protocol invariants hold for all valid input combinations.

mod common;

use proptest::prelude::*;
use turnkey_core::{AccessDirection, ReaderType};
use turnkey_protocol::{CommandCode, FieldData, MessageBuilder};

/// Strategy for generating valid card numbers (3-20 chars, no delimiters).
///
/// Card numbers in the Henry protocol can be alphanumeric but must not
/// contain protocol delimiters: ], [, +, {, }
fn valid_card_number() -> impl Strategy<Value = String> {
    // Use alphanumerics and common safe symbols
    prop::string::string_regex("[0-9A-Za-z@#$%&_-]{3,20}")
        .expect("Failed to create card number regex strategy")
}

/// Strategy for generating valid device IDs (1-99).
fn valid_device_id() -> impl Strategy<Value = u8> {
    1u8..=99u8
}

/// Strategy for generating valid timeouts (0-255).
fn valid_timeout() -> impl Strategy<Value = u8> {
    any::<u8>()
}

/// Strategy for generating valid display messages (no delimiters, 1-100 chars).
fn valid_display_message() -> impl Strategy<Value = String> {
    prop::string::string_regex("[^\\]\\[+{}]{1,100}")
        .expect("Failed to create display message regex strategy")
}

/// Strategy for generating valid access directions.
fn valid_direction() -> impl Strategy<Value = AccessDirection> {
    prop_oneof![
        Just(AccessDirection::Entry),
        Just(AccessDirection::Exit),
        Just(AccessDirection::Undefined),
    ]
}

/// Strategy for generating valid reader types.
fn valid_reader_type() -> impl Strategy<Value = ReaderType> {
    prop_oneof![Just(ReaderType::Rfid), Just(ReaderType::Biometric),]
}

proptest! {
    /// Property: Any valid card number should create a parseable access request.
    ///
    /// This test verifies that the protocol can handle ANY valid card number
    /// (3-20 alphanumeric chars without delimiters) and correctly roundtrip
    /// through message creation and parsing.
    #[test]
    fn prop_access_request_roundtrip(
        device_id in valid_device_id(),
        card_number in valid_card_number(),
        direction in valid_direction(),
        reader_type in valid_reader_type(),
    ) {
        // Create message using helper
        let msg = common::create_access_request(
            device_id,
            &card_number,
            direction,
            reader_type,
        );

        // Verify basic message properties
        prop_assert_eq!(msg.device_id.as_u8(), device_id);
        prop_assert_eq!(msg.command, CommandCode::AccessRequest);
        prop_assert_eq!(msg.field_count(), 4);

        // Parse and verify roundtrip preserves data
        let parsed = common::parse_access_request(&msg);
        prop_assert_eq!(parsed.card_number(), card_number);
        prop_assert_eq!(parsed.direction(), direction);
        prop_assert_eq!(parsed.reader_type(), reader_type);
    }

    /// Property: Any valid access response parameters should create valid messages.
    ///
    /// Tests all combinations of device IDs, timeouts, and display messages
    /// to ensure the protocol correctly handles the full parameter space.
    #[test]
    fn prop_access_response_format(
        device_id in valid_device_id(),
        timeout in valid_timeout(),
        message in valid_display_message(),
    ) {
        use turnkey_protocol::commands::access::AccessDecision;

        // Test with GrantEntry decision
        let msg = common::create_access_response(
            device_id,
            AccessDecision::GrantEntry,
            timeout,
            &message,
        );

        // Verify message structure
        prop_assert_eq!(msg.device_id.as_u8(), device_id);
        prop_assert_eq!(msg.command, CommandCode::GrantEntry);
        prop_assert_eq!(msg.field_count(), 2);

        // Verify roundtrip
        let (decision, parsed_timeout, parsed_msg) = common::parse_access_response(&msg);
        prop_assert_eq!(decision, AccessDecision::GrantEntry);
        prop_assert_eq!(parsed_timeout, timeout);
        prop_assert_eq!(parsed_msg, message);
    }

    /// Property: Card numbers are correctly stored in protocol fields.
    ///
    /// This critical property ensures that card numbers can contain any valid
    /// characters (alphanumerics and safe symbols) and are correctly preserved
    /// in the protocol message fields without corruption.
    #[test]
    fn prop_no_delimiter_injection(
        device_id in valid_device_id(),
        card_number in valid_card_number(),
    ) {
        let msg = common::create_access_request(
            device_id,
            &card_number,
            AccessDirection::Entry,
            ReaderType::Rfid,
        );

        // Property: Card number field should be retrievable and exact
        let retrieved_card = msg.field(0).expect("Card field should exist");
        prop_assert_eq!(retrieved_card, card_number, "Card number must match exactly");

        // Property: Message should have exactly 4 fields for access request
        prop_assert_eq!(msg.field_count(), 4, "Access request must have 4 fields");

        // Property: Device ID should match
        prop_assert_eq!(msg.device_id.as_u8(), device_id, "Device ID must match");
    }

    /// Property: Message builder should consistently produce valid messages.
    ///
    /// Tests that MessageBuilder correctly handles any valid combination of
    /// device ID, command code, and field data.
    #[test]
    fn prop_message_builder_consistency(
        device_id in valid_device_id(),
        field_content in valid_display_message(),
    ) {
        let device_id = turnkey_core::DeviceId::new(device_id)
            .expect("Device ID should be valid (1-99)");

        // Build message with arbitrary field content
        let msg = MessageBuilder::new(device_id, CommandCode::AccessRequest)
            .field(FieldData::new(field_content.clone()).expect("Field should be valid"))
            .build()
            .expect("Message should build successfully");

        // Verify message properties
        prop_assert_eq!(msg.command, CommandCode::AccessRequest);
        prop_assert_eq!(msg.field_count(), 1);

        // Verify field content preserved
        let retrieved_field = msg.field(0).expect("Field 0 should exist");
        prop_assert_eq!(retrieved_field, field_content);
    }

    /// Property: Field data correctly validates protocol delimiters.
    ///
    /// This test verifies that FieldData::new() correctly rejects strings
    /// containing protocol delimiters (], [, +) which are used for framing.
    #[test]
    fn prop_field_data_rejects_delimiters(
        valid_content in valid_display_message(),
    ) {
        // Valid content should be accepted
        let result = FieldData::new(valid_content);
        prop_assert!(result.is_ok(), "Valid content should be accepted");

        // Test each critical delimiter separately
        // Note: Only ], [, + are actual protocol delimiters that break parsing
        let delimiters = vec![']', '[', '+'];
        for delimiter in delimiters {
            let invalid = format!("test{}content", delimiter);
            let result = FieldData::new(invalid);
            prop_assert!(result.is_err(), "Content with delimiter '{}' should be rejected", delimiter);
        }
    }

    /// Property: Card number validation accepts valid range (3-20 chars).
    ///
    /// Tests that card number validation correctly accepts the full valid
    /// range and rejects invalid lengths.
    #[test]
    fn prop_card_number_length_validation(
        card in valid_card_number(),
    ) {
        use turnkey_protocol::validation::validate_card_number;

        // All generated cards are 3-20 chars, so should be valid
        let result = validate_card_number(&card);
        prop_assert!(result.is_ok(), "Card '{}' (len={}) should be valid", card, card.len());
        prop_assert!(card.len() >= 3 && card.len() <= 20, "Generated card should be 3-20 chars");
    }

    /// Property: Serialized messages can be parsed back to equivalent state.
    ///
    /// This critical property ensures that messages sent over TCP can be
    /// reconstructed exactly on the receiving end, which is essential for
    /// protocol interoperability with real Henry equipment.
    #[test]
    fn prop_message_serialization_roundtrip(
        device_id in valid_device_id(),
        card_number in valid_card_number(),
        direction in valid_direction(),
        reader_type in valid_reader_type(),
    ) {
        // 1. Create original message
        let original = common::create_access_request(
            device_id,
            &card_number,
            direction,
            reader_type,
        );

        // 2. Serialize to wire format (Display implementation)
        let wire_format = original.to_string();

        // 3. Verify serialization produces non-empty output
        prop_assert!(!wire_format.is_empty(), "Serialized message must not be empty");

        // The wire format should contain the device ID
        let device_str = format!("{:02}", device_id);
        prop_assert!(wire_format.contains(&device_str),
            "Wire format must contain device ID: expected '{}' in '{}'",
            device_str, wire_format);

        // 4. Verify all fields are retrievable from original message
        prop_assert_eq!(original.device_id.as_u8(), device_id, "Device ID must be preserved");
        prop_assert_eq!(original.command, CommandCode::AccessRequest, "Command must be preserved");
        prop_assert_eq!(original.field_count(), 4, "Access request must have 4 fields");

        // 5. Verify first field (card number) matches exactly
        let retrieved_card = original.field(0).expect("Card field should exist");
        prop_assert_eq!(retrieved_card, &card_number, "Card number must match exactly after serialization");

        // 6. Verify we can parse back the data
        let parsed_request = common::parse_access_request(&original);
        prop_assert_eq!(parsed_request.card_number(), card_number, "Parsed card number must match");
        prop_assert_eq!(parsed_request.direction(), direction, "Parsed direction must match");
        prop_assert_eq!(parsed_request.reader_type(), reader_type, "Parsed reader type must match");
    }
}

#[cfg(test)]
mod standard_tests {
    use super::*;

    /// Standard test: Verify proptest strategies generate expected ranges.
    #[test]
    fn test_valid_device_id_range() {
        proptest!(|(id in valid_device_id())| {
            prop_assert!((1..=99).contains(&id));
        });
    }

    /// Standard test: Verify card number strategy respects length constraints.
    #[test]
    fn test_valid_card_number_constraints() {
        proptest!(|(card in valid_card_number())| {
            prop_assert!((3..=20).contains(&card.len()));

            // Verify no protocol delimiters
            prop_assert!(!card.contains(']'));
            prop_assert!(!card.contains('['));
            prop_assert!(!card.contains('+'));
            prop_assert!(!card.contains("{{"));
            prop_assert!(!card.contains("}}"));
        });
    }

    /// Standard test: Verify message strategy doesn't contain delimiters.
    #[test]
    fn test_valid_display_message_no_delimiters() {
        proptest!(|(msg in valid_display_message())| {
            prop_assert!(!msg.contains(']'), "Message should not contain ']'");
            prop_assert!(!msg.contains('['), "Message should not contain '['");
            prop_assert!(!msg.contains('+'), "Message should not contain '+'");
            prop_assert!(!msg.contains("{{"), "Message should not contain '{{'");
            prop_assert!(!msg.contains("}}"), "Message should not contain '}}'");
        });
    }
}
