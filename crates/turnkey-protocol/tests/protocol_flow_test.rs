//! Integration tests for end-to-end access control flow.
//!
//! This module tests the complete access control flow from request to completion:
//! 1. Access request → validation → grant/deny
//! 2. Waiting for rotation → rotation complete/timeout
//!
//! Tests cover both ONLINE and OFFLINE modes as specified in issue #9.

mod common;

use turnkey_core::{AccessDirection, ReaderType};
use turnkey_protocol::{
    CommandCode,
    commands::{
        access::{AccessDecision, AccessRequest},
        turnstile::{TurnstileState, TurnstileStatus},
    },
};

// ============================================================================
// Test Data Constants
// ============================================================================

/// Common test data used across multiple tests
mod test_data {
    /// Standard test device ID
    pub const TEST_DEVICE_ID: u8 = 15;

    /// Valid card numbers for testing
    pub const VALID_CARD_1: &str = "12345678";

    /// Invalid card number for denial scenarios
    pub const INVALID_CARD: &str = "99999999";

    /// Biometric identifier for biometric reader tests
    pub const BIO_ID: &str = "BIO001234567";

    /// Minimum valid card length (3 characters)
    pub const MIN_CARD: &str = "123";

    /// Maximum valid card length (20 characters)
    pub const MAX_CARD: &str = "12345678901234567890";

    /// Standard grant timeout in seconds
    pub const GRANT_TIMEOUT_SECS: u8 = 5;

    /// Permanent display timeout (0 = until next action)
    pub const DENY_TIMEOUT_SECS: u8 = 0;

    /// Standard grant message (Portuguese)
    pub const MSG_ACCESS_GRANTED: &str = "Acesso liberado";

    /// Standard denial message (Portuguese)
    pub const MSG_ACCESS_DENIED: &str = "Acesso negado";

    /// Biometric success message
    pub const MSG_BIO_OK: &str = "Biometria OK";
}

// ============================================================================
// ONLINE Mode Tests - Successful Access Flow
// ============================================================================

#[test]
fn test_complete_access_flow_online_grant_exit() {
    use test_data::*;

    // Complete exit flow: request → grant → waiting → completed
    common::assert_access_flow_grant(
        TEST_DEVICE_ID,
        VALID_CARD_1,
        AccessDirection::Exit,
        ReaderType::Rfid,
        GRANT_TIMEOUT_SECS,
        MSG_ACCESS_GRANTED,
    );
}

#[test]
fn test_complete_access_flow_online_grant_entry() {
    // Complete entry flow with different device ID and message
    common::assert_access_flow_grant(
        1,
        "87654321",
        AccessDirection::Entry,
        ReaderType::Rfid,
        5,
        "Bem-vindo",
    );

    // Additional validation: verify message content
    let grant_msg = common::create_access_response(1, AccessDecision::GrantEntry, 5, "Bem-vindo");
    let (decision, _, display_msg) = common::parse_access_response(&grant_msg);
    assert_eq!(decision, AccessDecision::GrantEntry);
    assert_eq!(display_msg, "Bem-vindo");
}

#[test]
fn test_access_denied_flow_online() {
    use test_data::*;

    // Complete deny flow: request → deny
    common::assert_access_flow_deny(
        TEST_DEVICE_ID,
        INVALID_CARD,
        AccessDirection::Entry,
        ReaderType::Rfid,
        DENY_TIMEOUT_SECS,
        MSG_ACCESS_DENIED,
    );

    // Additional validation: verify decision type
    let deny_msg = common::create_access_response(
        TEST_DEVICE_ID,
        AccessDecision::Deny,
        DENY_TIMEOUT_SECS,
        MSG_ACCESS_DENIED,
    );
    let (decision, _, _) = common::parse_access_response(&deny_msg);
    assert!(decision.is_deny());
}

#[test]
fn test_rotation_timeout_flow() {
    let device_id = 15;
    let card_number = "12345678";

    let request_msg = common::create_access_request(
        device_id,
        card_number,
        AccessDirection::Entry,
        ReaderType::Rfid,
    );
    common::assert_access_request(&request_msg);

    let grant_msg =
        common::create_access_response(device_id, AccessDecision::GrantEntry, 5, "Acesso liberado");
    common::assert_access_response(&grant_msg, AccessDecision::GrantEntry);

    let waiting_msg = common::create_turnstile_status(
        device_id,
        TurnstileState::WaitingRotation,
        None,
        AccessDirection::Undefined,
        ReaderType::Rfid,
    );
    common::assert_turnstile_status(&waiting_msg, TurnstileState::WaitingRotation);

    let timeout_msg = common::create_turnstile_status(
        device_id,
        TurnstileState::RotationTimeout,
        None,
        AccessDirection::Undefined,
        ReaderType::Rfid,
    );
    common::assert_turnstile_status(&timeout_msg, TurnstileState::RotationTimeout);

    let timeout_status = common::parse_turnstile_status(&timeout_msg);
    assert_eq!(timeout_status.state(), TurnstileState::RotationTimeout);
}

