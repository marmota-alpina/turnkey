//! Hardware device trait definitions.
//!
//! This module defines trait interfaces for hardware device abstraction in the
//! turnstile emulator. These traits establish the contract between the emulator
//! core and peripheral devices (keypad, RFID, biometric), enabling polymorphic
//! behavior and easy substitution between mock and real hardware implementations.
//!
//! All traits use native `async fn` methods (Rust 1.90 + Edition 2024 RPITIT),
//! eliminating the need for the `async_trait` macro.

#![allow(async_fn_in_trait)]

use crate::error::Result;
use crate::types::{DeviceInfo, LedColor, ReaderInfo};

/// Input from a keypad device.
///
/// Represents all possible inputs that can be received from a numeric
/// keypad, including digits, special keys, and function keys.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum KeypadInput {
    /// Numeric digit (0-9).
    Digit(u8),

    /// Star key (*).
    Star,

    /// Hash/pound key (#).
    Hash,

    /// Enter/confirm key.
    Enter,

    /// Cancel operation key.
    Cancel,

    /// Clear input key.
    Clear,

    /// Function key (F1-F12).
    FunctionKey(u8),
}

impl KeypadInput {
    /// Create a digit input.
    ///
    /// # Errors
    ///
    /// Returns an error if the digit is greater than 9.
    ///
    /// # Examples
    ///
    /// ```
    /// use turnkey_hardware::traits::KeypadInput;
    ///
    /// let input = KeypadInput::digit(5).unwrap();
    /// assert_eq!(input.as_digit(), Some(5));
    ///
    /// assert!(KeypadInput::digit(10).is_err());
    /// ```
    pub fn digit(d: u8) -> Result<Self> {
        if d > 9 {
            return Err(crate::error::HardwareError::invalid_data(format!(
                "Digit must be 0-9, got {}",
                d
            )));
        }
        Ok(Self::Digit(d))
    }

    /// Create a function key input.
    ///
    /// # Errors
    ///
    /// Returns an error if the function key number is not in the range 1-12.
    ///
    /// # Examples
    ///
    /// ```
    /// use turnkey_hardware::traits::KeypadInput;
    ///
    /// let input = KeypadInput::function_key(1).unwrap();
    ///
    /// assert!(KeypadInput::function_key(0).is_err());
    /// assert!(KeypadInput::function_key(13).is_err());
    /// ```
    pub fn function_key(key: u8) -> Result<Self> {
        if !(1..=12).contains(&key) {
            return Err(crate::error::HardwareError::invalid_data(format!(
                "Function key must be 1-12, got {}",
                key
            )));
        }
        Ok(Self::FunctionKey(key))
    }

    /// Create a digit input without validation (for internal use).
    ///
    /// # Safety
    ///
    /// Caller must ensure the digit is in range 0-9.
    #[allow(dead_code)]
    pub(crate) fn digit_unchecked(d: u8) -> Self {
        debug_assert!(d <= 9, "Digit must be 0-9");
        Self::Digit(d)
    }

    /// Create a function key input without validation (for internal use).
    ///
    /// # Safety
    ///
    /// Caller must ensure the function key is in range 1-12.
    #[allow(dead_code)]
    pub(crate) fn function_key_unchecked(key: u8) -> Self {
        debug_assert!((1..=12).contains(&key), "Function key must be 1-12");
        Self::FunctionKey(key)
    }

    /// Check if this input is a digit.
    pub fn is_digit(&self) -> bool {
        matches!(self, Self::Digit(_))
    }

    /// Get the digit value if this is a digit input.
    pub fn as_digit(&self) -> Option<u8> {
        match self {
            Self::Digit(d) => Some(*d),
            _ => None,
        }
    }
}

