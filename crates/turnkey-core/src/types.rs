use crate::{
    Result,
    constants::{MAX_CARD_LENGTH, MAX_DEVICE_ID, MIN_CARD_LENGTH, MIN_DEVICE_ID},
    error::Error,
};
use chrono::{DateTime, Local, TimeZone};
use serde::{Deserialize, Serialize};
use std::fmt;
use subtle::ConstantTimeEq;

/// Device identifier (2 digits, zero-padded)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DeviceId(u8);

impl DeviceId {
    /// Create a new device ID with validation.
    ///
    /// # Errors
    /// Returns `Error::InvalidMessageFormat` if the ID is outside the valid range (1-99).
    pub fn new(id: u8) -> Result<Self> {
        if !(MIN_DEVICE_ID..=MAX_DEVICE_ID).contains(&id) {
            return Err(Error::InvalidMessageFormat {
                message: format!("Device ID must be {MIN_DEVICE_ID}-{MAX_DEVICE_ID}, got {id}"),
            });
        }
        Ok(DeviceId(id))
    }

    /// Get the raw device ID as u8.
    #[must_use]
    pub fn as_u8(&self) -> u8 {
        self.0
    }

    /// Format the device ID as a zero-padded 2-digit string.
    #[must_use]
    pub fn to_string_padded(&self) -> String {
        format!("{:02}", self.0)
    }
}

impl fmt::Display for DeviceId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:02}", self.0)
    }
}

impl std::str::FromStr for DeviceId {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let id: u8 = s.parse().map_err(|_| Error::InvalidMessageFormat {
            message: format!("Invalid device ID: {s}"),
        })?;
        DeviceId::new(id)
    }
}

/// Card/badge number (3-20 characters)
///
/// # Security
/// This type implements constant-time comparison to prevent timing attacks
/// when comparing card numbers during authentication.
#[derive(Debug, Clone, Eq, Serialize, Deserialize)]
pub struct CardNumber(String);

impl CardNumber {
    /// Create a new card number with validation.
    ///
    /// The card number is normalized (trimmed and converted to uppercase) before validation.
    ///
    /// # Errors
    /// Returns `Error::InvalidCardFormat` if:
    /// - The card number length is not between 3-20 characters
    /// - The card number contains non-ASCII characters
    pub fn new(number: &str) -> Result<Self> {
        // Normalize: trim and uppercase
        let number = number.trim().to_uppercase();

        let len = number.len();
        if !(MIN_CARD_LENGTH..=MAX_CARD_LENGTH).contains(&len) {
            return Err(Error::InvalidCardFormat(format!(
                "Card number must be {MIN_CARD_LENGTH}-{MAX_CARD_LENGTH} chars, got {len}"
            )));
        }

        // Ensure ASCII only
        if !number.is_ascii() {
            return Err(Error::InvalidCardFormat(
                "Card number must be ASCII".to_string(),
            ));
        }

        Ok(CardNumber(number))
    }

    /// Get the card number as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Left-pad with zeros to 20 characters (protocol standard).
    #[must_use]
    pub fn padded(&self) -> String {
        format!("{:0>20}", self.0)
    }
}

impl fmt::Display for CardNumber {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for CardNumber {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        CardNumber::new(s)
    }
}

/// Constant-time comparison implementation for CardNumber
///
/// This prevents timing attacks by ensuring comparison takes the same time
/// regardless of where the strings differ.
impl PartialEq for CardNumber {
    fn eq(&self, other: &Self) -> bool {
        self.0.as_bytes().ct_eq(other.0.as_bytes()).into()
    }
}

/// Hash implementation for CardNumber
///
/// Implements standard hashing for use in hash-based collections.
impl std::hash::Hash for CardNumber {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

/// Access direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum AccessDirection {
    Undefined = 0,
    Entry = 1,
    Exit = 2,
}

impl AccessDirection {
    /// Create an access direction from a u8 value.
    ///
    /// # Errors
    /// Returns `Error::InvalidDirection` if the value is not 0, 1, or 2.
    #[inline]
    pub fn from_u8(value: u8) -> Result<Self> {
        match value {
            0 => Ok(AccessDirection::Undefined),
            1 => Ok(AccessDirection::Entry),
            2 => Ok(AccessDirection::Exit),
            _ => Err(Error::InvalidDirection { code: value }),
        }
    }

    /// Convert the access direction to a u8 value.
    #[inline]
    #[must_use]
    pub fn to_u8(self) -> u8 {
        self as u8
    }