#[test]
fn test_state_machine_valid_transitions() {
    assert!(TurnstileState::Idle.can_transition_to(TurnstileState::Reading));
    assert!(TurnstileState::Reading.can_transition_to(TurnstileState::Validating));
    assert!(TurnstileState::Validating.can_transition_to(TurnstileState::Granted));
    assert!(TurnstileState::Granted.can_transition_to(TurnstileState::WaitingRotation));
    assert!(TurnstileState::WaitingRotation.can_transition_to(TurnstileState::RotationInProgress));
    assert!(
        TurnstileState::RotationInProgress.can_transition_to(TurnstileState::RotationCompleted)
    );
    assert!(TurnstileState::RotationCompleted.can_transition_to(TurnstileState::Idle));
    assert!(TurnstileState::Validating.can_transition_to(TurnstileState::Denied));
    assert!(TurnstileState::Denied.can_transition_to(TurnstileState::Idle));
    assert!(TurnstileState::WaitingRotation.can_transition_to(TurnstileState::RotationTimeout));
    assert!(TurnstileState::RotationTimeout.can_transition_to(TurnstileState::Idle));
}

#[test]
fn test_state_machine_invalid_transitions() {
    assert!(!TurnstileState::Idle.can_transition_to(TurnstileState::Granted));
    assert!(!TurnstileState::Reading.can_transition_to(TurnstileState::WaitingRotation));
    assert!(!TurnstileState::Granted.can_transition_to(TurnstileState::RotationCompleted));
}

#[test]
fn test_access_request_message_format() {
    let device_id = 15;
    let card_number = "00000000000011912322";
    let msg = common::create_access_request(
        device_id,
        card_number,
        AccessDirection::Entry,
        ReaderType::Rfid,
    );

    assert_eq!(msg.command, CommandCode::AccessRequest);
    assert_eq!(msg.field_count(), AccessRequest::REQUIRED_FIELD_COUNT);
    assert_eq!(msg.field(0), Some(card_number));
}

#[test]
fn test_turnstile_status_message_format() {
    let device_id = 15;

    let waiting_msg = common::create_turnstile_status(
        device_id,
        TurnstileState::WaitingRotation,
        None,
        AccessDirection::Undefined,
        ReaderType::Rfid,
    );

    assert_eq!(waiting_msg.command, CommandCode::WaitingRotation);
    assert_eq!(
        waiting_msg.field_count(),
        TurnstileStatus::REQUIRED_FIELD_COUNT
    );
    assert_eq!(waiting_msg.field(0), Some(""));
}

#[test]
fn test_grant_timeout_values() {
    let device_id = 15;

    let msg1 = common::create_access_response(device_id, AccessDecision::GrantEntry, 5, "Test");
    let (_, timeout1, _) = common::parse_access_response(&msg1);
    assert_eq!(timeout1, 5);

    let msg2 = common::create_access_response(device_id, AccessDecision::Deny, 0, "Test");
    let (_, timeout2, _) = common::parse_access_response(&msg2);
    assert_eq!(timeout2, 0);

    let msg3 = common::create_access_response(device_id, AccessDecision::GrantBoth, 10, "Test");
    let (_, timeout3, _) = common::parse_access_response(&msg3);
    assert_eq!(timeout3, 10);
}

// ============================================================================
// Server Timeout and Fallback Tests
// ============================================================================

#[test]
fn test_server_timeout_no_response() {
    // Scenario: Server does not respond within timeout period (3000ms default)
    //
    // Expected flow:
    // 1. Turnstile sends access request
    // 2. No response received within timeout
    // 3. System should log timeout event
    // 4. System should switch to offline mode (if configured)
    //
    // NOTE: This test verifies the message structure for timeout scenarios.
    // Actual timeout behavior (async waiting, timer management) should be
    // tested at the emulator/network layer with tokio::time::timeout.

    let device_id = 15;
    let card_number = "12345678";

    // Step 1: Create access request that would be sent
    let request_msg = common::create_access_request(
        device_id,
        card_number,
        AccessDirection::Entry,
        ReaderType::Rfid,
    );

    common::assert_access_request(&request_msg);
    assert_eq!(request_msg.device_id.as_u8(), device_id);

    // Step 2: Verify request format is correct for timeout detection
    let request = common::parse_access_request(&request_msg);
    assert_eq!(request.card_number(), card_number);
    assert_eq!(request.direction(), AccessDirection::Entry);

    // Step 3: After timeout, system should either:
    // a) Retry the request (if configured)
    // b) Switch to offline validation (if configured)
    // c) Deny access and return to idle
    //
    // The specific behavior depends on configuration settings:
    // - TIMEOUT_ON: Online validation timeout (default 3000ms)
    // - MODE_AUTOMATIC: Online with offline fallback
    // - MODE_SMART: Optimized retry logic
}

