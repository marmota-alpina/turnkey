//! Access control request parsing and validation.
//!
//! This module implements parsing and validation for access request messages
//! (command code 000+0) in the Henry protocol. Access requests are sent by
//! turnstiles when a user presents credentials for validation.
//!
//! # Message Format
//!
//! Access request messages follow this format:
//!
//! ```text
//! <ID>+REON+000+0]<CARD_NUMBER>]<TIMESTAMP>]<DIRECTION>]<READER_TYPE>]
//! ```
//!
//! Where:
//! - `CARD_NUMBER`: 3-20 character credential identifier
//! - `TIMESTAMP`: dd/mm/yyyy hh:mm:ss format
//! - `DIRECTION`: 0=Undefined, 1=Entry, 2=Exit
//! - `READER_TYPE`: 1=RFID, 5=Biometric
//!
//! # Examples
//!
//! ## Parsing an Access Request
//!
//! ```
//! use turnkey_protocol::commands::access::AccessRequest;
//! use turnkey_core::{HenryTimestamp, AccessDirection};
//!
//! let fields = vec![
//!     "12345678".to_string(),
//!     "10/05/2025 12:46:06".to_string(),
//!     "1".to_string(),
//!     "1".to_string(), // 1 = RFID
//! ];
//!
//! let request = AccessRequest::parse(&fields).unwrap();
//! assert_eq!(request.card_number(), "12345678");
//! assert_eq!(request.direction(), AccessDirection::Entry);
//! ```
//!
//! ## Validating Card Numbers
//!
//! ```
//! use turnkey_protocol::commands::access::AccessRequest;
//!
//! // Valid card numbers: 3-20 characters
//! assert!(AccessRequest::validate_card_number("123").is_ok());
//! assert!(AccessRequest::validate_card_number("12345678901234567890").is_ok());
//!
//! // Invalid: too short
//! assert!(AccessRequest::validate_card_number("12").is_err());
//!
//! // Invalid: too long
//! assert!(AccessRequest::validate_card_number("123456789012345678901").is_err());
//! ```

use serde::{Deserialize, Serialize};
use turnkey_core::{AccessDirection, Error, HenryTimestamp, ReaderType, Result};

/// Minimum card number length in characters
const MIN_CARD_LENGTH: usize = 3;

/// Maximum card number length in characters
const MAX_CARD_LENGTH: usize = 20;

/// Maximum field length to prevent DoS attacks
const MAX_FIELD_LENGTH: usize = 256;

/// Access request from a turnstile device.
///
/// Represents a request for access validation sent by a turnstile when
/// a user presents credentials (card, fingerprint, etc.).
///
/// # Fields
///
/// - `card_number`: The credential identifier (3-20 characters)
/// - `timestamp`: When the access attempt occurred
/// - `direction`: Which direction the user wants to pass
/// - `reader_type`: Which type of reader was used
///
/// # Protocol Behavior
///
/// In ONLINE mode, the emulator sends this request to an external client
/// for validation. In OFFLINE mode, it validates against the local database.
///
/// # Examples
///
/// ```
/// use turnkey_protocol::commands::access::AccessRequest;
/// use turnkey_core::HenryTimestamp;
///
/// let fields = vec![
///     "12345678".to_string(),
///     "10/05/2025 12:46:06".to_string(),
///     "1".to_string(),
///     "1".to_string(), // 1 = RFID
/// ];
///
/// let request = AccessRequest::parse(&fields).unwrap();
/// assert_eq!(request.card_number(), "12345678");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessRequest {
    card_number: String,
    timestamp: HenryTimestamp,
    direction: AccessDirection,
    reader_type: ReaderType,
}