/// Keypad device abstraction.
///
/// Represents a numeric keypad that can receive user input. The keypad may
/// support additional features like backlight control and audio feedback.
///
/// # Object Safety and Dynamic Dispatch
///
/// **NOTE**: This trait is NOT object-safe because `async fn` methods return
/// `impl Future`, which is an opaque type that cannot be used in trait objects
/// (Edition 2024 RPITIT). You cannot use `Box<dyn KeypadDevice>` or `&dyn KeypadDevice`.
///
/// For most use cases, use generic type parameters:
///
/// ```no_run
/// use turnkey_hardware::traits::KeypadDevice;
/// use turnkey_hardware::error::Result;
///
/// async fn process_input<K: KeypadDevice>(keypad: &mut K) -> Result<()> {
///     let input = keypad.read_input().await?;
///     Ok(())
/// }
/// ```
///
/// For dynamic dispatch (e.g., in the `PeripheralManager`), use the enum wrapper
/// pattern from the [`devices`](crate::devices) module:
///
/// ```no_run
/// use turnkey_hardware::devices::AnyKeypadDevice;
/// use turnkey_hardware::traits::KeypadDevice;
/// use turnkey_hardware::mock::MockKeypad;
///
/// # async fn example() -> turnkey_hardware::Result<()> {
/// let (keypad, _handle) = MockKeypad::new();
/// let mut any_keypad = AnyKeypadDevice::Mock(keypad);
///
/// // Use through trait interface with zero-cost abstraction
/// let input = any_keypad.read_input().await?;
/// # Ok(())
/// # }
/// ```
///
/// This approach provides:
/// - Zero-cost abstraction (compile-time monomorphization)
/// - Type-safe extensibility
/// - Support for feature flags (conditional compilation)
///
/// See `docs/enum-dispatch-pattern-async-traits.md` for detailed explanation.
///
/// # Examples
///
/// ```no_run
/// use turnkey_hardware::traits::{KeypadDevice, KeypadInput};
/// use turnkey_hardware::error::Result;
///
/// async fn read_pin_code<K: KeypadDevice>(keypad: &mut K) -> Result<String> {
///     let mut code = String::new();
///
///     loop {
///         let input = keypad.read_input().await?;
///
///         match input {
///             KeypadInput::Digit(d) => {
///                 code.push_str(&d.to_string());
///                 keypad.beep(100).await?;
///             }
///             KeypadInput::Enter => {
///                 keypad.beep(200).await?;
///                 break;
///             }
///             KeypadInput::Clear => {
///                 code.clear();
///                 keypad.beep(100).await?;
///             }
///             _ => {}
///         }
///     }
///
///     Ok(code)
/// }
/// ```
pub trait KeypadDevice: Send + Sync {
    /// Read the next input from the keypad.
    ///
    /// This method blocks asynchronously until input is available from
    /// the keypad. It returns the input that was received.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The device is disconnected
    /// - A communication error occurs
    /// - The operation times out
    async fn read_input(&mut self) -> Result<KeypadInput>;

    /// Set the keypad backlight state.
    ///
    /// Enables or disables the keypad backlight if the device supports it.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The device does not support backlight control
    /// - A communication error occurs
    async fn set_backlight(&mut self, enabled: bool) -> Result<()>;

    /// Play a beep sound with the specified duration.
    ///
    /// The duration is specified in milliseconds. The actual sound
    /// characteristics depend on the device hardware.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The device does not support beep functionality
    /// - A communication error occurs
    async fn beep(&mut self, duration_ms: u16) -> Result<()>;

    /// Get device information.
    ///
    /// Returns metadata about the keypad device including name, model,
    /// and optional firmware version.
    ///
    /// # Errors
    ///
    /// Returns an error if a communication error occurs while querying
    /// device information.
    async fn get_info(&self) -> Result<DeviceInfo>;
}

/// RFID card type identification.
///
/// Identifies the type of RFID/NFC card that was read. Different card
/// types have different memory layouts and security features.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CardType {
    /// Mifare Classic 1K (1024 bytes).
    MifareClassic1K,

    /// Mifare Classic 4K (4096 bytes).
    MifareClassic4K,

    /// Mifare Ultralight (64 bytes).
    MifareUltralight,

    /// Mifare DESFire (variable size, secure).
    MifareDESFire,

    /// Unknown card type with ATR/ATS bytes.
    Unknown(Vec<u8>),
}

impl CardType {
    /// Get a human-readable name for the card type.
    pub fn name(&self) -> &str {
        match self {
            Self::MifareClassic1K => "Mifare Classic 1K",
            Self::MifareClassic4K => "Mifare Classic 4K",
            Self::MifareUltralight => "Mifare Ultralight",
            Self::MifareDESFire => "Mifare DESFire",
            Self::Unknown(_) => "Unknown",
        }
    }