#[test]
fn test_online_fallback_to_offline_after_timeout() {
    // Scenario: Online validation times out, system falls back to offline validation
    //
    // Expected flow:
    // 1. Send access request to server
    // 2. Timeout occurs (no response)
    // 3. Switch to offline mode indicator
    // 4. Query local database for card
    // 5. Grant or deny based on local validation
    //
    // This test verifies the message flow for fallback scenarios.
    // The actual timeout detection and mode switching logic is tested
    // at the emulator level where async timeouts can be properly simulated.

    let device_id = 20;
    let card_number = "OFFLINE001";

    // Access request sent (would timeout in real scenario)
    let request_msg = common::create_access_request(
        device_id,
        card_number,
        AccessDirection::Entry,
        ReaderType::Rfid,
    );

    common::assert_access_request(&request_msg);

    // After timeout and local validation, system grants access
    // based on local database (assuming card exists and is valid)
    let grant_msg = common::create_access_response(
        device_id,
        AccessDecision::GrantEntry,
        5,
        "MODO OFFLINE - Acesso liberado",
    );

    common::assert_access_response(&grant_msg, AccessDecision::GrantEntry);
    let (decision, _, display_msg) = common::parse_access_response(&grant_msg);
    assert_eq!(decision, AccessDecision::GrantEntry);
    assert!(display_msg.contains("OFFLINE"));
}

// ============================================================================
// Concurrent Access Request Tests
// ============================================================================

#[test]
fn test_concurrent_access_requests_second_rejected() {
    // Scenario: Second card read while first request is still being validated
    //
    // Expected behavior:
    // - First request enters validation state
    // - Second request arrives before first completes
    // - Second request should be rejected or queued (depending on config)
    // - Device should not process second request until first completes
    //
    // This test verifies the message structure for concurrent scenarios.
    // The actual concurrent request handling and state locking is tested
    // at the emulator level with proper async synchronization primitives.

    let device_id = 15;

    // First access request
    let request1 = common::create_access_request(
        device_id,
        "12345678",
        AccessDirection::Entry,
        ReaderType::Rfid,
    );

    common::assert_access_request(&request1);
    let parsed1 = common::parse_access_request(&request1);
    assert_eq!(parsed1.card_number(), "12345678");

    // Second access request arrives immediately (concurrent)
    let request2 = common::create_access_request(
        device_id,
        "87654321",
        AccessDirection::Entry,
        ReaderType::Rfid,
    );

    common::assert_access_request(&request2);
    let parsed2 = common::parse_access_request(&request2);
    assert_eq!(parsed2.card_number(), "87654321");

    // Verify both requests are properly formatted
    // The emulator should either:
    // 1. Queue the second request (process after first completes)
    // 2. Reject the second request (return to idle, display "Aguarde")
    // 3. Deny the second request (00+30 with "Aguarde validacao anterior")

    // Expected response for second request (rejection)
    let deny_msg = common::create_access_response(
        device_id,
        AccessDecision::Deny,
        3,
        "Aguarde validacao anterior",
    );

    let (decision, timeout, msg) = common::parse_access_response(&deny_msg);
    assert_eq!(decision, AccessDecision::Deny);
    assert_eq!(timeout, 3);
    assert!(msg.contains("Aguarde"));
}

#[test]
fn test_access_request_during_rotation() {
    // Scenario: Card presented while turnstile is physically rotating
    //
    // Expected behavior:
    // - First user granted access, turnstile in rotation state
    // - Second card read occurs during rotation
    // - Second request must be rejected (cannot grant during rotation)
    // - State machine prevents invalid state transitions
    //
    // Protocol flow:
    // 1. First user: Request → Grant → WaitingRotation → RotationInProgress
    // 2. Second user: Request arrives during RotationInProgress
    // 3. Second request: Immediate denial or queued for after rotation

    let device_id = 15;

    // First user granted, turnstile waiting for rotation
    let waiting_msg = common::create_turnstile_status(
        device_id,
        TurnstileState::WaitingRotation,
        Some("12345678"),
        AccessDirection::Entry,
        ReaderType::Rfid,
    );

    common::assert_turnstile_status(&waiting_msg, TurnstileState::WaitingRotation);
    let waiting_status = common::parse_turnstile_status(&waiting_msg);
    assert_eq!(waiting_status.state(), TurnstileState::WaitingRotation);

    // Second user presents card during rotation
    let request2 = common::create_access_request(
        device_id,
        "87654321",
        AccessDirection::Entry,
        ReaderType::Rfid,
    );

    common::assert_access_request(&request2);

    // Expected response: Denial with message indicating device is busy
    let deny_msg = common::create_access_response(
        device_id,
        AccessDecision::Deny,
        0,
        "Catraca em uso - Aguarde",
    );

    let (decision, _, msg) = common::parse_access_response(&deny_msg);
    assert_eq!(decision, AccessDecision::Deny);
    assert!(msg.contains("Aguarde") || msg.contains("uso"));

    // Verify state machine would prevent invalid transitions
    // Cannot transition from WaitingRotation directly to another access request
    assert!(!TurnstileState::WaitingRotation.can_transition_to(TurnstileState::Validating));
    assert!(!TurnstileState::RotationInProgress.can_transition_to(TurnstileState::Reading));
}