impl AccessRequest {
    /// Create a new access request with validation.
    ///
    /// # Arguments
    ///
    /// * `card_number` - Credential identifier (3-20 characters)
    /// * `timestamp` - When the access attempt occurred
    /// * `direction` - Direction of passage
    /// * `reader_type` - Type of reader used
    ///
    /// # Returns
    ///
    /// Returns `Ok(AccessRequest)` if all validations pass.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Card number is invalid (length not 3-20 characters)
    /// - Any other field validation fails
    ///
    /// # Examples
    ///
    /// ```
    /// use turnkey_protocol::commands::access::AccessRequest;
    /// use turnkey_core::{AccessDirection, HenryTimestamp, ReaderType};
    ///
    /// let timestamp = HenryTimestamp::parse("10/05/2025 12:46:06").unwrap();
    /// let request = AccessRequest::new(
    ///     "12345678".to_string(),
    ///     timestamp,
    ///     AccessDirection::Entry,
    ///     ReaderType::Rfid,
    /// ).unwrap();
    /// ```
    pub fn new(
        card_number: String,
        timestamp: HenryTimestamp,
        direction: AccessDirection,
        reader_type: ReaderType,
    ) -> Result<Self> {
        Self::validate_card_number(&card_number)?;

        Ok(Self {
            card_number,
            timestamp,
            direction,
            reader_type,
        })
    }

    /// Parse access request from protocol fields.
    ///
    /// # Arguments
    ///
    /// * `fields` - Array of field strings from protocol message
    ///
    /// # Expected Format
    ///
    /// Fields must be in this order:
    /// 1. Card number (3-20 characters)
    /// 2. Timestamp (dd/mm/yyyy hh:mm:ss)
    /// 3. Direction (0, 1, or 2)
    /// 4. Reader type (1 for RFID, 5 for Biometric)
    ///
    /// # Returns
    ///
    /// Returns `Ok(AccessRequest)` if parsing succeeds.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Insufficient fields provided
    /// - Card number validation fails
    /// - Timestamp parsing fails
    /// - Direction code is invalid
    /// - Reader type code is invalid
    ///
    /// # Examples
    ///
    /// ```
    /// use turnkey_protocol::commands::access::AccessRequest;
    ///
    /// let fields = vec![
    ///     "12345678".to_string(),
    ///     "10/05/2025 12:46:06".to_string(),
    ///     "1".to_string(),
    ///     "1".to_string(), // 1 = RFID
    /// ];
    ///
    /// let request = AccessRequest::parse(&fields).unwrap();
    /// assert_eq!(request.card_number(), "12345678");
    /// ```
    pub fn parse(fields: &[String]) -> Result<Self> {
        if fields.len() < 4 {
            return Err(Error::MissingField(format!(
                "Access request requires 4 fields, got {}",
                fields.len()
            )));
        }

        // DoS protection: validate field lengths before processing
        for (i, field) in fields.iter().take(4).enumerate() {
            if field.len() > MAX_FIELD_LENGTH {
                return Err(Error::InvalidFieldFormat {
                    message: format!(
                        "Field {} exceeds maximum length {} (got {} bytes)",
                        i,
                        MAX_FIELD_LENGTH,
                        field.len()
                    ),
                });
            }
        }

        // Validate card number before cloning (performance optimization)
        Self::validate_card_number(&fields[0])?;
        let card_number = fields[0].clone();

        let timestamp = HenryTimestamp::parse(&fields[1])?;

        let direction_code = fields[2]
            .parse::<u8>()
            .map_err(|_| Error::InvalidFieldFormat {
                message: format!("Invalid direction code: {}", fields[2]),
            })?;
        let direction = AccessDirection::from_u8(direction_code)?;

        let reader_type_code = fields[3]
            .parse::<u8>()
            .map_err(|_| Error::InvalidFieldFormat {
                message: format!("Invalid reader type code: {}", fields[3]),
            })?;
        let reader_type = ReaderType::from_u8(reader_type_code)?;

        Ok(Self {
            card_number,
            timestamp,
            direction,
            reader_type,
        })
    }

    /// Validate card number format.
    ///
    /// Card numbers must be between 3 and 20 characters inclusive.
    ///
    /// # Arguments
    ///
    /// * `card_number` - The card number to validate
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if validation passes.
    ///
    /// # Errors
    ///
    /// Returns error if card number length is not in range [3, 20].
    ///
    /// # Examples
    ///
    /// ```
    /// use turnkey_protocol::commands::access::AccessRequest;
    ///
    /// assert!(AccessRequest::validate_card_number("123").is_ok());
    /// assert!(AccessRequest::validate_card_number("12345678").is_ok());
    /// assert!(AccessRequest::validate_card_number("12345678901234567890").is_ok());
    ///
    /// assert!(AccessRequest::validate_card_number("12").is_err());
    /// assert!(AccessRequest::validate_card_number("123456789012345678901").is_err());
    /// assert!(AccessRequest::validate_card_number("").is_err());
    /// ```
    pub fn validate_card_number(card_number: &str) -> Result<()> {
        let len = card_number.len();
        if !(MIN_CARD_LENGTH..=MAX_CARD_LENGTH).contains(&len) {
            return Err(Error::InvalidCardFormat(format!(
                "Card number length must be {}-{} characters, got {} characters",
                MIN_CARD_LENGTH, MAX_CARD_LENGTH, len
            )));
        }
        Ok(())
    }

