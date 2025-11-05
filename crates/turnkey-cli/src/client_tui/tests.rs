//! Integration tests for client-emulator TUI protocol handling.

#[cfg(test)]
mod integration_tests {
    use turnkey_core::{AccessDirection, DeviceId};
    use turnkey_protocol::commands::access::{AccessDecision, AccessRequest, AccessResponse};

    #[test]
    fn test_access_request_parsing() {
        // Simulate receiving an access request from turnstile
        let fields = vec![
            "1234".to_string(),
            "25/10/2025 14:30:05".to_string(),
            "1".to_string(), // Entry
            "1".to_string(), // RFID
        ];

        let request = AccessRequest::parse(&fields).unwrap();

        assert_eq!(request.card_number(), "1234");
        assert_eq!(request.direction(), AccessDirection::Entry);
    }

    #[test]
    fn test_access_response_building() {
        // Test that we can build all decision types correctly
        let grant_entry =
            AccessResponse::new(AccessDecision::GrantEntry, 5, "Acesso liberado".to_string());

        assert_eq!(grant_entry.decision(), AccessDecision::GrantEntry);
        assert_eq!(grant_entry.timeout_seconds(), 5);

        let grant_exit =
            AccessResponse::new(AccessDecision::GrantExit, 5, "Acesso liberado".to_string());

        assert_eq!(grant_exit.decision(), AccessDecision::GrantExit);

        let grant_both =
            AccessResponse::new(AccessDecision::GrantBoth, 5, "Acesso liberado".to_string());

        assert_eq!(grant_both.decision(), AccessDecision::GrantBoth);

        let deny = AccessResponse::new(AccessDecision::Deny, 0, "Acesso negado".to_string());

        assert_eq!(deny.decision(), AccessDecision::Deny);
        assert_eq!(deny.timeout_seconds(), 0);
    }

    #[test]
    fn test_command_code_mapping() {
        // Verify protocol command codes are correct
        assert_eq!(AccessDecision::GrantEntry.command_code(), "00+5");
        assert_eq!(AccessDecision::GrantExit.command_code(), "00+6");
        assert_eq!(AccessDecision::GrantBoth.command_code(), "00+1");
        assert_eq!(AccessDecision::Deny.command_code(), "00+30");
    }

    #[test]
    fn test_device_id_validation() {
        // Valid device IDs
        assert!(DeviceId::new(1).is_ok());
        assert!(DeviceId::new(50).is_ok());
        assert!(DeviceId::new(99).is_ok());

        // Invalid device IDs
        assert!(DeviceId::new(0).is_err());
        assert!(DeviceId::new(100).is_err());
    }

    #[test]
    fn test_device_id_formatting() {
        let device_id = DeviceId::new(5).unwrap();
        assert_eq!(device_id.as_u8(), 5);
        assert_eq!(format!("{}", device_id), "05");

        let device_id = DeviceId::new(15).unwrap();
        assert_eq!(device_id.as_u8(), 15);
        assert_eq!(format!("{}", device_id), "15");
    }

    #[test]
    fn test_access_decision_helpers() {
        assert!(AccessDecision::GrantEntry.is_grant());
        assert!(AccessDecision::GrantExit.is_grant());
        assert!(AccessDecision::GrantBoth.is_grant());
        assert!(!AccessDecision::Deny.is_grant());

        assert!(!AccessDecision::GrantEntry.is_deny());
        assert!(!AccessDecision::GrantExit.is_deny());
        assert!(!AccessDecision::GrantBoth.is_deny());
        assert!(AccessDecision::Deny.is_deny());
    }

    #[test]
    fn test_access_response_helpers() {
        let grant = AccessResponse::grant_entry("Acesso liberado".to_string());
        assert!(grant.is_grant());
        assert!(!grant.is_deny());

        let deny = AccessResponse::deny("Acesso negado".to_string());
        assert!(!deny.is_grant());
        assert!(deny.is_deny());
    }

    // Error scenario tests

    #[test]
    fn test_malformed_access_request_too_few_fields() {
        // Access request requires 4 fields minimum
        let fields = vec!["1234".to_string(), "25/10/2025 14:30:05".to_string()];

        let result = AccessRequest::parse(&fields);
        assert!(result.is_err(), "Should fail with too few fields");
    }