    /// Returns `true` if direction is Entry.
    #[inline]
    #[must_use]
    pub fn is_entry(self) -> bool {
        matches!(self, AccessDirection::Entry)
    }

    /// Returns `true` if direction is Exit.
    #[inline]
    #[must_use]
    pub fn is_exit(self) -> bool {
        matches!(self, AccessDirection::Exit)
    }

    /// Returns `true` if direction is Undefined.
    #[inline]
    #[must_use]
    pub fn is_undefined(self) -> bool {
        matches!(self, AccessDirection::Undefined)
    }
}

impl fmt::Display for AccessDirection {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AccessDirection::Undefined => write!(f, "Undefined"),
            AccessDirection::Entry => write!(f, "Entry"),
            AccessDirection::Exit => write!(f, "Exit"),
        }
    }
}

/// Reader type indicator
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum ReaderType {
    Rfid = 1,
    Biometric = 5,
}

impl ReaderType {
    /// Create a reader type from a u8 value.
    ///
    /// Supports both legacy and modern Henry protocol encoding schemes:
    /// - Code 0: RFID (legacy devices like ACR122U)
    /// - Code 1: RFID (modern devices)
    /// - Code 5: Biometric (modern devices)
    ///
    /// # Errors
    /// Returns `Error::InvalidReaderType` if the value is not a recognized reader type code.
    ///
    /// # Protocol Compatibility
    ///
    /// Different Henry-compatible devices use different encoding:
    /// - **Legacy**: 0=RFID, 1=Biometric
    /// - **Modern**: 1=RFID, 5=Biometric
    ///
    /// This implementation accepts code 0 or 1 for RFID to maintain backward
    /// compatibility with real Henry equipment (e.g., ACR122U RFID readers).
    #[inline]
    pub fn from_u8(value: u8) -> Result<Self> {
        match value {
            0 | 1 => Ok(ReaderType::Rfid), // Accept both legacy (0) and modern (1)
            5 => Ok(ReaderType::Biometric),
            _ => Err(Error::InvalidReaderType { code: value }),
        }
    }

    /// Convert the reader type to a u8 value (modern encoding).
    ///
    /// Returns codes compatible with modern Henry devices:
    /// - RFID: 1
    /// - Biometric: 5
    #[inline]
    #[must_use]
    pub fn to_u8(self) -> u8 {
        self as u8
    }

    /// Returns `true` if reader type is RFID.
    #[inline]
    #[must_use]
    pub fn is_rfid(self) -> bool {
        matches!(self, ReaderType::Rfid)
    }

    /// Returns `true` if reader type is Biometric.
    #[inline]
    #[must_use]
    pub fn is_biometric(self) -> bool {
        matches!(self, ReaderType::Biometric)
    }
}

/// Timestamp for Henry protocol (dd/mm/yyyy hh:mm:ss)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HenryTimestamp(DateTime<Local>);

impl HenryTimestamp {
    /// Create a timestamp from the current local time.
    #[must_use]
    pub fn now() -> Self {
        HenryTimestamp(Local::now())
    }

    /// Create a timestamp from a DateTime instance.
    #[must_use]
    pub fn from_datetime(dt: DateTime<Local>) -> Self {
        HenryTimestamp(dt)
    }

    /// Parse from Henry format: "10/05/2025 12:46:06".
    ///
    /// # Errors
    /// Returns `Error::InvalidMessageFormat` if the timestamp string does not match
    /// the expected format "dd/mm/yyyy hh:mm:ss", or if the timestamp represents
    /// an invalid local time (e.g., during DST transitions).
    ///
    /// # DST Handling
    ///
    /// During "spring forward" DST transitions, some times don't exist. During
    /// "fall back" transitions, some times are ambiguous. This function:
    /// - Returns error for non-existent times (spring forward gap)
    /// - Chooses the earlier occurrence for ambiguous times (fall back)
    pub fn parse(s: &str) -> Result<Self> {
        let dt = chrono::NaiveDateTime::parse_from_str(s, "%d/%m/%Y %H:%M:%S").map_err(|e| {
            Error::InvalidMessageFormat {
                message: format!("Invalid timestamp '{s}': {e}"),
            }
        })?;

        // Handle DST ambiguity gracefully without panicking
        let local_dt = Local
            .from_local_datetime(&dt)
            .earliest() // Choose first occurrence during "fall back"
            .ok_or_else(|| Error::InvalidMessageFormat {
                message: format!("Invalid local time '{s}' (possibly during DST transition)"),
            })?;

        Ok(HenryTimestamp(local_dt))
    }

