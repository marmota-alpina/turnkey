//! Mock biometric scanner implementation for testing and development.
//!
//! This module provides a simulated fingerprint scanner that can be controlled
//! programmatically for testing without requiring physical hardware.

use crate::{
    Result,
    traits::{BiometricData, BiometricDevice, DEFAULT_QUALITY_THRESHOLD},
    types::{DeviceInfo, LedColor},
};
use std::collections::HashMap;
use tokio::sync::mpsc;

/// Mock biometric scanner for testing and development.
///
/// This device simulates a fingerprint scanner by maintaining a database
/// of fingerprint templates that can be programmatically captured and verified.
///
/// # Examples
///
/// ```
/// use turnkey_hardware::mock::MockBiometric;
/// use turnkey_hardware::traits::BiometricDevice;
///
/// #[tokio::main]
/// async fn main() -> turnkey_hardware::Result<()> {
///     let (mut scanner, handle) = MockBiometric::new();
///
///     // Simulate fingerprint capture
///     let template = vec![1, 2, 3, 4, 5];
///     handle.queue_fingerprint(template.clone(), 75).await?;
///
///     let captured = scanner.capture_fingerprint().await?;
///     assert_eq!(captured.template, template);
///     assert_eq!(captured.quality, 75);
///
///     Ok(())
/// }
/// ```
#[derive(Debug)]
pub struct MockBiometric {
    /// Channel receiver for fingerprint events
    event_rx: mpsc::Receiver<BiometricEvent>,

    /// Device name
    name: String,

    /// Currently set LED color
    led_color: LedColor,
}

impl MockBiometric {
    /// Create a new mock biometric scanner with the default name.
    ///
    /// Returns a tuple of (MockBiometric, MockBiometricHandle) where the handle
    /// can be used to simulate fingerprint captures and verifications.
    ///
    /// # Examples
    ///
    /// ```
    /// use turnkey_hardware::mock::MockBiometric;
    ///
    /// let (scanner, handle) = MockBiometric::new();
    /// ```
    pub fn new() -> (Self, MockBiometricHandle) {
        Self::with_name("Mock Biometric Scanner".to_string())
    }

    /// Create a new mock biometric scanner with a custom name.
    ///
    /// # Examples
    ///
    /// ```
    /// use turnkey_hardware::mock::MockBiometric;
    ///
    /// let (scanner, handle) = MockBiometric::with_name("Test Scanner 1".to_string());
    /// ```
    pub fn with_name(name: String) -> (Self, MockBiometricHandle) {
        let (event_tx, event_rx) = mpsc::channel(32);

        let scanner = Self {
            event_rx,
            name: name.clone(),
            led_color: LedColor::Off,
        };

        let handle = MockBiometricHandle {
            event_tx,
            name,
            templates: HashMap::new(),
        };

        (scanner, handle)
    }

    /// Get the current LED color.
    ///
    /// This is useful for testing LED control.
    pub fn led_color(&self) -> LedColor {
        self.led_color
    }
}

impl Default for MockBiometric {
    fn default() -> Self {
        Self::new().0
    }
}

impl BiometricDevice for MockBiometric {
    async fn capture_fingerprint(&mut self) -> Result<BiometricData> {
        let event =
            self.event_rx.recv().await.ok_or_else(|| {
                crate::HardwareError::disconnected("Biometric event channel closed")
            })?;

        match event {
            BiometricEvent::FingerprintCaptured(data) => Ok(data),
        }
    }

    async fn verify_fingerprint(&mut self, template: &[u8]) -> Result<bool> {
        let captured = self.capture_fingerprint().await?;

        // Simple byte-by-byte comparison for mock implementation
        // Real implementations would use sophisticated matching algorithms
        Ok(captured.template == template)
    }

    async fn get_device_info(&self) -> Result<DeviceInfo> {
        Ok(
            DeviceInfo::new(self.name.clone(), "Mock Biometric Scanner v1.0")
                .with_firmware_version("1.0.0"),
        )
    }