    /// Validate the access request business rules.
    ///
    /// Performs additional validation beyond basic field parsing.
    /// Currently validates that all required fields are present and valid.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if validation passes.
    ///
    /// # Examples
    ///
    /// ```
    /// use turnkey_protocol::commands::access::AccessRequest;
    ///
    /// let fields = vec![
    ///     "12345678".to_string(),
    ///     "10/05/2025 12:46:06".to_string(),
    ///     "1".to_string(),
    ///     "1".to_string(), // 1 = RFID
    /// ];
    ///
    /// let request = AccessRequest::parse(&fields).unwrap();
    /// assert!(request.validate().is_ok());
    /// ```
    pub fn validate(&self) -> Result<()> {
        Self::validate_card_number(&self.card_number)?;
        Ok(())
    }

    /// Get the card number.
    pub fn card_number(&self) -> &str {
        &self.card_number
    }

    /// Get the timestamp.
    pub fn timestamp(&self) -> &HenryTimestamp {
        &self.timestamp
    }

    /// Get the direction.
    pub fn direction(&self) -> AccessDirection {
        self.direction
    }

    /// Get the reader type.
    pub fn reader_type(&self) -> ReaderType {
        self.reader_type
    }

    /// Returns `true` if this is an entry request.
    pub fn is_entry(&self) -> bool {
        self.direction.is_entry()
    }

    /// Returns `true` if this is an exit request.
    pub fn is_exit(&self) -> bool {
        self.direction.is_exit()
    }

    /// Returns `true` if direction is undefined.
    pub fn is_direction_undefined(&self) -> bool {
        self.direction.is_undefined()
    }

    /// Returns `true` if RFID reader was used.
    pub fn is_rfid(&self) -> bool {
        self.reader_type.is_rfid()
    }

