//! Hardware device abstraction layer for the Turnkey access control emulator.
//!
//! This crate provides trait-based abstractions for hardware peripherals used in
//! access control systems, including keypads, RFID/NFC readers, and biometric
//! scanners. These traits enable polymorphic behavior and easy substitution between
//! mock implementations (for development and testing) and real hardware drivers.
//!
//! # Design Philosophy
//!
//! The hardware abstraction layer is designed with the following principles:
//!
//! - **Async-first**: All I/O operations are asynchronous using native `async fn`
//!   in traits (Rust 1.90 + Edition 2024 RPITIT).
//! - **Object-safe**: All traits can be used as trait objects (`Box<dyn Trait>`).
//! - **Thread-safe**: All traits require `Send + Sync` for use with Tokio.
//! - **Error-aware**: All operations return `Result<T>` with detailed error information.
//!
//! # Device Traits
//!
//! The crate defines three main device trait families:
//!
//! ## Keypad Devices
//!
//! The [`KeypadDevice`] trait represents numeric keypads for user input:
//!
//! ```no_run
//! use turnkey_hardware::traits::{KeypadDevice, KeypadInput};
//! use turnkey_hardware::error::Result;
//!
//! async fn read_code<K: KeypadDevice>(keypad: &mut K) -> Result<String> {
//!     let mut code = String::new();
//!
//!     loop {
//!         let input = keypad.read_input().await?;
//!
//!         match input {
//!             KeypadInput::Digit(d) => code.push_str(&d.to_string()),
//!             KeypadInput::Enter => break,
//!             KeypadInput::Clear => code.clear(),
//!             _ => {}
//!         }
//!     }
//!
//!     Ok(code)
//! }
//! ```
//!
//! ## RFID Readers
//!
//! The [`RfidDevice`] trait represents RFID/NFC card readers:
//!
//! ```no_run
//! use turnkey_hardware::traits::RfidDevice;
//! use turnkey_hardware::types::LedColor;
//! use turnkey_hardware::error::Result;
//!
//! async fn authenticate_card<R: RfidDevice>(reader: &mut R) -> Result<String> {
//!     let card = reader.read_card().await?;
//!     reader.set_led(LedColor::Green).await.ok();
//!     Ok(card.uid_decimal())
//! }
//! ```
//!
//! ## Biometric Scanners
//!
//! The [`BiometricDevice`] trait represents fingerprint scanners:
//!
//! ```no_run
//! use turnkey_hardware::traits::BiometricDevice;
//! use turnkey_hardware::types::LedColor;
//! use turnkey_hardware::error::Result;
//!
//! async fn verify_user<B: BiometricDevice>(scanner: &mut B, template: &[u8]) -> Result<bool> {
//!     let matched = scanner.verify_fingerprint(template).await?;
//!
//!     if matched {
//!         scanner.set_led(LedColor::Green).await.ok();
//!     } else {
//!         scanner.set_led(LedColor::Red).await.ok();
//!     }
//!
//!     Ok(matched)
//! }
//! ```
//!
//! # Error Handling
//!
//! All operations return [`Result<T>`][error::Result] which uses the
//! [`HardwareError`] error type. This provides detailed
//! context about hardware failures including disconnections, timeouts, and
//! protocol errors.
//!
//! # Thread Safety
//!
//! All traits require `Send + Sync`, making them safe to use across thread
//! boundaries. This is essential for use with the Tokio async runtime where
//! tasks may migrate between threads.
//!
//! # Mock Implementations
//!
//! While this crate only defines the trait interfaces, mock implementations
//! are available in the `turnkey-rfid`, `turnkey-keypad`, and `turnkey-biometric`
//! crates for development and testing without physical hardware.
//!
//! [`KeypadDevice`]: traits::KeypadDevice
//! [`RfidDevice`]: traits::RfidDevice
//! [`BiometricDevice`]: traits::BiometricDevice

pub mod devices;
pub mod error;
pub mod manager;
pub mod mock;
pub mod traits;
pub mod types;

// ===== Primary API Surface =====
//
// The types below are re-exported at the crate root for convenient access.
// Users can import them directly without navigating into submodules:
//
// use turnkey_hardware::{KeypadDevice, RfidDevice, BiometricDevice};
// use turnkey_hardware::{HardwareError, Result};
// use turnkey_hardware::PeripheralManager;

/// Error handling types for hardware operations.
pub use error::{HardwareError, Result};

/// Main device trait definitions and associated types.
///
/// These are the primary traits that hardware implementers should use:
/// - [`KeypadDevice`] - Numeric keypad input devices
/// - [`RfidDevice`] - RFID/NFC card readers
/// - [`BiometricDevice`] - Fingerprint scanners
pub use traits::{
    BiometricData, BiometricDevice, CardData, CardType, DEFAULT_QUALITY_THRESHOLD, KeypadDevice,
    KeypadInput, MAX_QUALITY_SCORE, MAX_UID_LENGTH, MIN_UID_LENGTH, RfidDevice,
};

/// Common hardware types (LED colors, device info, reader info).
pub use types::{DeviceInfo, LedColor, ReaderInfo};

/// Peripheral management system for coordinating multiple devices.
///
/// The [`PeripheralManager`] provides centralized device lifecycle management,
/// event handling, and statistics tracking for all connected peripherals.
pub use manager::{
    DeviceType, PeripheralConfig, PeripheralEvent, PeripheralHandle, PeripheralManager,
    PeripheralStats,
};