    /// Check if this is a known card type.
    pub fn is_known(&self) -> bool {
        !matches!(self, Self::Unknown(_))
    }
}

/// Minimum UID length in bytes (per ISO 14443 specification).
pub const MIN_UID_LENGTH: usize = 4;

/// Maximum UID length in bytes (per ISO 14443 specification).
pub const MAX_UID_LENGTH: usize = 10;

/// RFID card data.
///
/// Contains information about a card that was read by an RFID reader,
/// including the unique identifier (UID), card type, and timestamp.
#[derive(Debug, Clone)]
pub struct CardData {
    /// Card unique identifier (4-10 bytes).
    pub uid: Vec<u8>,

    /// Card type identification.
    pub card_type: CardType,

    /// Timestamp when the card was read.
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl CardData {
    /// Create new card data with the current timestamp.
    ///
    /// # Errors
    ///
    /// Returns an error if the UID length is not within the valid range
    /// of 4-10 bytes as specified by ISO 14443.
    ///
    /// # Examples
    ///
    /// ```
    /// use turnkey_hardware::traits::{CardData, CardType};
    ///
    /// let uid = vec![0x04, 0xAB, 0xCD, 0xEF];
    /// let card = CardData::new(uid, CardType::MifareClassic1K).unwrap();
    /// ```
    pub fn new(uid: Vec<u8>, card_type: CardType) -> Result<Self> {
        CardDataBuilder::new(uid, card_type).build()
    }

    /// Create a builder for constructing card data with optional fields.
    ///
    /// This allows setting custom timestamps for testing or replaying
    /// historical events.
    ///
    /// # Examples
    ///
    /// ```
    /// use turnkey_hardware::traits::{CardData, CardType};
    /// use chrono::{Utc, TimeZone};
    ///
    /// // With custom timestamp
    /// let historical_time = Utc.with_ymd_and_hms(2025, 1, 15, 12, 30, 0).unwrap();
    /// let card = CardData::builder(vec![0x01, 0x02, 0x03, 0x04], CardType::MifareClassic1K)
    ///     .timestamp(historical_time)
    ///     .build()
    ///     .unwrap();
    ///
    /// // Without custom timestamp (uses current time)
    /// let card2 = CardData::builder(vec![0x05, 0x06, 0x07, 0x08], CardType::MifareClassic1K)
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn builder(uid: Vec<u8>, card_type: CardType) -> CardDataBuilder {
        CardDataBuilder::new(uid, card_type)
    }

    /// Create new card data without validation (for internal use).
    ///
    /// # Safety
    ///
    /// Caller must ensure the UID length is within valid range (4-10 bytes).
    #[allow(dead_code)]
    pub(crate) fn new_unchecked(uid: Vec<u8>, card_type: CardType) -> Self {
        debug_assert!(
            uid.len() >= MIN_UID_LENGTH && uid.len() <= MAX_UID_LENGTH,
            "UID length must be 4-10 bytes"
        );
        Self {
            uid,
            card_type,
            timestamp: chrono::Utc::now(),
        }
    }

    /// Get the UID as a hexadecimal string.
    pub fn uid_hex(&self) -> String {
        self.uid
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join("")
    }

    /// Get the UID as a decimal string (common in Brazil).
    ///
    /// # Note
    ///
    /// This method only supports UIDs up to 8 bytes to prevent integer overflow.
    /// For UIDs longer than 8 bytes, this method uses only the first 8 bytes
    /// for conversion. If you need to represent the full UID, use `uid_hex()` instead.
    ///
    /// # Examples
    ///
    /// ```
    /// use turnkey_hardware::traits::{CardData, CardType};
    ///
    /// let uid = vec![0x01, 0x02, 0x03, 0x04];
    /// let card = CardData::new(uid, CardType::MifareClassic1K).unwrap();
    /// assert_eq!(card.uid_decimal(), "16909060");
    /// ```
    pub fn uid_decimal(&self) -> String {
        // Limit to first 8 bytes to prevent overflow in u64
        let bytes = &self.uid[..self.uid.len().min(8)];
        let mut result = 0u64;
        for byte in bytes {
            result = result.saturating_mul(256).saturating_add(*byte as u64);
        }
        result.to_string()
    }
}

/// Builder for constructing CardData with optional fields.
///
/// This builder provides a flexible way to construct CardData instances
/// with custom timestamps or default values, following the Builder pattern
/// for better ergonomics and maintainability.
///
/// # Examples
///
/// ```
/// use turnkey_hardware::traits::{CardData, CardType};
/// use chrono::Utc;
///
/// // Using default timestamp (now)
/// let card = CardData::builder(vec![0x01, 0x02, 0x03, 0x04], CardType::MifareClassic1K)
///     .build()
///     .unwrap();
///
/// // Using custom timestamp
/// let timestamp = Utc::now();
/// let card = CardData::builder(vec![0x01, 0x02, 0x03, 0x04], CardType::MifareClassic1K)
///     .timestamp(timestamp)
///     .build()
///     .unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct CardDataBuilder {
    uid: Vec<u8>,
    card_type: CardType,
    timestamp: Option<chrono::DateTime<chrono::Utc>>,
}

