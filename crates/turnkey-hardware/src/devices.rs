//! Enum wrappers for hardware device dispatch.
//!
//! This module provides enum wrappers that enable the use of native async traits
//! with concrete type dispatch, avoiding the object-safety limitations while
//! maintaining zero-cost abstractions.
//!
//! # Enum Dispatch Pattern
//!
//! The enum wrappers in this module solve a fundamental challenge: native `async fn`
//! in traits (RPITIT - Rust Edition 2024) are not object-safe, so we cannot use
//! `Box<dyn KeypadDevice>`. Instead, we use enums to provide concrete type dispatch
//! at compile time.
//!
//! This approach provides:
//! - Zero-cost abstraction (monomorphization at compile-time)
//! - Type-safe extensibility
//! - Support for feature flags (conditional compilation)
//! - Clear evolution path to plugin systems
//!
//! # Examples
//!
//! ```
//! use turnkey_hardware::devices::AnyKeypadDevice;
//! use turnkey_hardware::mock::MockKeypad;
//!
//! let (keypad, _handle) = MockKeypad::new();
//! let any_keypad = AnyKeypadDevice::Mock(keypad);
//!
//! // Can now be used polymorphically through the KeypadDevice trait
//! ```

use crate::mock::{MockBiometric, MockKeypad, MockRfid};
use crate::traits::{BiometricDevice, KeypadDevice, RfidDevice};
use crate::{BiometricData, CardData, DeviceInfo, KeypadInput, LedColor, ReaderInfo, Result};

/// Enum wrapper for keypad device dispatch.
///
/// This enum allows us to maintain the benefits of native async fn in traits
/// while providing concrete type dispatch for the PeripheralManager.
///
/// # Examples
///
/// ```
/// use turnkey_hardware::devices::AnyKeypadDevice;
/// use turnkey_hardware::traits::KeypadDevice;
/// use turnkey_hardware::mock::MockKeypad;
///
/// #[tokio::main]
/// async fn main() -> turnkey_hardware::Result<()> {
///     let (keypad, _handle) = MockKeypad::new();
///     let mut any_keypad = AnyKeypadDevice::Mock(keypad);
///
///     // Use through trait interface
///     let info = any_keypad.get_info().await?;
///     println!("Keypad: {}", info.name);
///
///     Ok(())
/// }
/// ```
#[derive(Debug)]
#[non_exhaustive]
pub enum AnyKeypadDevice {
    /// Mock keypad for development and testing.
    Mock(MockKeypad),
    // TODO: Add hardware implementations when ready
    // Planned variants:
    // - UsbHid(UsbHidKeypad) - USB HID keypad support
    // - Serial(SerialKeypad) - Serial protocol keypad
    // - Wiegand(WiegandKeypad) - Wiegand protocol keypad
    //
    // See issue #63 for hardware integration roadmap
}

impl KeypadDevice for AnyKeypadDevice {
    async fn read_input(&mut self) -> Result<KeypadInput> {
        match self {
            Self::Mock(device) => device.read_input().await,
        }
    }

    async fn set_backlight(&mut self, enabled: bool) -> Result<()> {
        match self {
            Self::Mock(device) => device.set_backlight(enabled).await,
        }
    }

    async fn beep(&mut self, duration_ms: u16) -> Result<()> {
        match self {
            Self::Mock(device) => device.beep(duration_ms).await,
        }
    }

    async fn get_info(&self) -> Result<DeviceInfo> {
        match self {
            Self::Mock(device) => device.get_info().await,
        }
    }
}

/// Enum wrapper for RFID reader device dispatch.
///
/// This enum allows us to maintain the benefits of native async fn in traits
/// while providing concrete type dispatch for the PeripheralManager.
///
/// # Examples
///
/// ```
/// use turnkey_hardware::devices::AnyRfidDevice;
/// use turnkey_hardware::traits::RfidDevice;
/// use turnkey_hardware::mock::MockRfid;
///
/// #[tokio::main]
/// async fn main() -> turnkey_hardware::Result<()> {
///     let (reader, _handle) = MockRfid::new();
///     let any_reader = AnyRfidDevice::Mock(reader);
///
///     // Use through trait interface
///     let info = any_reader.get_reader_info().await?;
///     println!("Reader: {}", info.name);
///
///     Ok(())
/// }
/// ```
#[derive(Debug)]
#[non_exhaustive]
pub enum AnyRfidDevice {
    /// Mock RFID reader for development and testing.
    Mock(MockRfid),
    // TODO: Add hardware implementations when ready
    // Planned variants:
    // - PcSc(PcScRfidReader) - PC/SC compatible readers (ACR122U, etc.)
    // - Spi(SpiRfidReader) - SPI-based readers (RC522, etc.)
    //
    // See issue #63 for hardware integration roadmap
}

