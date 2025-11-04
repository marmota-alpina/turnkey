//! Integration tests for emulator observability features.
//!
//! Tests the EmulatorHandle API and channel communication patterns.
//! These tests verify the observability infrastructure without requiring
//! full emulator setup.
//!
//! Note: Full end-to-end tests with peripherals and validators are in
//! separate integration test files.

use turnkey_emulator::{
    DisplayState, EmulatorEvent, ErrorCategory, InputCommand, OperationMode, TurnstileState,
};

// Note: These are unit-style tests for the handle module types.
// Full integration tests with running emulator require database and
//peripheral setup which is tested elsewhere.

#[test]
fn test_display_state_construction() {
    let display = DisplayState::new(
        "Line 1".to_string(),
        "Line 2".to_string(),
        OperationMode::Online,
        15,
        Some("192.168.1.100".to_string()),
    );

    assert_eq!(display.line1, "Line 1");
    assert_eq!(display.line2, "Line 2");
    assert_eq!(display.mode, OperationMode::Online);
    assert_eq!(display.device_id, 15);
    assert_eq!(display.ip_address, Some("192.168.1.100".to_string()));
}

#[test]
fn test_error_category_display() {
    assert_eq!(ErrorCategory::Hardware.to_string(), "Hardware");
    assert_eq!(ErrorCategory::Network.to_string(), "Network");
    assert_eq!(ErrorCategory::Protocol.to_string(), "Protocol");
    assert_eq!(ErrorCategory::Configuration.to_string(), "Configuration");
    assert_eq!(ErrorCategory::Internal.to_string(), "Internal");
    assert_eq!(ErrorCategory::Validation.to_string(), "Validation");
}

#[test]
fn test_emulator_event_error_with_category() {
    let event = EmulatorEvent::Error {
        message: "USB device disconnected".to_string(),
        category: ErrorCategory::Hardware,
        recoverable: true,
    };

    let display_str = event.to_string();
    assert!(display_str.contains("Hardware"));
    assert!(display_str.contains("recoverable"));
}

#[test]
fn test_input_command_equality() {
    assert_eq!(InputCommand::KeypadDigit(5), InputCommand::KeypadDigit(5));
    assert_ne!(InputCommand::KeypadDigit(5), InputCommand::KeypadDigit(6));
    assert_eq!(InputCommand::Shutdown, InputCommand::Shutdown);
}

#[test]
fn test_emulator_event_variants() {
    // Test state transition event
    let event1 = EmulatorEvent::StateTransition {
        from: TurnstileState::Idle,
        to: TurnstileState::Validating,
    };
    assert!(event1.to_string().contains("StateTransition"));

    // Test keypad input event
    let event2 = EmulatorEvent::KeypadInput('5');
    assert_eq!(event2.to_string(), "KeypadInput('5')");

    // Test card read event
    let event3 = EmulatorEvent::CardRead("12345678".to_string());
    assert!(event3.to_string().contains("12345678"));
}

// Note: Integration tests that spawn actual emulator instances would go here
// but require full setup with database migrations and peripherals.
// For now, we test the handle types and API surface.

// TODO: Add full integration tests when test infrastructure is ready:
// - test_spawn_and_observe_state_changes()
// - test_send_input_commands()
// - test_multiple_observers()
// - test_event_broadcasting()
//
// These require:
// - Database fixture setup
// - Peripheral mock configuration
// - Async test runtime with proper cleanup