    async fn set_led(&mut self, color: LedColor) -> Result<()> {
        self.led_color = color;
        Ok(())
    }
}

/// Internal event type for mock biometric scanner.
#[derive(Debug, Clone)]
enum BiometricEvent {
    FingerprintCaptured(BiometricData),
}

/// Handle for controlling a mock biometric scanner.
///
/// This handle allows programmatic control of the mock scanner by managing
/// a template database and simulating fingerprint captures.
///
/// # Examples
///
/// ```
/// use turnkey_hardware::mock::MockBiometric;
///
/// #[tokio::main]
/// async fn main() -> turnkey_hardware::Result<()> {
///     let (_scanner, mut handle) = MockBiometric::new();
///
///     // Store templates
///     let template1 = vec![1, 2, 3, 4, 5];
///     let template2 = vec![6, 7, 8, 9, 10];
///
///     handle.add_template("user1".to_string(), template1.clone()).await;
///     handle.add_template("user2".to_string(), template2.clone()).await;
///
///     // Simulate captures
///     handle.queue_fingerprint(template1, 80).await?;
///     handle.queue_fingerprint(template2, 75).await?;
///
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone)]
pub struct MockBiometricHandle {
    /// Channel sender for biometric events
    event_tx: mpsc::Sender<BiometricEvent>,

    /// Device name
    name: String,

    /// Template database (user_id -> template)
    templates: HashMap<String, Vec<u8>>,
}

impl MockBiometricHandle {
    /// Add a fingerprint template to the database.
    ///
    /// This registers a template that can be used for verification.
    ///
    /// # Examples
    ///
    /// ```
    /// use turnkey_hardware::mock::MockBiometric;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let (_scanner, mut handle) = MockBiometric::new();
    ///
    ///     let template = vec![1, 2, 3, 4, 5];
    ///     handle.add_template("user123".to_string(), template).await;
    /// }
    /// ```
    pub async fn add_template(&mut self, user_id: String, template: Vec<u8>) {
        self.templates.insert(user_id, template);
    }

    /// Queue a fingerprint for capture.
    ///
    /// This simulates placing a finger on the scanner. The fingerprint
    /// will be returned by the next call to `capture_fingerprint()`.
    ///
    /// # Errors
    ///
    /// Returns an error if the scanner has been dropped and the channel is closed.
    ///
    /// # Examples
    ///
    /// ```
    /// use turnkey_hardware::mock::MockBiometric;
    ///
    /// #[tokio::main]
    /// async fn main() -> turnkey_hardware::Result<()> {
    ///     let (_scanner, mut handle) = MockBiometric::new();
    ///
    ///     let template = vec![1, 2, 3, 4, 5];
    ///     handle.queue_fingerprint(template, 80).await?;
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn queue_fingerprint(&self, template: Vec<u8>, quality: u8) -> Result<()> {
        let data = BiometricData::new(template, quality)?;

        self.event_tx
            .send(BiometricEvent::FingerprintCaptured(data))
            .await
            .map_err(|_| crate::HardwareError::disconnected("Biometric event channel closed"))?;

        Ok(())
    }

    /// Queue a fingerprint with default quality.
    ///
    /// This is a convenience method that uses the default quality threshold (50).
    ///
    /// # Errors
    ///
    /// Returns an error if the scanner has been dropped and the channel is closed.
    ///
    /// # Examples
    ///
    /// ```
    /// use turnkey_hardware::mock::MockBiometric;
    ///
    /// #[tokio::main]
    /// async fn main() -> turnkey_hardware::Result<()> {
    ///     let (_scanner, mut handle) = MockBiometric::new();
    ///
    ///     let template = vec![1, 2, 3, 4, 5];
    ///     handle.queue_fingerprint_default_quality(template).await?;
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn queue_fingerprint_default_quality(&self, template: Vec<u8>) -> Result<()> {
        self.queue_fingerprint(template, DEFAULT_QUALITY_THRESHOLD)
            .await
    }