    /// Format for Henry protocol (dd/mm/yyyy hh:mm:ss).
    #[must_use]
    pub fn format(&self) -> String {
        self.0.format("%d/%m/%Y %H:%M:%S").to_string()
    }

    /// Get the inner DateTime reference.
    #[must_use]
    pub fn inner(&self) -> &DateTime<Local> {
        &self.0
    }
}

impl fmt::Display for HenryTimestamp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.format())
    }
}

/// Validation mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidationMode {
    /// Offline - local validation only
    Offline,
    /// Online - server validation required
    Online,
    /// Automatic - online with offline fallback
    Automatic,
    /// Semi-automatic - configurable hybrid
    SemiAutomatic,
}

impl ValidationMode {
    /// Create a validation mode from a character code.
    ///
    /// Valid codes: 'F' (Offline), 'O' (Online), 'A' (Automatic), 'S' (Semi-automatic).
    ///
    /// # Errors
    /// Returns `Error::Config` if the character is not a valid validation mode code.
    pub fn from_char(c: char) -> Result<Self> {
        match c {
            'F' => Ok(ValidationMode::Offline),
            'O' => Ok(ValidationMode::Online),
            'A' => Ok(ValidationMode::Automatic),
            'S' => Ok(ValidationMode::SemiAutomatic),
            _ => Err(Error::Config(format!("Invalid validation mode: {c}"))),
        }
    }

    /// Convert the validation mode to its character code.
    #[must_use]
    pub fn to_char(self) -> char {
        match self {
            ValidationMode::Offline => 'F',
            ValidationMode::Online => 'O',
            ValidationMode::Automatic => 'A',
            ValidationMode::SemiAutomatic => 'S',
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case("15", 15)]
    #[case("01", 1)]
    #[case("99", 99)]
    fn test_device_id_valid(#[case] input: &str, #[case] expected: u8) {
        let id: DeviceId = input.parse().unwrap();
        assert_eq!(id.as_u8(), expected);
        assert_eq!(id.to_string_padded(), format!("{:02}", expected));
    }

    #[rstest]
    #[case("00")] // 0 invalid
    #[case("100")] // > 99 invalid
    #[case("abc")] // non-numeric
    fn test_device_id_invalid(#[case] input: &str) {
        let result: Result<DeviceId> = input.parse();
        assert!(result.is_err());
    }

    #[rstest]
    #[case("123", "123")]
    #[case("12345678", "12345678")]
    #[case("12345678901234567890", "12345678901234567890")]
    fn test_card_number_valid(#[case] input: &str, #[case] expected: &str) {
        let card = CardNumber::new(input).unwrap();
        assert_eq!(card.as_str(), expected);
    }

    #[test]
    fn test_card_number_padding() {
        let card = CardNumber::new("12345678").unwrap();
        assert_eq!(card.padded(), "00000000000012345678");
    }

    #[rstest]
    #[case("12")] // too short
    #[case("123456789012345678901")] // too long
    fn test_card_number_invalid(#[case] input: &str) {
        let result = CardNumber::new(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_access_direction() {
        assert_eq!(
            AccessDirection::from_u8(0).unwrap(),
            AccessDirection::Undefined
        );
        assert_eq!(AccessDirection::from_u8(1).unwrap(), AccessDirection::Entry);
        assert_eq!(AccessDirection::from_u8(2).unwrap(), AccessDirection::Exit);
        assert!(AccessDirection::from_u8(3).is_err());

        assert_eq!(AccessDirection::Entry.to_u8(), 1);
    }

    #[test]
    fn test_henry_timestamp() {
        let timestamp = HenryTimestamp::parse("10/05/2025 12:46:06").unwrap();
        let formatted = timestamp.format();
        assert_eq!(formatted, "10/05/2025 12:46:06");
    }

    #[test]
    fn test_validation_mode() {
        assert_eq!(
            ValidationMode::from_char('F').unwrap(),
            ValidationMode::Offline
        );
        assert_eq!(
            ValidationMode::from_char('O').unwrap(),
            ValidationMode::Online
        );
        assert_eq!(
            ValidationMode::from_char('A').unwrap(),
            ValidationMode::Automatic
        );
        assert_eq!(
            ValidationMode::from_char('S').unwrap(),
            ValidationMode::SemiAutomatic
        );
        assert!(ValidationMode::from_char('X').is_err());

        assert_eq!(ValidationMode::Online.to_char(), 'O');
    }
}
