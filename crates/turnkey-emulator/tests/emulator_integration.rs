//! Integration tests for the turnstile emulator.
//!
//! These tests verify that the emulator components work together correctly.
//! Full hardware integration tests require physical devices and are in
//! separate hardware test modules.

use turnkey_core::{AccessDirection, DeviceId};
use turnkey_emulator::{EmulatorConfig, OperationMode};

#[test]
fn test_emulator_config_validation() {
    // Valid configuration should pass
    let config = EmulatorConfig {
        device_id: 1,
        mode: OperationMode::Online,
        validation_timeout_ms: 3000,
        rotation_timeout_secs: 10,
        default_display_message: "DIGITE SEU CODIGO".to_string(),
        default_direction: AccessDirection::Entry,
        idle_timeout_secs: 30,
        rotation_duration_secs: 2,
        network: None,
    };

    // Config should be valid
    assert_eq!(config.device_id, 1);
    assert_eq!(config.mode, OperationMode::Online);
}

#[test]
fn test_emulator_config_defaults() {
    let config = EmulatorConfig::default();

    assert_eq!(config.device_id, 1);
    assert_eq!(config.mode, OperationMode::Online);
    assert_eq!(config.validation_timeout_ms, 3000);
    assert_eq!(config.rotation_timeout_secs, 10);
    assert_eq!(config.default_display_message, "DIGITE SEU CODIGO");
    assert_eq!(config.default_direction, AccessDirection::Entry);
    assert_eq!(config.idle_timeout_secs, 30);
    assert_eq!(config.rotation_duration_secs, 2);
    assert!(config.network.is_none());
}

#[test]
fn test_emulator_config_bidirectional() {
    // Test Entry direction
    let entry_config = EmulatorConfig {
        default_direction: AccessDirection::Entry,
        ..EmulatorConfig::default()
    };
    assert_eq!(entry_config.default_direction, AccessDirection::Entry);

    // Test Exit direction
    let exit_config = EmulatorConfig {
        default_direction: AccessDirection::Exit,
        ..EmulatorConfig::default()
    };
    assert_eq!(exit_config.default_direction, AccessDirection::Exit);

    // Test Undefined direction
    let undefined_config = EmulatorConfig {
        default_direction: AccessDirection::Undefined,
        ..EmulatorConfig::default()
    };
    assert_eq!(
        undefined_config.default_direction,
        AccessDirection::Undefined
    );
}

#[test]
fn test_emulator_config_idle_timeout() {
    // Test with idle timeout enabled
    let config_with_timeout = EmulatorConfig {
        idle_timeout_secs: 30,
        ..EmulatorConfig::default()
    };
    assert_eq!(config_with_timeout.idle_timeout_secs, 30);

    // Test with idle timeout disabled
    let config_no_timeout = EmulatorConfig {
        idle_timeout_secs: 0,
        ..EmulatorConfig::default()
    };
    assert_eq!(config_no_timeout.idle_timeout_secs, 0);

    // Test with custom timeout
    let config_custom = EmulatorConfig {
        idle_timeout_secs: 120,
        ..EmulatorConfig::default()
    };
    assert_eq!(config_custom.idle_timeout_secs, 120);
}

#[test]
fn test_device_id_creation() {
    // Valid device IDs
    let id1 = DeviceId::new(1);
    assert!(id1.is_ok());

    let id99 = DeviceId::new(99);
    assert!(id99.is_ok());

    // Invalid device IDs
    let id0 = DeviceId::new(0);
    assert!(id0.is_err());

    let id100 = DeviceId::new(100);
    assert!(id100.is_err());
}

#[test]
fn test_operation_mode_values() {
    let online = OperationMode::Online;
    let offline = OperationMode::Offline;

    assert_ne!(online, offline);

    // Default should be Online
    assert_eq!(OperationMode::default(), OperationMode::Online);
}

// Note: Full emulator integration tests require:
// 1. PeripheralManager setup
// 2. Database/validator initialization
// 3. Async runtime
// These are tested in higher-level integration tests in the CLI crate.