    #[test]
    fn test_malformed_access_request_invalid_direction() {
        // Direction must be 0, 1, or 2
        let fields = vec![
            "1234".to_string(),
            "25/10/2025 14:30:05".to_string(),
            "5".to_string(), // Invalid direction
            "1".to_string(),
        ];

        let result = AccessRequest::parse(&fields);
        // Should either fail or treat as Undefined
        if let Ok(request) = result {
            assert_eq!(request.direction(), AccessDirection::Undefined);
        }
    }

    #[test]
    fn test_malformed_access_request_empty_card() {
        // Empty card number should fail validation (too short per protocol)
        let fields = vec![
            "".to_string(),
            "25/10/2025 14:30:05".to_string(),
            "1".to_string(),
            "5".to_string(), // Biometric reader
        ];

        let result = AccessRequest::parse(&fields);
        assert!(
            result.is_err(),
            "Empty card number should fail (minimum 3 characters per Henry protocol)"
        );
    }

    #[test]
    fn test_protocol_delimiter_rejection_in_message() {
        use turnkey_protocol::is_protocol_delimiter;

        // Verify that all protocol delimiters are properly rejected
        let delimiters = [']', '+', '[', '{', '}'];

        for delimiter in delimiters {
            assert!(
                is_protocol_delimiter(delimiter),
                "Character '{}' should be recognized as delimiter",
                delimiter
            );
        }
    }

    #[test]
    fn test_access_response_with_protocol_delimiters() {
        use turnkey_protocol::FieldData;

        // Attempting to create FieldData with protocol delimiters should fail
        let result = FieldData::new("Message with ] delimiter".to_string());
        assert!(result.is_err(), "Should reject message with ] delimiter");

        let result = FieldData::new("Message with + delimiter".to_string());
        assert!(result.is_err(), "Should reject message with + delimiter");

        let result = FieldData::new("Message with [ delimiter".to_string());
        assert!(result.is_err(), "Should reject message with [ delimiter");
    }

    #[test]
    fn test_device_id_boundary_values() {
        // Valid boundary values
        assert!(DeviceId::new(1).is_ok(), "Device ID 1 should be valid");
        assert!(DeviceId::new(99).is_ok(), "Device ID 99 should be valid");

        // Invalid boundary values
        assert!(DeviceId::new(0).is_err(), "Device ID 0 should be invalid");
        assert!(
            DeviceId::new(100).is_err(),
            "Device ID 100 should be invalid"
        );
    }

    #[test]
    fn test_access_decision_all_variants() {
        // Ensure all decision types have correct command codes
        let decisions = [
            (AccessDecision::GrantEntry, "00+5"),
            (AccessDecision::GrantExit, "00+6"),
            (AccessDecision::GrantBoth, "00+1"),
            (AccessDecision::Deny, "00+30"),
        ];

        for (decision, expected_code) in decisions {
            assert_eq!(
                decision.command_code(),
                expected_code,
                "Command code mismatch for {:?}",
                decision
            );
        }
    }

    #[test]
    fn test_access_response_timeout_bounds() {
        // Test timeout values at boundaries
        let response = AccessResponse::new(
            AccessDecision::GrantExit,
            0, // Minimum timeout
            "Test".to_string(),
        );
        assert_eq!(response.timeout_seconds(), 0);

        let response = AccessResponse::new(
            AccessDecision::GrantExit,
            255, // Maximum u8 value
            "Test".to_string(),
        );
        assert_eq!(response.timeout_seconds(), 255);
    }

    #[test]
    fn test_access_request_with_special_characters() {
        // Card numbers can contain alphanumeric and some special chars (not delimiters)
        let fields = vec![
            "ABC-123-XYZ".to_string(),
            "25/10/2025 14:30:05".to_string(),
            "1".to_string(),
            "1".to_string(),
        ];

        let result = AccessRequest::parse(&fields);
        assert!(result.is_ok(), "Should accept card with dashes");

        if let Ok(request) = result {
            assert_eq!(request.card_number(), "ABC-123-XYZ");
        }
    }
}