impl CardDataBuilder {
    /// Create a new CardDataBuilder with required fields.
    ///
    /// # Arguments
    ///
    /// * `uid` - The card's unique identifier bytes
    /// * `card_type` - The type of card detected
    pub fn new(uid: Vec<u8>, card_type: CardType) -> Self {
        Self {
            uid,
            card_type,
            timestamp: None,
        }
    }

    /// Set a custom timestamp for the card read event.
    ///
    /// If not set, the current time will be used when build() is called.
    ///
    /// # Arguments
    ///
    /// * `timestamp` - Custom timestamp for the card read event
    pub fn timestamp(mut self, timestamp: chrono::DateTime<chrono::Utc>) -> Self {
        self.timestamp = Some(timestamp);
        self
    }

    /// Build the CardData instance with validation.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - UID length is not between MIN_UID_LENGTH and MAX_UID_LENGTH
    /// - UID is empty
    ///
    /// # Examples
    ///
    /// ```
    /// use turnkey_hardware::traits::{CardData, CardType};
    ///
    /// let card = CardData::builder(vec![0x01, 0x02, 0x03, 0x04], CardType::MifareClassic1K)
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn build(self) -> Result<CardData> {
        let uid_len = self.uid.len();
        if !(MIN_UID_LENGTH..=MAX_UID_LENGTH).contains(&uid_len) {
            return Err(crate::HardwareError::invalid_data(format!(
                "Card UID length must be between {} and {} bytes, got {}",
                MIN_UID_LENGTH, MAX_UID_LENGTH, uid_len
            )));
        }

        if self.uid.is_empty() {
            return Err(crate::HardwareError::invalid_data(
                "Card UID cannot be empty",
            ));
        }

        Ok(CardData {
            uid: self.uid,
            card_type: self.card_type,
            timestamp: self.timestamp.unwrap_or_else(chrono::Utc::now),
        })
    }
}

/// RFID reader device abstraction.
///
/// Represents an RFID/NFC card reader that can detect and read cards.
/// The reader may support LED control for visual feedback.
///
/// # Object Safety and Dynamic Dispatch
///
/// **NOTE**: This trait is NOT object-safe because `async fn` methods return
/// `impl Future`, which is an opaque type that cannot be used in trait objects
/// (Edition 2024 RPITIT). You cannot use `Box<dyn RfidDevice>` or `&dyn RfidDevice`.
///
/// For generic type parameters, see [`KeypadDevice`] documentation.
/// For dynamic dispatch, use [`AnyRfidDevice`](crate::devices::AnyRfidDevice).
///
/// # Examples
///
/// ```no_run
/// use turnkey_hardware::traits::RfidDevice;
/// use turnkey_hardware::types::LedColor;
/// use turnkey_hardware::error::Result;
///
/// async fn wait_for_card<R: RfidDevice>(reader: &mut R) -> Result<String> {
///     println!("Present card...");
///
///     let card = reader.read_card().await?;
///
///     reader.set_led(LedColor::Green).await.ok();
///
///     Ok(card.uid_decimal())
/// }
/// ```
pub trait RfidDevice: Send + Sync {
    /// Read a card from the reader.
    ///
    /// This method blocks asynchronously until a card is presented to the
    /// reader or a timeout occurs. It returns the card data including UID
    /// and card type.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No card is detected within the timeout period
    /// - The card cannot be read (communication error)
    /// - The device is disconnected
    async fn read_card(&mut self) -> Result<CardData>;

