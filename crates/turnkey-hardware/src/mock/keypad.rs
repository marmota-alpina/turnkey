//! Mock keypad implementation for testing and development.
//!
//! This module provides a simulated keypad device that can be controlled
//! programmatically for testing without requiring physical hardware.

use crate::{
    Result,
    traits::{KeypadDevice, KeypadInput},
    types::DeviceInfo,
};
use tokio::sync::mpsc;

/// Mock keypad device for testing and development.
///
/// This device simulates a numeric keypad by receiving input through
/// an internal channel. Tests and applications can send keypad input
/// programmatically using a `MockKeypadHandle`.
///
/// # Examples
///
/// ```
/// use turnkey_hardware::mock::MockKeypad;
/// use turnkey_hardware::traits::{KeypadDevice, KeypadInput};
///
/// #[tokio::main]
/// async fn main() -> turnkey_hardware::Result<()> {
///     let (mut keypad, handle) = MockKeypad::new();
///
///     // Simulate user input
///     tokio::spawn(async move {
///         handle.send_input(KeypadInput::Digit(1)).await.unwrap();
///         handle.send_input(KeypadInput::Digit(2)).await.unwrap();
///         handle.send_input(KeypadInput::Enter).await.unwrap();
///     });
///
///     // Read simulated input
///     let input1 = keypad.read_input().await?;
///     let input2 = keypad.read_input().await?;
///     let input3 = keypad.read_input().await?;
///
///     assert_eq!(input1, KeypadInput::Digit(1));
///     assert_eq!(input2, KeypadInput::Digit(2));
///     assert_eq!(input3, KeypadInput::Enter);
///
///     Ok(())
/// }
/// ```
#[derive(Debug)]
pub struct MockKeypad {
    /// Channel receiver for simulated input
    input_rx: mpsc::Receiver<KeypadInput>,

    /// Device name
    name: String,

    /// Backlight state
    backlight_enabled: bool,
}

impl MockKeypad {
    /// Create a new mock keypad with the default name.
    ///
    /// Returns a tuple of (MockKeypad, MockKeypadHandle) where the handle
    /// can be used to simulate input to the keypad.
    ///
    /// # Examples
    ///
    /// ```
    /// use turnkey_hardware::mock::MockKeypad;
    ///
    /// let (keypad, handle) = MockKeypad::new();
    /// ```
    pub fn new() -> (Self, MockKeypadHandle) {
        Self::with_name("Mock Keypad".to_string())
    }

    /// Create a new mock keypad with a custom name.
    ///
    /// # Examples
    ///
    /// ```
    /// use turnkey_hardware::mock::MockKeypad;
    ///
    /// let (keypad, handle) = MockKeypad::with_name("Test Keypad 1".to_string());
    /// ```
    pub fn with_name(name: String) -> (Self, MockKeypadHandle) {
        let (input_tx, input_rx) = mpsc::channel(32);

        let keypad = Self {
            input_rx,
            name: name.clone(),
            backlight_enabled: false,
        };

        let handle = MockKeypadHandle { input_tx, name };

        (keypad, handle)
    }

    /// Check if the backlight is currently enabled.
    ///
    /// This is useful for testing backlight control.
    pub fn is_backlight_enabled(&self) -> bool {
        self.backlight_enabled
    }
}

impl Default for MockKeypad {
    fn default() -> Self {
        Self::new().0
    }
}

impl KeypadDevice for MockKeypad {
    async fn read_input(&mut self) -> Result<KeypadInput> {
        self.input_rx
            .recv()
            .await
            .ok_or_else(|| crate::HardwareError::disconnected("Keypad input channel closed"))
    }

    async fn set_backlight(&mut self, enabled: bool) -> Result<()> {
        self.backlight_enabled = enabled;
        Ok(())
    }

    async fn beep(&mut self, _duration_ms: u16) -> Result<()> {
        // Mock implementation does nothing
        Ok(())
    }