impl RfidDevice for AnyRfidDevice {
    async fn read_card(&mut self) -> Result<CardData> {
        match self {
            Self::Mock(device) => device.read_card().await,
        }
    }

    async fn is_card_present(&self) -> Result<bool> {
        match self {
            Self::Mock(device) => device.is_card_present().await,
        }
    }

    async fn get_reader_info(&self) -> Result<ReaderInfo> {
        match self {
            Self::Mock(device) => device.get_reader_info().await,
        }
    }

    async fn set_led(&mut self, color: LedColor) -> Result<()> {
        match self {
            Self::Mock(device) => device.set_led(color).await,
        }
    }
}

/// Enum wrapper for biometric scanner device dispatch.
///
/// This enum allows us to maintain the benefits of native async fn in traits
/// while providing concrete type dispatch for the PeripheralManager.
///
/// # Examples
///
/// ```
/// use turnkey_hardware::devices::AnyBiometricDevice;
/// use turnkey_hardware::traits::BiometricDevice;
/// use turnkey_hardware::mock::MockBiometric;
///
/// #[tokio::main]
/// async fn main() -> turnkey_hardware::Result<()> {
///     let (scanner, _handle) = MockBiometric::new();
///     let any_scanner = AnyBiometricDevice::Mock(scanner);
///
///     // Use through trait interface
///     let info = any_scanner.get_device_info().await?;
///     println!("Scanner: {}", info.name);
///
///     Ok(())
/// }
/// ```
#[derive(Debug)]
#[non_exhaustive]
pub enum AnyBiometricDevice {
    /// Mock biometric scanner for development and testing.
    Mock(MockBiometric),
    // TODO: Add hardware implementations when ready
    // Planned variants:
    // - ControlId(ControlIdScanner) - Control iD biometric scanners
    // - DigitalPersona(DigitalPersonaScanner) - Digital Persona scanners
    //
    // See issue #63 for hardware integration roadmap
}

impl BiometricDevice for AnyBiometricDevice {
    async fn capture_fingerprint(&mut self) -> Result<BiometricData> {
        match self {
            Self::Mock(device) => device.capture_fingerprint().await,
        }
    }

    async fn verify_fingerprint(&mut self, template: &[u8]) -> Result<bool> {
        match self {
            Self::Mock(device) => device.verify_fingerprint(template).await,
        }
    }

    async fn get_device_info(&self) -> Result<DeviceInfo> {
        match self {
            Self::Mock(device) => device.get_device_info().await,
        }
    }

    async fn set_led(&mut self, color: LedColor) -> Result<()> {
        match self {
            Self::Mock(device) => device.set_led(color).await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_any_keypad_device_mock() {
        let (keypad, _handle) = crate::mock::MockKeypad::new();
        let any_keypad = AnyKeypadDevice::Mock(keypad);

        let info = any_keypad.get_info().await.unwrap();
        assert_eq!(info.name, "Mock Keypad");
    }

    #[tokio::test]
    async fn test_any_rfid_device_mock() {
        let (reader, _handle) = crate::mock::MockRfid::new();
        let any_reader = AnyRfidDevice::Mock(reader);

        let info = any_reader.get_reader_info().await.unwrap();
        assert_eq!(info.name, "Mock RFID Reader");
    }

    #[tokio::test]
    async fn test_any_biometric_device_mock() {
        let (scanner, _handle) = crate::mock::MockBiometric::new();
        let any_scanner = AnyBiometricDevice::Mock(scanner);

        let info = any_scanner.get_device_info().await.unwrap();
        assert_eq!(info.name, "Mock Biometric Scanner");
    }
}