    /// Check if a card is currently present on the reader.
    ///
    /// This is a non-blocking check that returns immediately.
    ///
    /// # Errors
    ///
    /// Returns an error if a communication error occurs while checking
    /// for card presence.
    async fn is_card_present(&self) -> Result<bool>;

    /// Get reader information.
    ///
    /// Returns metadata about the RFID reader including name, supported
    /// protocols, and maximum baud rate.
    ///
    /// # Errors
    ///
    /// Returns an error if a communication error occurs while querying
    /// reader information.
    async fn get_reader_info(&self) -> Result<ReaderInfo>;

    /// Set the reader LED color.
    ///
    /// Controls the LED on the reader for visual feedback. Not all readers
    /// support LED control.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The device does not support LED control
    /// - A communication error occurs
    async fn set_led(&mut self, color: LedColor) -> Result<()>;
}

/// Minimum biometric quality score for acceptable captures.
///
/// This threshold is based on industry standards for fingerprint matching.
/// Values 0-49 are considered poor quality, 50-100 are acceptable.
pub const DEFAULT_QUALITY_THRESHOLD: u8 = 50;

/// Maximum biometric quality score.
///
/// Quality scores range from 0 (lowest) to 100 (highest).
pub const MAX_QUALITY_SCORE: u8 = 100;

/// Biometric template data.
///
/// Contains a fingerprint template captured by a biometric scanner.
/// The template format is device-specific and not interchangeable
/// between different scanner vendors.
///
/// # Development Note
///
/// This emulator is designed for development and testing without real hardware.
/// All data is synthetic test data. In production systems using real biometric
/// hardware, template data should be encrypted at rest and handled according
/// to privacy regulations (LGPD in Brazil, GDPR in EU).
#[derive(Debug, Clone)]
pub struct BiometricData {
    /// Fingerprint template data (format is device-specific).
    pub template: Vec<u8>,

    /// Quality score of the capture (0-100, higher is better).
    pub quality: u8,