    async fn get_info(&self) -> Result<DeviceInfo> {
        Ok(DeviceInfo::new(self.name.clone(), "Mock Keypad v1.0").with_firmware_version("1.0.0"))
    }
}

/// Handle for controlling a mock keypad.
///
/// This handle allows programmatic control of the mock keypad by sending
/// input events. It can be cloned and shared across tasks.
///
/// # Examples
///
/// ```
/// use turnkey_hardware::mock::MockKeypad;
/// use turnkey_hardware::traits::KeypadInput;
///
/// #[tokio::main]
/// async fn main() -> turnkey_hardware::Result<()> {
///     let (_keypad, handle) = MockKeypad::new();
///
///     // Simulate a complete PIN entry
///     handle.send_input(KeypadInput::Digit(1)).await?;
///     handle.send_input(KeypadInput::Digit(2)).await?;
///     handle.send_input(KeypadInput::Digit(3)).await?;
///     handle.send_input(KeypadInput::Digit(4)).await?;
///     handle.send_input(KeypadInput::Enter).await?;
///
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone)]
pub struct MockKeypadHandle {
    /// Channel sender for simulated input
    input_tx: mpsc::Sender<KeypadInput>,

    /// Device name
    name: String,
}

impl MockKeypadHandle {
    /// Send an input event to the mock keypad.
    ///
    /// # Errors
    ///
    /// Returns an error if the keypad has been dropped and the channel is closed.
    ///
    /// # Examples
    ///
    /// ```
    /// use turnkey_hardware::mock::MockKeypad;
    /// use turnkey_hardware::traits::KeypadInput;
    ///
    /// #[tokio::main]
    /// async fn main() -> turnkey_hardware::Result<()> {
    ///     let (_keypad, handle) = MockKeypad::new();
    ///
    ///     handle.send_input(KeypadInput::Digit(5)).await?;
    ///     handle.send_input(KeypadInput::Enter).await?;
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn send_input(&self, input: KeypadInput) -> Result<()> {
        self.input_tx
            .send(input)
            .await
            .map_err(|_| crate::HardwareError::disconnected("Keypad input channel closed"))
    }

    /// Send a sequence of digit inputs.
    ///
    /// This is a convenience method for sending multiple digits at once.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Any digit is greater than 9
    /// - The keypad has been dropped and the channel is closed
    ///
    /// # Examples
    ///
    /// ```
    /// use turnkey_hardware::mock::MockKeypad;
    ///
    /// #[tokio::main]
    /// async fn main() -> turnkey_hardware::Result<()> {
    ///     let (_keypad, handle) = MockKeypad::new();
    ///
    ///     // Send PIN code "1234"
    ///     handle.send_digits(&[1, 2, 3, 4]).await?;
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn send_digits(&self, digits: &[u8]) -> Result<()> {
        for &digit in digits {
            let input = KeypadInput::digit(digit)?;
            self.send_input(input).await?;
        }
        Ok(())
    }

    /// Send a complete PIN code followed by Enter.
    ///
    /// This is a convenience method for common test scenarios.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Any digit in the PIN is greater than 9
    /// - The keypad has been dropped and the channel is closed
    ///
    /// # Examples
    ///
    /// ```
    /// use turnkey_hardware::mock::MockKeypad;
    ///
    /// #[tokio::main]
    /// async fn main() -> turnkey_hardware::Result<()> {
    ///     let (_keypad, handle) = MockKeypad::new();
    ///
    ///     // Simulate complete PIN entry
    ///     handle.send_pin(&[1, 2, 3, 4]).await?;
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn send_pin(&self, digits: &[u8]) -> Result<()> {
        self.send_digits(digits).await?;
        self.send_input(KeypadInput::Enter).await?;
        Ok(())
    }

    /// Get the device name.
    pub fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_keypad_basic_input() {
        let (mut keypad, handle) = MockKeypad::new();

        handle.send_input(KeypadInput::Digit(5)).await.unwrap();

        let input = keypad.read_input().await.unwrap();
        assert_eq!(input, KeypadInput::Digit(5));
    }