    /// Queue a fingerprint for a registered user.
    ///
    /// This simulates a user placing their finger on the scanner. The user
    /// must have been previously registered with `add_template()`.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The user is not in the template database
    /// - The scanner has been dropped and the channel is closed
    ///
    /// # Examples
    ///
    /// ```
    /// use turnkey_hardware::mock::MockBiometric;
    ///
    /// #[tokio::main]
    /// async fn main() -> turnkey_hardware::Result<()> {
    ///     let (_scanner, mut handle) = MockBiometric::new();
    ///
    ///     let template = vec![1, 2, 3, 4, 5];
    ///     handle.add_template("user123".to_string(), template).await;
    ///
    ///     handle.queue_user_fingerprint("user123", 85).await?;
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn queue_user_fingerprint(&self, user_id: &str, quality: u8) -> Result<()> {
        let template = self.templates.get(user_id).ok_or_else(|| {
            crate::HardwareError::invalid_data(format!("User {} not in database", user_id))
        })?;

        self.queue_fingerprint(template.clone(), quality).await
    }

    /// Get a template from the database.
    ///
    /// # Examples
    ///
    /// ```
    /// use turnkey_hardware::mock::MockBiometric;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let (_scanner, mut handle) = MockBiometric::new();
    ///
    ///     let template = vec![1, 2, 3, 4, 5];
    ///     handle.add_template("user123".to_string(), template.clone()).await;
    ///
    ///     let retrieved = handle.get_template("user123");
    ///     assert_eq!(retrieved, Some(&template));
    /// }
    /// ```
    pub fn get_template(&self, user_id: &str) -> Option<&Vec<u8>> {
        self.templates.get(user_id)
    }

    /// Get the device name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the number of templates in the database.
    pub fn template_count(&self) -> usize {
        self.templates.len()
    }

    /// Remove a template from the database.
    pub fn remove_template(&mut self, user_id: &str) -> Option<Vec<u8>> {
        self.templates.remove(user_id)
    }