    /// Timestamp when the fingerprint was captured.
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl BiometricData {
    /// Create new biometric data with the current timestamp.
    ///
    /// This is a convenience method that delegates to the builder pattern.
    /// For more control over construction, use `BiometricData::builder()`.
    ///
    /// # Errors
    ///
    /// Returns an error if the quality score is greater than 100.
    ///
    /// # Examples
    ///
    /// ```
    /// use turnkey_hardware::traits::BiometricData;
    ///
    /// let data = BiometricData::new(vec![0u8; 512], 75).unwrap();
    /// assert_eq!(data.quality, 75);
    ///
    /// // Invalid quality
    /// assert!(BiometricData::new(vec![0u8; 512], 101).is_err());
    /// ```
    pub fn new(template: Vec<u8>, quality: u8) -> Result<Self> {
        BiometricDataBuilder::new(template, quality).build()
    }

    /// Create a builder for constructing BiometricData with optional fields.
    ///
    /// # Examples
    ///
    /// ```
    /// use turnkey_hardware::traits::BiometricData;
    /// use chrono::Utc;
    ///
    /// // Using default timestamp (now)
    /// let data = BiometricData::builder(vec![0u8; 512], 75).build().unwrap();
    ///
    /// // Using custom timestamp
    /// let timestamp = Utc::now();
    /// let data = BiometricData::builder(vec![0u8; 512], 75)
    ///     .timestamp(timestamp)
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn builder(template: Vec<u8>, quality: u8) -> BiometricDataBuilder {
        BiometricDataBuilder::new(template, quality)
    }

    /// Check if the capture quality meets or exceeds the default threshold.
    ///
    /// Uses the default threshold of 50. For custom thresholds, use
    /// `is_quality_acceptable_with_threshold()` or compare the `quality`
    /// field directly.
    ///
    /// # Examples
    ///
    /// ```
    /// use turnkey_hardware::traits::BiometricData;
    ///
    /// let data = BiometricData::new(vec![0u8; 512], 60).unwrap();
    /// assert!(data.is_quality_acceptable());
    ///
    /// // Custom threshold
    /// const STRICT_THRESHOLD: u8 = 70;
    /// let high_quality = BiometricData::new(vec![0u8; 512], 75).unwrap();
    /// assert!(high_quality.quality >= STRICT_THRESHOLD);
    /// ```
    pub fn is_quality_acceptable(&self) -> bool {
        self.is_quality_acceptable_with_threshold(DEFAULT_QUALITY_THRESHOLD)
    }

    /// Check if the capture quality meets or exceeds a custom threshold.
    ///
    /// # Examples
    ///
    /// ```
    /// use turnkey_hardware::traits::BiometricData;
    ///
    /// let data = BiometricData::new(vec![0u8; 512], 65).unwrap();
    /// assert!(data.is_quality_acceptable_with_threshold(60));
    /// assert!(!data.is_quality_acceptable_with_threshold(70));
    /// ```
    pub fn is_quality_acceptable_with_threshold(&self, threshold: u8) -> bool {
        self.quality >= threshold
    }
}

/// Builder for constructing BiometricData with optional fields.
///
/// This builder provides a flexible way to construct BiometricData instances
/// with custom timestamps or default values, following the Builder pattern
/// for better ergonomics and maintainability.
///
/// # Examples
///
/// ```
/// use turnkey_hardware::traits::BiometricData;
/// use chrono::Utc;
///
/// // Using default timestamp (now)
/// let data = BiometricData::builder(vec![0u8; 512], 75).build();
///
/// // Using custom timestamp
/// let timestamp = Utc::now();
/// let data = BiometricData::builder(vec![0u8; 512], 75)
///     .timestamp(timestamp)
///     .build();
/// ```
#[derive(Debug, Clone)]
pub struct BiometricDataBuilder {
    template: Vec<u8>,
    quality: u8,
    timestamp: Option<chrono::DateTime<chrono::Utc>>,
}

impl BiometricDataBuilder {
    /// Create a new BiometricDataBuilder with required fields.
    ///
    /// # Arguments
    ///
    /// * `template` - The fingerprint template data
    /// * `quality` - The quality score (0-100)
    pub fn new(template: Vec<u8>, quality: u8) -> Self {
        Self {
            template,
            quality,
            timestamp: None,
        }
    }

    /// Set a custom timestamp for the biometric capture event.
    ///
    /// If not set, the current time will be used when build() is called.
    ///
    /// # Arguments
    ///
    /// * `timestamp` - Custom timestamp for the capture event
    pub fn timestamp(mut self, timestamp: chrono::DateTime<chrono::Utc>) -> Self {
        self.timestamp = Some(timestamp);
        self
    }

    /// Build the BiometricData instance with validation.
    ///
    /// # Errors
    ///
    /// Returns an error if the quality score is greater than 100.
    ///
    /// # Examples
    ///
    /// ```
    /// use turnkey_hardware::traits::BiometricData;
    ///
    /// let data = BiometricData::builder(vec![0u8; 512], 75).build().unwrap();
    /// assert_eq!(data.quality, 75);
    /// assert_eq!(data.template.len(), 512);
    ///
    /// // Invalid quality
    /// let result = BiometricData::builder(vec![0u8; 512], 101).build();
    /// assert!(result.is_err());
    /// ```
    pub fn build(self) -> Result<BiometricData> {
        Self::validate_quality(self.quality)?;

        Ok(BiometricData {
            template: self.template,
            quality: self.quality,
            timestamp: self.timestamp.unwrap_or_else(chrono::Utc::now),
        })
    }

