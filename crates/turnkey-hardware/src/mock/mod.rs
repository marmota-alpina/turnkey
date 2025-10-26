//! Mock device implementations for testing and development.
//!
//! This module provides simulated device implementations that can be controlled
//! programmatically without requiring physical hardware.

pub mod biometric;
pub mod keypad;
pub mod rfid;

// Re-export commonly used types
pub use biometric::{MockBiometric, MockBiometricHandle};
pub use keypad::{MockKeypad, MockKeypadHandle};
pub use rfid::{MockRfid, MockRfidHandle};