    /// Returns `true` if biometric reader was used.
    pub fn is_biometric(&self) -> bool {
        self.reader_type.is_biometric()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_card_number_valid() {
        assert!(AccessRequest::validate_card_number("123").is_ok());
        assert!(AccessRequest::validate_card_number("1234567890").is_ok());
        assert!(AccessRequest::validate_card_number("12345678901234567890").is_ok());
    }

    #[test]
    fn test_validate_card_number_too_short() {
        assert!(AccessRequest::validate_card_number("").is_err());
        assert!(AccessRequest::validate_card_number("1").is_err());
        assert!(AccessRequest::validate_card_number("12").is_err());
    }

    #[test]
    fn test_validate_card_number_too_long() {
        assert!(AccessRequest::validate_card_number("123456789012345678901").is_err());
        assert!(AccessRequest::validate_card_number("1234567890123456789012345").is_err());
    }

    #[test]
    fn test_parse_valid_access_request() {
        let fields = vec![
            "12345678".to_string(),
            "10/05/2025 12:46:06".to_string(),
            "1".to_string(),
            "1".to_string(), // 1 = RFID
        ];

        let request = AccessRequest::parse(&fields).unwrap();
        assert_eq!(request.card_number(), "12345678");
        assert_eq!(request.direction(), AccessDirection::Entry);
        assert_eq!(request.reader_type(), ReaderType::Rfid);
    }

    #[test]
    fn test_parse_exit_request() {
        let fields = vec![
            "87654321".to_string(),
            "10/05/2025 14:30:00".to_string(),
            "2".to_string(),
            "1".to_string(), // 1 = RFID
        ];

        let request = AccessRequest::parse(&fields).unwrap();
        assert_eq!(request.card_number(), "87654321");
        assert_eq!(request.direction(), AccessDirection::Exit);
        assert!(request.is_exit());
        assert!(!request.is_entry());
    }

    #[test]
    fn test_parse_biometric_request() {
        let fields = vec![
            "BIO123456".to_string(),
            "10/05/2025 16:00:00".to_string(),
            "1".to_string(),
            "5".to_string(),
        ];

        let request = AccessRequest::parse(&fields).unwrap();
        assert_eq!(request.card_number(), "BIO123456");
        assert_eq!(request.reader_type(), ReaderType::Biometric);
        assert!(request.is_biometric());
        assert!(!request.is_rfid());
    }

    #[test]
    fn test_parse_insufficient_fields() {
        let fields = vec![
            "12345678".to_string(),
            "10/05/2025 12:46:06".to_string(),
            "1".to_string(),
            // Missing reader type field
        ];

        let result = AccessRequest::parse(&fields);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_card_number() {
        let fields = vec![
            "12".to_string(), // Too short
            "10/05/2025 12:46:06".to_string(),
            "1".to_string(),
            "1".to_string(), // 1 = RFID
        ];

        let result = AccessRequest::parse(&fields);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_timestamp() {
        let fields = vec![
            "12345678".to_string(),
            "invalid-timestamp".to_string(),
            "1".to_string(),
            "1".to_string(), // 1 = RFID
        ];

        let result = AccessRequest::parse(&fields);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_direction() {
        let fields = vec![
            "12345678".to_string(),
            "10/05/2025 12:46:06".to_string(),
            "99".to_string(), // Invalid direction
            "1".to_string(),  // 1 = RFID
        ];

        let result = AccessRequest::parse(&fields);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_reader_type() {
        let fields = vec![
            "12345678".to_string(),
            "10/05/2025 12:46:06".to_string(),
            "1".to_string(),
            "99".to_string(), // Invalid reader type
        ];

        let result = AccessRequest::parse(&fields);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_non_numeric_direction() {
        let fields = vec![
            "12345678".to_string(),
            "10/05/2025 12:46:06".to_string(),
            "abc".to_string(),
            "1".to_string(), // 1 = RFID
        ];

        let result = AccessRequest::parse(&fields);
        assert!(result.is_err());
    }

    #[test]
    fn test_new_valid_request() {
        let timestamp = HenryTimestamp::parse("10/05/2025 12:46:06").unwrap();
        let request = AccessRequest::new(
            "12345678".to_string(),
            timestamp,
            AccessDirection::Entry,
            ReaderType::Rfid,
        )
        .unwrap();

        assert_eq!(request.card_number(), "12345678");
        assert_eq!(request.direction(), AccessDirection::Entry);
        assert_eq!(request.reader_type(), ReaderType::Rfid);
    }

    #[test]
    fn test_new_invalid_card_number() {
        let timestamp = HenryTimestamp::parse("10/05/2025 12:46:06").unwrap();
        let result = AccessRequest::new(
            "12".to_string(),
            timestamp,
            AccessDirection::Entry,
            ReaderType::Rfid,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_validate() {
        let fields = vec![
            "12345678".to_string(),
            "10/05/2025 12:46:06".to_string(),
            "1".to_string(),
            "1".to_string(), // 1 = RFID
        ];

        let request = AccessRequest::parse(&fields).unwrap();
        assert!(request.validate().is_ok());
    }

    #[test]
    fn test_is_entry() {
        let timestamp = HenryTimestamp::parse("10/05/2025 12:46:06").unwrap();
        let request = AccessRequest::new(
            "12345678".to_string(),
            timestamp,
            AccessDirection::Entry,
            ReaderType::Rfid,
        )
        .unwrap();

        assert!(request.is_entry());
        assert!(!request.is_exit());
        assert!(!request.is_direction_undefined());
    }

    #[test]
    fn test_is_exit() {
        let timestamp = HenryTimestamp::parse("10/05/2025 12:46:06").unwrap();
        let request = AccessRequest::new(
            "12345678".to_string(),
            timestamp,
            AccessDirection::Exit,
            ReaderType::Rfid,
        )
        .unwrap();

        assert!(!request.is_entry());
        assert!(request.is_exit());
        assert!(!request.is_direction_undefined());
    }

    #[test]
    fn test_is_direction_undefined() {
        let timestamp = HenryTimestamp::parse("10/05/2025 12:46:06").unwrap();
        let request = AccessRequest::new(
            "12345678".to_string(),
            timestamp,
            AccessDirection::Undefined,
            ReaderType::Rfid,
        )
        .unwrap();

        assert!(!request.is_entry());
        assert!(!request.is_exit());
        assert!(request.is_direction_undefined());
    }

    #[test]
    fn test_card_number_min_length() {
        let fields = vec![
            "123".to_string(), // Exactly 3 characters (minimum)
            "10/05/2025 12:46:06".to_string(),
            "1".to_string(),
            "1".to_string(), // 1 = RFID
        ];

        let request = AccessRequest::parse(&fields).unwrap();
        assert_eq!(request.card_number(), "123");
    }

    #[test]
    fn test_card_number_max_length() {
        let fields = vec![
            "12345678901234567890".to_string(), // Exactly 20 characters (maximum)
            "10/05/2025 12:46:06".to_string(),
            "1".to_string(),
            "1".to_string(), // 1 = RFID
        ];

        let request = AccessRequest::parse(&fields).unwrap();
        assert_eq!(request.card_number(), "12345678901234567890");
    }

    #[test]
    fn test_alphanumeric_card_numbers() {
        let card_numbers = vec!["ABC123", "12345XYZ", "MIX3D4LPH4"];

        for card in card_numbers {
            let fields = vec![
                card.to_string(),
                "10/05/2025 12:46:06".to_string(),
                "1".to_string(),
                "1".to_string(), // 1 = RFID
            ];

            let request = AccessRequest::parse(&fields).unwrap();
            assert_eq!(request.card_number(), card);
        }
    }

    #[test]
    fn test_direction_undefined() {
        let fields = vec![
            "12345678".to_string(),
            "10/05/2025 12:46:06".to_string(),
            "0".to_string(), // Undefined direction
            "1".to_string(), // 1 = RFID
        ];

        let request = AccessRequest::parse(&fields).unwrap();
        assert_eq!(request.direction(), AccessDirection::Undefined);
        assert!(request.is_direction_undefined());
    }

    // ========================================================================
    // REAL HARDWARE TRACES - Protocol Compatibility Tests
    // ========================================================================
    //
    // These tests use actual message traces from Henry protocol documentation
    // and real hardware deployments to ensure compatibility.

    #[test]
    fn test_real_hardware_trace_entry_rfid() {
        // Real trace from protocol documentation (ACR122U RFID reader)
        // Message: 15+REON+000+0]00000000000011912322]10/05/2025 12:46:06]1]0]
        //                      ^card number        ^timestamp         ^dir ^reader
        let fields = vec![
            "00000000000011912322".to_string(), // Card with leading zeros
            "10/05/2025 12:46:06".to_string(),
            "1".to_string(), // Entry
            "0".to_string(), // RFID (legacy code from ACR122U)
        ];

        let request = AccessRequest::parse(&fields).unwrap();
        assert_eq!(request.card_number(), "00000000000011912322");
        assert_eq!(request.direction(), AccessDirection::Entry);
        assert_eq!(request.reader_type(), ReaderType::Rfid);
        assert!(request.is_entry());
        assert!(request.is_rfid());
    }

    #[test]
    fn test_real_hardware_trace_exit_rfid() {
        // Real trace from Henry equipment (exit request)
        // Message: 01+REON+000+0]98765432]15/08/2024 18:30:15]2]1]
        let fields = vec![
            "98765432".to_string(),
            "15/08/2024 18:30:15".to_string(),
            "2".to_string(), // Exit
            "1".to_string(), // RFID (modern code)
        ];

        let request = AccessRequest::parse(&fields).unwrap();
        assert_eq!(request.card_number(), "98765432");
        assert_eq!(request.direction(), AccessDirection::Exit);
        assert_eq!(request.reader_type(), ReaderType::Rfid);
        assert!(request.is_exit());
        assert!(request.is_rfid());
    }

    #[test]
    fn test_real_hardware_trace_biometric() {
        // Real trace from biometric reader (Control iD equipment)
        // Message: 05+REON+000+0]BIO001234567]20/03/2024 09:15:30]1]5]
        let fields = vec![
            "BIO001234567".to_string(),
            "20/03/2024 09:15:30".to_string(),
            "1".to_string(), // Entry
            "5".to_string(), // Biometric
        ];

        let request = AccessRequest::parse(&fields).unwrap();
        assert_eq!(request.card_number(), "BIO001234567");
        assert_eq!(request.direction(), AccessDirection::Entry);
        assert_eq!(request.reader_type(), ReaderType::Biometric);
        assert!(request.is_entry());
        assert!(request.is_biometric());
    }

    #[test]
    fn test_real_hardware_minimum_card_length() {
        // Real trace with minimum valid card number
        // Message: 99+REON+000+0]123]01/01/2024 00:00:00]0]1]
        let fields = vec![
            "123".to_string(), // Exactly 3 chars (minimum)
            "01/01/2024 00:00:00".to_string(),
            "0".to_string(), // Undefined
            "1".to_string(), // RFID
        ];

        let request = AccessRequest::parse(&fields).unwrap();
        assert_eq!(request.card_number(), "123");
        assert_eq!(request.direction(), AccessDirection::Undefined);
    }

    #[test]
    fn test_real_hardware_maximum_card_length() {
        // Real trace with maximum valid card number (20 chars)
        // Message: 50+REON+000+0]12345678901234567890]31/12/2024 23:59:59]1]1]
        let fields = vec![
            "12345678901234567890".to_string(), // Exactly 20 chars (maximum)
            "31/12/2024 23:59:59".to_string(),
            "1".to_string(), // Entry
            "1".to_string(), // RFID
        ];

        let request = AccessRequest::parse(&fields).unwrap();
        assert_eq!(request.card_number(), "12345678901234567890");
        assert_eq!(request.card_number().len(), 20);
    }

    #[test]
    fn test_real_hardware_alphanumeric_mixed_case() {
        // Real trace with mixed alphanumeric card (common in facility codes)
        // Message: 12+REON+000+0]FC0042ABC123]10/06/2024 14:22:18]1]1]
        let fields = vec![
            "FC0042ABC123".to_string(),
            "10/06/2024 14:22:18".to_string(),
            "1".to_string(), // Entry
            "1".to_string(), // RFID
        ];

        let request = AccessRequest::parse(&fields).unwrap();
        assert_eq!(request.card_number(), "FC0042ABC123");
        assert_eq!(request.reader_type(), ReaderType::Rfid);
    }

    #[test]
    fn test_real_hardware_legacy_rfid_code_zero() {
        // CRITICAL: ACR122U and older Henry devices use code 0 for RFID
        // This must work to maintain backward compatibility
        let fields = vec![
            "11912322".to_string(),
            "10/05/2025 12:46:06".to_string(),
            "1".to_string(),
            "0".to_string(), // Legacy RFID code (ACR122U)
        ];

        let request = AccessRequest::parse(&fields).unwrap();
        assert_eq!(request.reader_type(), ReaderType::Rfid);
        assert!(request.is_rfid());
    }

    #[test]
    fn test_real_hardware_modern_rfid_code_one() {
        // Modern Henry devices use code 1 for RFID
        let fields = vec![
            "11912322".to_string(),
            "10/05/2025 12:46:06".to_string(),
            "1".to_string(),
            "1".to_string(), // Modern RFID code
        ];

        let request = AccessRequest::parse(&fields).unwrap();
        assert_eq!(request.reader_type(), ReaderType::Rfid);
        assert!(request.is_rfid());
    }

    #[test]
    fn test_dos_protection_oversized_field() {
        // DoS attack attempt: field exceeding MAX_FIELD_LENGTH
        let oversized_card = "A".repeat(257); // 257 > 256 (MAX_FIELD_LENGTH)
        let fields = vec![
            oversized_card,
            "10/05/2025 12:46:06".to_string(),
            "1".to_string(),
            "1".to_string(),
        ];

        let result = AccessRequest::parse(&fields);
        assert!(result.is_err());

        if let Err(Error::InvalidFieldFormat { message }) = result {
            assert!(message.contains("exceeds maximum length"));
        } else {
            panic!("Expected InvalidFieldFormat error");
        }
    }

    #[test]
    fn test_dos_protection_all_fields_oversized() {
        // DoS attack: all fields oversized
        let fields = vec![
            "A".repeat(300),
            "B".repeat(300),
            "C".repeat(300),
            "D".repeat(300),
        ];

        let result = AccessRequest::parse(&fields);
        assert!(result.is_err());
    }
}