    /// Validate that quality score is within valid range (0-100).
    ///
    /// Returns an error if quality exceeds MAX_QUALITY_SCORE.
    fn validate_quality(quality: u8) -> Result<()> {
        if quality > MAX_QUALITY_SCORE {
            return Err(crate::HardwareError::invalid_data(format!(
                "Biometric quality must be 0-{}, got {}",
                MAX_QUALITY_SCORE, quality
            )));
        }
        Ok(())
    }
}

/// Biometric device abstraction.
///
/// Represents a fingerprint scanner that can capture fingerprints and
/// verify them against stored templates. The device may support LED
/// control for visual feedback.
///
/// # Object Safety and Dynamic Dispatch
///
/// **NOTE**: This trait is NOT object-safe because `async fn` methods return
/// `impl Future`, which is an opaque type that cannot be used in trait objects
/// (Edition 2024 RPITIT). You cannot use `Box<dyn BiometricDevice>` or `&dyn BiometricDevice`.
///
/// For generic type parameters, see [`KeypadDevice`] documentation.
/// For dynamic dispatch, use [`AnyBiometricDevice`](crate::devices::AnyBiometricDevice).
///
/// # Examples
///
/// ```no_run
/// use turnkey_hardware::traits::BiometricDevice;
/// use turnkey_hardware::types::LedColor;
/// use turnkey_hardware::error::Result;
///
/// async fn enroll_fingerprint<B: BiometricDevice>(scanner: &mut B) -> Result<Vec<u8>> {
///     println!("Place finger on scanner...");
///
///     let biometric = scanner.capture_fingerprint().await?;
///
///     if biometric.is_quality_acceptable() {
///         scanner.set_led(LedColor::Green).await.ok();
///         Ok(biometric.template)
///     } else {
///         scanner.set_led(LedColor::Red).await.ok();
///         Err(turnkey_hardware::error::HardwareError::biometric_capture(
///             "Low quality capture",
///         ))
///     }
/// }
/// ```
pub trait BiometricDevice: Send + Sync {
    /// Capture a fingerprint from the scanner.
    ///
    /// This method blocks asynchronously until a finger is placed on the
    /// scanner and a capture is completed. It returns the biometric data
    /// including the template and quality score.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No finger is detected within the timeout period
    /// - The capture quality is too low
    /// - A communication error occurs
    /// - The device is disconnected
    async fn capture_fingerprint(&mut self) -> Result<BiometricData>;

    /// Verify a fingerprint against a stored template.
    ///
    /// This method captures a fingerprint and compares it against the
    /// provided template. It returns true if the fingerprint matches,
    /// false otherwise.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The fingerprint capture fails
    /// - A communication error occurs
    /// - The device is disconnected
    async fn verify_fingerprint(&mut self, template: &[u8]) -> Result<bool>;

    /// Get device information.
    ///
    /// Returns metadata about the biometric scanner including name, model,
    /// and optional firmware version.
    ///
    /// # Errors
    ///
    /// Returns an error if a communication error occurs while querying
    /// device information.
    async fn get_device_info(&self) -> Result<DeviceInfo>;

    /// Set the scanner LED color.
    ///
    /// Controls the LED on the scanner for visual feedback. Not all scanners
    /// support LED control.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The device does not support LED control
    /// - A communication error occurs
    async fn set_led(&mut self, color: LedColor) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keypad_input_digit() {
        let input = KeypadInput::digit(5).unwrap();
        assert_eq!(input, KeypadInput::Digit(5));
        assert!(input.is_digit());
        assert_eq!(input.as_digit(), Some(5));
    }

    #[test]
    fn test_keypad_input_invalid_digit() {
        let result = KeypadInput::digit(10);
        assert!(result.is_err());
    }

    #[test]
    fn test_keypad_input_function_key() {
        let input = KeypadInput::function_key(1).unwrap();
        assert_eq!(input, KeypadInput::FunctionKey(1));
        assert!(!input.is_digit());
        assert_eq!(input.as_digit(), None);
    }

    #[test]
    fn test_keypad_input_invalid_function_key() {
        let result = KeypadInput::function_key(13);
        assert!(result.is_err());

        let result = KeypadInput::function_key(0);
        assert!(result.is_err());
    }

    #[test]
    fn test_card_type_name() {
        assert_eq!(CardType::MifareClassic1K.name(), "Mifare Classic 1K");
        assert_eq!(CardType::MifareDESFire.name(), "Mifare DESFire");
        assert_eq!(CardType::Unknown(vec![]).name(), "Unknown");
    }