    #[tokio::test]
    async fn test_mock_keypad_sequence() {
        let (mut keypad, handle) = MockKeypad::new();

        tokio::spawn(async move {
            handle.send_input(KeypadInput::Digit(1)).await.unwrap();
            handle.send_input(KeypadInput::Digit(2)).await.unwrap();
            handle.send_input(KeypadInput::Enter).await.unwrap();
        });

        let input1 = keypad.read_input().await.unwrap();
        let input2 = keypad.read_input().await.unwrap();
        let input3 = keypad.read_input().await.unwrap();

        assert_eq!(input1, KeypadInput::Digit(1));
        assert_eq!(input2, KeypadInput::Digit(2));
        assert_eq!(input3, KeypadInput::Enter);
    }

    #[tokio::test]
    async fn test_mock_keypad_backlight() {
        let (mut keypad, _handle) = MockKeypad::new();

        assert!(!keypad.is_backlight_enabled());

        keypad.set_backlight(true).await.unwrap();
        assert!(keypad.is_backlight_enabled());

        keypad.set_backlight(false).await.unwrap();
        assert!(!keypad.is_backlight_enabled());
    }

    #[tokio::test]
    async fn test_mock_keypad_beep() {
        let (mut keypad, _handle) = MockKeypad::new();

        // Should not error
        keypad.beep(100).await.unwrap();
        keypad.beep(500).await.unwrap();
    }

    #[tokio::test]
    async fn test_mock_keypad_get_info() {
        let (keypad, _handle) = MockKeypad::with_name("Test Keypad".to_string());

        let info = keypad.get_info().await.unwrap();
        assert_eq!(info.name, "Test Keypad");
        assert_eq!(info.model, "Mock Keypad v1.0");
        assert_eq!(info.firmware_version, Some("1.0.0".to_string()));
    }

    #[tokio::test]
    async fn test_mock_keypad_handle_send_digits() {
        let (mut keypad, handle) = MockKeypad::new();

        tokio::spawn(async move {
            handle.send_digits(&[1, 2, 3, 4]).await.unwrap();
        });

        for expected in [1, 2, 3, 4] {
            let input = keypad.read_input().await.unwrap();
            assert_eq!(input, KeypadInput::Digit(expected));
        }
    }

    #[tokio::test]
    async fn test_mock_keypad_handle_send_pin() {
        let (mut keypad, handle) = MockKeypad::new();

        tokio::spawn(async move {
            handle.send_pin(&[9, 8, 7, 6]).await.unwrap();
        });

        for expected in [9, 8, 7, 6] {
            let input = keypad.read_input().await.unwrap();
            assert_eq!(input, KeypadInput::Digit(expected));
        }

        let input = keypad.read_input().await.unwrap();
        assert_eq!(input, KeypadInput::Enter);
    }

    #[tokio::test]
    async fn test_mock_keypad_handle_clone() {
        let (mut keypad, handle) = MockKeypad::new();

        let handle_clone = handle.clone();

        tokio::spawn(async move {
            handle.send_input(KeypadInput::Digit(1)).await.unwrap();
        });

        tokio::spawn(async move {
            handle_clone
                .send_input(KeypadInput::Digit(2))
                .await
                .unwrap();
        });

        let input1 = keypad.read_input().await.unwrap();
        let input2 = keypad.read_input().await.unwrap();

        // Order is not guaranteed but both should be received
        assert!(matches!(
            input1,
            KeypadInput::Digit(1) | KeypadInput::Digit(2)
        ));
        assert!(matches!(
            input2,
            KeypadInput::Digit(1) | KeypadInput::Digit(2)
        ));
    }

    #[tokio::test]
    async fn test_mock_keypad_closed_channel() {
        let (mut keypad, handle) = MockKeypad::new();

        // Drop the handle, closing the channel
        drop(handle);

        // Reading should return error
        let result = keypad.read_input().await;
        assert!(result.is_err());
    }
}