#[test]
fn test_rapid_sequential_access_requests() {
    // Scenario: Multiple cards read in rapid succession (no concurrency,
    // but each arrives immediately after previous completes)
    //
    // Expected behavior:
    // - Each request processed in order
    // - No state corruption between requests
    // - Each request gets proper validation
    //
    // This verifies message format consistency across multiple sequential requests

    let device_id = 10;
    let cards = ["CARD001", "CARD002", "CARD003"];

    for (i, card) in cards.iter().enumerate() {
        // Each request should be properly formatted
        let request = common::create_access_request(
            device_id,
            card,
            AccessDirection::Entry,
            ReaderType::Rfid,
        );

        common::assert_access_request(&request);
        let parsed = common::parse_access_request(&request);
        assert_eq!(parsed.card_number(), *card);

        // Each should get a valid response (grant or deny)
        let response = if i % 2 == 0 {
            common::create_access_response(
                device_id,
                AccessDecision::GrantEntry,
                5,
                "Acesso liberado",
            )
        } else {
            common::create_access_response(device_id, AccessDecision::Deny, 0, "Acesso negado")
        };

        // Verify response format
        let (decision, _, _) = common::parse_access_response(&response);
        assert!(decision.is_grant() || decision.is_deny());
    }
}

// ============================================================================
// Biometric and Other Reader Tests
// ============================================================================

#[test]
fn test_biometric_reader_flow() {
    use test_data::*;

    let device_id = 5;

    // Test biometric request with complete validation
    let request_msg = common::create_access_request(
        device_id,
        BIO_ID,
        AccessDirection::Entry,
        ReaderType::Biometric,
    );

    common::assert_access_request_complete(
        &request_msg,
        device_id,
        BIO_ID,
        AccessDirection::Entry,
        ReaderType::Biometric,
    );

    // Additional validation: verify biometric type
    let request = common::parse_access_request(&request_msg);
    assert!(request.is_biometric());

    // Test response
    let grant_msg =
        common::create_access_response(device_id, AccessDecision::GrantEntry, 5, MSG_BIO_OK);
    common::assert_access_response_complete(
        &grant_msg,
        device_id,
        AccessDecision::GrantEntry,
        5,
        MSG_BIO_OK,
    );
}

#[test]
fn test_minimum_card_length() {
    use test_data::*;

    let device_id = 99;

    // Test minimum card length (3 chars) with complete validation
    let msg = common::create_access_request(
        device_id,
        MIN_CARD,
        AccessDirection::Entry,
        ReaderType::Rfid,
    );

    common::assert_access_request_complete(
        &msg,
        device_id,
        MIN_CARD,
        AccessDirection::Entry,
        ReaderType::Rfid,
    );

    // Additional validation: verify length
    let request = common::parse_access_request(&msg);
    assert_eq!(request.card_number().len(), 3);
}

#[test]
fn test_maximum_card_length() {
    use test_data::*;

    let device_id = 99;

    // Test maximum card length (20 chars) with complete validation
    let msg = common::create_access_request(
        device_id,
        MAX_CARD,
        AccessDirection::Entry,
        ReaderType::Rfid,
    );

    common::assert_access_request_complete(
        &msg,
        device_id,
        MAX_CARD,
        AccessDirection::Entry,
        ReaderType::Rfid,
    );

    // Additional validation: verify length
    let request = common::parse_access_request(&msg);
    assert_eq!(request.card_number().len(), 20);
}

#[test]
fn test_alphanumeric_card_numbers() {
    let test_cards = vec![
        "ABC123",
        "12345XYZ",
        "FC0042ABC123",
        "MIX3D4LPH4",
        "BIO001234567",
    ];

    for card in test_cards {
        let msg = common::create_access_request(15, card, AccessDirection::Entry, ReaderType::Rfid);
        let request = common::parse_access_request(&msg);
        assert_eq!(request.card_number(), card);
    }
}