    #[test]
    fn test_card_type_is_known() {
        assert!(CardType::MifareClassic1K.is_known());
        assert!(CardType::MifareClassic4K.is_known());
        assert!(!CardType::Unknown(vec![]).is_known());
    }

    #[test]
    fn test_card_data_uid_hex() {
        let card = CardData::new(vec![0x04, 0xAB, 0xCD, 0xEF], CardType::MifareClassic1K).unwrap();
        assert_eq!(card.uid_hex(), "04ABCDEF");
    }

    #[test]
    fn test_card_data_uid_decimal() {
        let card = CardData::new(vec![0x01, 0x02, 0x03, 0x04], CardType::MifareClassic1K).unwrap();
        assert_eq!(card.uid_decimal(), "16909060");
    }

    #[test]
    fn test_card_data_invalid_uid_length() {
        // Too short
        let result = CardData::new(vec![0x01, 0x02], CardType::MifareClassic1K);
        assert!(result.is_err());

        // Too long
        let result = CardData::new(vec![0x01; 11], CardType::MifareClassic1K);
        assert!(result.is_err());

        // Valid lengths
        let result = CardData::new(vec![0x01; 4], CardType::MifareClassic1K);
        assert!(result.is_ok());

        let result = CardData::new(vec![0x01; 10], CardType::MifareClassic1K);
        assert!(result.is_ok());
    }

    #[test]
    fn test_biometric_data_quality() {
        let data = BiometricData::new(vec![0u8; 512], 60).unwrap();
        assert!(data.is_quality_acceptable());
        assert!(data.is_quality_acceptable_with_threshold(50));
        assert!(data.is_quality_acceptable_with_threshold(60));
        assert!(!data.is_quality_acceptable_with_threshold(61));

        let low_quality = BiometricData::new(vec![0u8; 512], 30).unwrap();
        assert!(!low_quality.is_quality_acceptable());
        assert!(!low_quality.is_quality_acceptable_with_threshold(50));
        assert!(low_quality.is_quality_acceptable_with_threshold(30));
    }

    #[test]
    fn test_biometric_data_template_access() {
        let template_data = vec![1u8, 2, 3, 4, 5];
        let data = BiometricData::new(template_data.clone(), 70).unwrap();

        // Direct field access (development emulator - no hiding needed)
        assert_eq!(data.template, template_data);
    }

    #[test]
    fn test_biometric_data_debug() {
        let data = BiometricData::new(vec![0xDE, 0xAD, 0xBE, 0xEF], 75).unwrap();
        let debug_output = format!("{:?}", data);

        // Development emulator - Debug shows all data
        assert!(debug_output.contains("quality"));
        assert!(debug_output.contains("75"));
        assert!(debug_output.contains("template"));
    }

    #[test]
    fn test_biometric_data_quality_validation() {
        // Valid quality values (0-100)
        assert!(BiometricData::new(vec![0u8; 512], 0).is_ok());
        assert!(BiometricData::new(vec![0u8; 512], 50).is_ok());
        assert!(BiometricData::new(vec![0u8; 512], 100).is_ok());

        // Invalid quality values (>100)
        assert!(BiometricData::new(vec![0u8; 512], 101).is_err());
        assert!(BiometricData::new(vec![0u8; 512], 200).is_err());
        assert!(BiometricData::new(vec![0u8; 512], 255).is_err());
    }

    #[test]
    fn test_biometric_builder_quality_validation() {
        // Valid quality via builder
        let result = BiometricData::builder(vec![0u8; 512], 75).build();
        assert!(result.is_ok());

        // Invalid quality via builder
        let result = BiometricData::builder(vec![0u8; 512], 150).build();
        assert!(result.is_err());
    }

    // Note: async fn in traits (Edition 2024 RPITIT) are not object-safe by default.
    // These traits are designed to work with generics and impl blocks, which is
    // the primary use case. If object safety is needed in the future, consider
    // using the `async_trait` crate or manually desugaring to return `impl Future + Send`.
    //
    // Example usage with generics:
    // async fn process_keypad<K: KeypadDevice>(keypad: &mut K) { ... }
}