    /// Clear all templates from the database.
    pub fn clear_templates(&mut self) {
        self.templates.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_biometric_capture() {
        let (mut scanner, handle) = MockBiometric::new();

        let template = vec![1, 2, 3, 4, 5];
        tokio::spawn(async move {
            handle
                .queue_fingerprint(template.clone(), 75)
                .await
                .unwrap();
        });

        let captured = scanner.capture_fingerprint().await.unwrap();
        assert_eq!(captured.template, vec![1, 2, 3, 4, 5]);
        assert_eq!(captured.quality, 75);
        assert!(captured.is_quality_acceptable());
    }

    #[tokio::test]
    async fn test_mock_biometric_verify_match() {
        let (mut scanner, handle) = MockBiometric::new();

        let template = vec![1, 2, 3, 4, 5];
        tokio::spawn(async move {
            handle
                .queue_fingerprint(template.clone(), 80)
                .await
                .unwrap();
        });

        let matches = scanner.verify_fingerprint(&[1, 2, 3, 4, 5]).await.unwrap();
        assert!(matches);
    }

    #[tokio::test]
    async fn test_mock_biometric_verify_no_match() {
        let (mut scanner, handle) = MockBiometric::new();

        let template = vec![1, 2, 3, 4, 5];
        tokio::spawn(async move {
            handle.queue_fingerprint(template, 80).await.unwrap();
        });

        let matches = scanner.verify_fingerprint(&[6, 7, 8, 9, 10]).await.unwrap();
        assert!(!matches);
    }

    #[tokio::test]
    async fn test_mock_biometric_led_control() {
        let (mut scanner, _handle) = MockBiometric::new();

        assert_eq!(scanner.led_color(), LedColor::Off);

        scanner.set_led(LedColor::Green).await.unwrap();
        assert_eq!(scanner.led_color(), LedColor::Green);

        scanner.set_led(LedColor::Red).await.unwrap();
        assert_eq!(scanner.led_color(), LedColor::Red);
    }

    #[tokio::test]
    async fn test_mock_biometric_get_device_info() {
        let (scanner, _handle) = MockBiometric::with_name("Test Scanner".to_string());

        let info = scanner.get_device_info().await.unwrap();
        assert_eq!(info.name, "Test Scanner");
        assert_eq!(info.model, "Mock Biometric Scanner v1.0");
        assert_eq!(info.firmware_version, Some("1.0.0".to_string()));
    }

    #[tokio::test]
    async fn test_mock_biometric_handle_add_template() {
        let (_scanner, mut handle) = MockBiometric::new();

        let template1 = vec![1, 2, 3, 4, 5];
        let template2 = vec![6, 7, 8, 9, 10];

        handle
            .add_template("user1".to_string(), template1.clone())
            .await;
        handle
            .add_template("user2".to_string(), template2.clone())
            .await;

        assert_eq!(handle.template_count(), 2);
        assert_eq!(handle.get_template("user1"), Some(&template1));
        assert_eq!(handle.get_template("user2"), Some(&template2));
    }

    #[tokio::test]
    async fn test_mock_biometric_handle_queue_user() {
        let (mut scanner, mut handle) = MockBiometric::new();

        let template = vec![1, 2, 3, 4, 5];
        handle
            .add_template("user123".to_string(), template.clone())
            .await;

        tokio::spawn(async move {
            handle.queue_user_fingerprint("user123", 90).await.unwrap();
        });

        let captured = scanner.capture_fingerprint().await.unwrap();
        assert_eq!(captured.template, template);
        assert_eq!(captured.quality, 90);
    }

    #[tokio::test]
    async fn test_mock_biometric_handle_unknown_user() {
        let (_scanner, handle) = MockBiometric::new();

        let result = handle.queue_user_fingerprint("unknown", 80).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mock_biometric_handle_remove_template() {
        let (_scanner, mut handle) = MockBiometric::new();

        let template = vec![1, 2, 3, 4, 5];
        handle
            .add_template("user1".to_string(), template.clone())
            .await;

        assert_eq!(handle.template_count(), 1);

        let removed = handle.remove_template("user1");
        assert_eq!(removed, Some(template));
        assert_eq!(handle.template_count(), 0);
    }

    #[tokio::test]
    async fn test_mock_biometric_handle_clear_templates() {
        let (_scanner, mut handle) = MockBiometric::new();

        handle
            .add_template("user1".to_string(), vec![1, 2, 3])
            .await;
        handle
            .add_template("user2".to_string(), vec![4, 5, 6])
            .await;

        assert_eq!(handle.template_count(), 2);

        handle.clear_templates();
        assert_eq!(handle.template_count(), 0);
    }

    #[tokio::test]
    async fn test_mock_biometric_handle_default_quality() {
        let (mut scanner, handle) = MockBiometric::new();

        let template = vec![1, 2, 3, 4, 5];
        tokio::spawn(async move {
            handle
                .queue_fingerprint_default_quality(template)
                .await
                .unwrap();
        });

        let captured = scanner.capture_fingerprint().await.unwrap();
        assert_eq!(captured.quality, DEFAULT_QUALITY_THRESHOLD);
        assert!(captured.is_quality_acceptable());
    }

    #[tokio::test]
    async fn test_mock_biometric_low_quality() {
        let (mut scanner, handle) = MockBiometric::new();

        let template = vec![1, 2, 3, 4, 5];
        tokio::spawn(async move {
            handle.queue_fingerprint(template, 30).await.unwrap();
        });

        let captured = scanner.capture_fingerprint().await.unwrap();
        assert_eq!(captured.quality, 30);
        assert!(!captured.is_quality_acceptable());
    }
}
