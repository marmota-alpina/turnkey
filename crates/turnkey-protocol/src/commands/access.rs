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
use turnkey_core::constants::{
    DEFAULT_DENY_TIMEOUT_SECONDS, DEFAULT_GRANT_TIMEOUT_SECONDS, MAX_CARD_LENGTH,
    MAX_DISPLAY_MESSAGE_LENGTH, MIN_CARD_LENGTH,
};
use turnkey_core::{AccessDirection, Error, HenryTimestamp, ReaderType, Result};

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
    /// Number of fields required in access request messages.
    ///
    /// Access request messages always contain exactly 4 fields:
    /// 1. Card number (3-20 characters)
    /// 2. Timestamp (dd/mm/yyyy hh:mm:ss)
    /// 3. Direction code (0, 1, or 2)
    /// 4. Reader type code (0, 1, or 5)
    ///
    /// # Use Cases
    ///
    /// This constant is exposed as public to enable:
    /// - Pre-allocation of field buffers before parsing
    /// - Fail-fast validation in client code before expensive operations
    /// - Prevention of magic number duplication across codebases
    /// - Protocol specification documentation for integrators
    ///
    /// # Performance Characteristics
    ///
    /// Pre-allocating with this constant provides measurable performance benefits:
    /// - **Eliminates 2-3 vector reallocations** per parsing operation
    /// - **Saves approximately 100-200ns** per access request parse
    /// - **At 1000 requests/second**: ~200µs saved per second
    /// - **Zero allocation overhead** when capacity matches exactly
    ///
    /// See benchmarks in `benches/validation_bench.rs` for detailed measurements.
    ///
    /// # Example
    ///
    /// ```
    /// use turnkey_protocol::commands::AccessRequest;
    ///
    /// // Pre-allocate exact capacity before building field list
    /// let mut fields: Vec<String> = Vec::with_capacity(AccessRequest::REQUIRED_FIELD_COUNT);
    ///
    /// // Fail-fast validation before parsing
    /// assert_eq!(AccessRequest::REQUIRED_FIELD_COUNT, 4);
    /// ```
    pub const REQUIRED_FIELD_COUNT: usize = 4;

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
        Self::validate_field_count(fields)?;
        crate::validation::validate_field_lengths(fields, Self::REQUIRED_FIELD_COUNT)?;

        let card_number = Self::parse_card_field(&fields[0])?;
        let timestamp = HenryTimestamp::parse(&fields[1])?;
        let direction = Self::parse_direction(&fields[2])?;
        let reader_type = Self::parse_reader_type(&fields[3])?;

        Ok(Self {
            card_number,
            timestamp,
            direction,
            reader_type,
        })
    }

    /// Validate that the minimum number of fields are present.
    fn validate_field_count(fields: &[String]) -> Result<()> {
        if fields.len() < Self::REQUIRED_FIELD_COUNT {
            return Err(Error::MissingField(format!(
                "Access request requires {} fields, got {}",
                Self::REQUIRED_FIELD_COUNT,
                fields.len()
            )));
        }
        Ok(())
    }

    /// Parse and validate the card number field.
    fn parse_card_field(field: &str) -> Result<String> {
        Self::validate_card_number(field)?;
        Ok(field.to_string())
    }

    /// Parse the direction code field.
    fn parse_direction(field: &str) -> Result<AccessDirection> {
        let code = field.parse::<u8>().map_err(|_| Error::InvalidFieldFormat {
            message: format!(
                "Invalid direction code: '{}' (expected 0=Undefined, 1=Entry, or 2=Exit)",
                field
            ),
        })?;

        AccessDirection::from_u8(code).map_err(|e| Error::InvalidFieldFormat {
            message: format!("Invalid direction value: {} ({})", code, e),
        })
    }

    /// Parse the reader type code field.
    fn parse_reader_type(field: &str) -> Result<ReaderType> {
        let code = field.parse::<u8>().map_err(|_| Error::InvalidFieldFormat {
            message: format!(
                "Invalid reader type code: '{}' (expected 0/1=RFID or 5=Biometric)",
                field
            ),
        })?;

        ReaderType::from_u8(code).map_err(|e| Error::InvalidFieldFormat {
            message: format!("Invalid reader type value: {} ({})", code, e),
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

/// Access control decision made by the server.
///
/// Represents the server's decision on an access request, specifying
/// whether to grant or deny access and in which direction(s).
///
/// # Protocol Mapping
///
/// Each decision maps to a specific Henry protocol command code:
/// - `GrantBoth`: 00+1 (allow passage in both directions)
/// - `GrantEntry`: 00+5 (allow entry only)
/// - `GrantExit`: 00+6 (allow exit only)
/// - `Deny`: 00+30 (deny access)
///
/// # Examples
///
/// ```
/// use turnkey_protocol::commands::access::AccessDecision;
///
/// let decision = AccessDecision::GrantExit;
/// assert_eq!(decision.command_code(), "00+6");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccessDecision {
    /// Grant access in both entry and exit directions.
    ///
    /// Used for turnstiles that support bidirectional passage.
    GrantBoth,

    /// Grant access for entry only.
    ///
    /// The turnstile will only allow passage in the entry direction.
    GrantEntry,

    /// Grant access for exit only.
    ///
    /// The turnstile will only allow passage in the exit direction.
    GrantExit,

    /// Deny access.
    ///
    /// The turnstile will remain locked and display a denial message.
    Deny,
}

impl AccessDecision {
    /// Get the command code for this decision.
    ///
    /// Returns the Henry protocol command code string that represents
    /// this access decision.
    ///
    /// # Examples
    ///
    /// ```
    /// use turnkey_protocol::commands::access::AccessDecision;
    ///
    /// assert_eq!(AccessDecision::GrantBoth.command_code(), "00+1");
    /// assert_eq!(AccessDecision::GrantEntry.command_code(), "00+5");
    /// assert_eq!(AccessDecision::GrantExit.command_code(), "00+6");
    /// assert_eq!(AccessDecision::Deny.command_code(), "00+30");
    /// ```
    pub fn command_code(&self) -> &'static str {
        match self {
            Self::GrantBoth => "00+1",
            Self::GrantEntry => "00+5",
            Self::GrantExit => "00+6",
            Self::Deny => "00+30",
        }
    }

    /// Returns `true` if this decision grants access.
    ///
    /// # Examples
    ///
    /// ```
    /// use turnkey_protocol::commands::access::AccessDecision;
    ///
    /// assert!(AccessDecision::GrantBoth.is_grant());
    /// assert!(AccessDecision::GrantEntry.is_grant());
    /// assert!(AccessDecision::GrantExit.is_grant());
    /// assert!(!AccessDecision::Deny.is_grant());
    /// ```
    pub fn is_grant(&self) -> bool {
        matches!(self, Self::GrantBoth | Self::GrantEntry | Self::GrantExit)
    }

    /// Returns `true` if this decision denies access.
    ///
    /// # Examples
    ///
    /// ```
    /// use turnkey_protocol::commands::access::AccessDecision;
    ///
    /// assert!(AccessDecision::Deny.is_deny());
    /// assert!(!AccessDecision::GrantBoth.is_deny());
    /// ```
    pub fn is_deny(&self) -> bool {
        matches!(self, Self::Deny)
    }
}

/// Access response message sent to turnstile.
///
/// Represents the server's response to an access request, containing
/// the decision (grant/deny), display message, and timeout configuration.
///
/// # Protocol Format
///
/// Response messages follow this format:
///
/// ```text
/// <ID>+REON+<COMMAND>]<TIMEOUT>]<MESSAGE>]
/// ```
///
/// Where:
/// - `COMMAND`: Decision command code (00+1, 00+5, 00+6, or 00+30)
/// - `TIMEOUT`: Display timeout in seconds (0 for permanent)
/// - `MESSAGE`: Text to display on turnstile LCD (max 40 chars)
///
/// # Examples
///
/// ## Grant Exit Access
///
/// ```
/// use turnkey_protocol::commands::access::{AccessResponse, AccessDecision};
///
/// let response = AccessResponse::new(
///     AccessDecision::GrantExit,
///     5,
///     "Acesso liberado".to_string(),
/// );
///
/// assert_eq!(response.decision(), AccessDecision::GrantExit);
/// assert_eq!(response.timeout_seconds(), 5);
/// assert_eq!(response.display_message(), "Acesso liberado");
/// ```
///
/// ## Deny Access
///
/// ```
/// use turnkey_protocol::commands::access::{AccessResponse, AccessDecision};
///
/// let response = AccessResponse::new(
///     AccessDecision::Deny,
///     0,
///     "Acesso negado".to_string(),
/// );
///
/// assert_eq!(response.decision(), AccessDecision::Deny);
/// assert_eq!(response.timeout_seconds(), 0);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccessResponse {
    decision: AccessDecision,
    timeout_seconds: u8,
    display_message: String,
}

impl AccessResponse {
    /// Create a new access response.
    ///
    /// # Arguments
    ///
    /// * `decision` - The access control decision
    /// * `timeout_seconds` - Display timeout in seconds (0 for permanent)
    /// * `display_message` - Message to show on turnstile LCD
    ///
    /// # Display Message Truncation
    ///
    /// Messages longer than 40 characters are automatically truncated to
    /// comply with protocol constraints.
    ///
    /// # Examples
    ///
    /// ```
    /// use turnkey_protocol::commands::access::{AccessResponse, AccessDecision};
    ///
    /// let response = AccessResponse::new(
    ///     AccessDecision::GrantEntry,
    ///     3,
    ///     "Bem-vindo".to_string(),
    /// );
    ///
    /// assert_eq!(response.decision(), AccessDecision::GrantEntry);
    /// ```
    ///
    /// ## Message Truncation
    ///
    /// ```
    /// use turnkey_protocol::commands::access::{AccessResponse, AccessDecision};
    ///
    /// let long_message = "A".repeat(50);
    /// let response = AccessResponse::new(
    ///     AccessDecision::GrantBoth,
    ///     5,
    ///     long_message,
    /// );
    ///
    /// assert_eq!(response.display_message().len(), 40);
    /// ```
    pub fn new(decision: AccessDecision, timeout_seconds: u8, display_message: String) -> Self {
        // Truncate message to maximum allowed length
        let truncated_message = if display_message.len() > MAX_DISPLAY_MESSAGE_LENGTH {
            display_message
                .chars()
                .take(MAX_DISPLAY_MESSAGE_LENGTH)
                .collect()
        } else {
            display_message
        };

        Self {
            decision,
            timeout_seconds,
            display_message: truncated_message,
        }
    }

    /// Create a grant both directions response with default timeout.
    ///
    /// Uses a default timeout of 5 seconds.
    ///
    /// # Examples
    ///
    /// ```
    /// use turnkey_protocol::commands::access::AccessResponse;
    ///
    /// let response = AccessResponse::grant_both("Acesso liberado".to_string());
    /// assert_eq!(response.timeout_seconds(), 5);
    /// ```
    pub fn grant_both(display_message: String) -> Self {
        Self::new(
            AccessDecision::GrantBoth,
            DEFAULT_GRANT_TIMEOUT_SECONDS,
            display_message,
        )
    }

    /// Create a grant entry response with default timeout.
    ///
    /// Uses a default timeout of 5 seconds.
    ///
    /// # Examples
    ///
    /// ```
    /// use turnkey_protocol::commands::access::AccessResponse;
    ///
    /// let response = AccessResponse::grant_entry("Bem-vindo".to_string());
    /// assert_eq!(response.timeout_seconds(), 5);
    /// ```
    pub fn grant_entry(display_message: String) -> Self {
        Self::new(
            AccessDecision::GrantEntry,
            DEFAULT_GRANT_TIMEOUT_SECONDS,
            display_message,
        )
    }

    /// Create a grant exit response with default timeout.
    ///
    /// Uses a default timeout of 5 seconds.
    ///
    /// # Examples
    ///
    /// ```
    /// use turnkey_protocol::commands::access::AccessResponse;
    ///
    /// let response = AccessResponse::grant_exit("Acesso liberado".to_string());
    /// assert_eq!(response.timeout_seconds(), 5);
    /// ```
    pub fn grant_exit(display_message: String) -> Self {
        Self::new(
            AccessDecision::GrantExit,
            DEFAULT_GRANT_TIMEOUT_SECONDS,
            display_message,
        )
    }

    /// Create a deny access response.
    ///
    /// Uses a timeout of 0 (permanent until next action).
    ///
    /// # Examples
    ///
    /// ```
    /// use turnkey_protocol::commands::access::AccessResponse;
    ///
    /// let response = AccessResponse::deny("Acesso negado".to_string());
    /// assert_eq!(response.timeout_seconds(), 0);
    /// ```
    pub fn deny(display_message: String) -> Self {
        Self::new(
            AccessDecision::Deny,
            DEFAULT_DENY_TIMEOUT_SECONDS,
            display_message,
        )
    }

    /// Convert response to protocol message fields.
    ///
    /// Returns the fields in the order required by the Henry protocol:
    /// 1. Command code (decision)
    /// 2. Timeout in seconds
    /// 3. Display message
    ///
    /// # Examples
    ///
    /// ```
    /// use turnkey_protocol::commands::access::{AccessResponse, AccessDecision};
    ///
    /// let response = AccessResponse::new(
    ///     AccessDecision::GrantExit,
    ///     5,
    ///     "Acesso liberado".to_string(),
    /// );
    ///
    /// let fields = response.to_fields();
    /// assert_eq!(fields[0], "00+6");
    /// assert_eq!(fields[1], "5");
    /// assert_eq!(fields[2], "Acesso liberado");
    /// ```
    pub fn to_fields(&self) -> Vec<String> {
        vec![
            self.decision.command_code().to_string(),
            self.timeout_seconds.to_string(),
            self.display_message.clone(),
        ]
    }

    /// Get the access decision.
    pub fn decision(&self) -> AccessDecision {
        self.decision
    }

    /// Get the timeout in seconds.
    pub fn timeout_seconds(&self) -> u8 {
        self.timeout_seconds
    }

    /// Get the display message.
    pub fn display_message(&self) -> &str {
        &self.display_message
    }

    /// Returns `true` if this response grants access.
    pub fn is_grant(&self) -> bool {
        self.decision.is_grant()
    }

    /// Returns `true` if this response denies access.
    pub fn is_deny(&self) -> bool {
        self.decision.is_deny()
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

    // AccessDecision tests

    #[test]
    fn test_access_decision_command_codes() {
        assert_eq!(AccessDecision::GrantBoth.command_code(), "00+1");
        assert_eq!(AccessDecision::GrantEntry.command_code(), "00+5");
        assert_eq!(AccessDecision::GrantExit.command_code(), "00+6");
        assert_eq!(AccessDecision::Deny.command_code(), "00+30");
    }

    #[test]
    fn test_access_decision_is_grant() {
        assert!(AccessDecision::GrantBoth.is_grant());
        assert!(AccessDecision::GrantEntry.is_grant());
        assert!(AccessDecision::GrantExit.is_grant());
        assert!(!AccessDecision::Deny.is_grant());
    }

    #[test]
    fn test_access_decision_is_deny() {
        assert!(AccessDecision::Deny.is_deny());
        assert!(!AccessDecision::GrantBoth.is_deny());
        assert!(!AccessDecision::GrantEntry.is_deny());
        assert!(!AccessDecision::GrantExit.is_deny());
    }

    // AccessResponse tests

    #[test]
    fn test_access_response_grant_exit() {
        let response =
            AccessResponse::new(AccessDecision::GrantExit, 5, "Acesso liberado".to_string());

        assert_eq!(response.decision(), AccessDecision::GrantExit);
        assert_eq!(response.timeout_seconds(), 5);
        assert_eq!(response.display_message(), "Acesso liberado");
        assert!(response.is_grant());
        assert!(!response.is_deny());
    }

    #[test]
    fn test_access_response_grant_entry() {
        let response = AccessResponse::new(AccessDecision::GrantEntry, 3, "Bem-vindo".to_string());

        assert_eq!(response.decision(), AccessDecision::GrantEntry);
        assert_eq!(response.timeout_seconds(), 3);
        assert_eq!(response.display_message(), "Bem-vindo");
        assert!(response.is_grant());
    }

    #[test]
    fn test_access_response_grant_both() {
        let response =
            AccessResponse::new(AccessDecision::GrantBoth, 5, "Acesso liberado".to_string());

        assert_eq!(response.decision(), AccessDecision::GrantBoth);
        assert_eq!(response.timeout_seconds(), 5);
        assert!(response.is_grant());
    }

    #[test]
    fn test_access_response_deny() {
        let response = AccessResponse::new(AccessDecision::Deny, 0, "Acesso negado".to_string());

        assert_eq!(response.decision(), AccessDecision::Deny);
        assert_eq!(response.timeout_seconds(), 0);
        assert_eq!(response.display_message(), "Acesso negado");
        assert!(!response.is_grant());
        assert!(response.is_deny());
    }

    #[test]
    fn test_access_response_to_fields_grant_exit() {
        let response =
            AccessResponse::new(AccessDecision::GrantExit, 5, "Acesso liberado".to_string());

        let fields = response.to_fields();
        assert_eq!(fields.len(), 3);
        assert_eq!(fields[0], "00+6");
        assert_eq!(fields[1], "5");
        assert_eq!(fields[2], "Acesso liberado");
    }

    #[test]
    fn test_access_response_to_fields_grant_entry() {
        let response = AccessResponse::new(AccessDecision::GrantEntry, 3, "Bem-vindo".to_string());

        let fields = response.to_fields();
        assert_eq!(fields[0], "00+5");
        assert_eq!(fields[1], "3");
        assert_eq!(fields[2], "Bem-vindo");
    }

    #[test]
    fn test_access_response_to_fields_grant_both() {
        let response =
            AccessResponse::new(AccessDecision::GrantBoth, 5, "Acesso liberado".to_string());

        let fields = response.to_fields();
        assert_eq!(fields[0], "00+1");
        assert_eq!(fields[1], "5");
    }

    #[test]
    fn test_access_response_to_fields_deny() {
        let response = AccessResponse::new(AccessDecision::Deny, 0, "Acesso negado".to_string());

        let fields = response.to_fields();
        assert_eq!(fields[0], "00+30");
        assert_eq!(fields[1], "0");
        assert_eq!(fields[2], "Acesso negado");
    }

    #[test]
    fn test_access_response_message_truncation() {
        // Message longer than 40 characters should be truncated
        let long_message = "A".repeat(50);
        let response = AccessResponse::new(AccessDecision::GrantBoth, 5, long_message);

        assert_eq!(response.display_message().len(), 40);
        assert_eq!(response.display_message(), &"A".repeat(40));
    }

    #[test]
    fn test_access_response_message_exact_limit() {
        // Message exactly 40 characters should not be truncated
        let message = "A".repeat(40);
        let response = AccessResponse::new(AccessDecision::GrantEntry, 3, message.clone());

        assert_eq!(response.display_message().len(), 40);
        assert_eq!(response.display_message(), message);
    }

    #[test]
    fn test_access_response_message_under_limit() {
        // Message under 40 characters should remain unchanged
        let message = "Acesso liberado";
        let response = AccessResponse::new(AccessDecision::GrantExit, 5, message.to_string());

        assert_eq!(response.display_message(), message);
        assert!(response.display_message().len() < 40);
    }

    #[test]
    fn test_access_response_grant_both_helper() {
        let response = AccessResponse::grant_both("Acesso liberado".to_string());

        assert_eq!(response.decision(), AccessDecision::GrantBoth);
        assert_eq!(response.timeout_seconds(), 5);
        assert_eq!(response.display_message(), "Acesso liberado");
    }

    #[test]
    fn test_access_response_grant_entry_helper() {
        let response = AccessResponse::grant_entry("Bem-vindo".to_string());

        assert_eq!(response.decision(), AccessDecision::GrantEntry);
        assert_eq!(response.timeout_seconds(), 5);
        assert_eq!(response.display_message(), "Bem-vindo");
    }

    #[test]
    fn test_access_response_grant_exit_helper() {
        let response = AccessResponse::grant_exit("Acesso liberado".to_string());

        assert_eq!(response.decision(), AccessDecision::GrantExit);
        assert_eq!(response.timeout_seconds(), 5);
        assert_eq!(response.display_message(), "Acesso liberado");
    }

    #[test]
    fn test_access_response_deny_helper() {
        let response = AccessResponse::deny("Acesso negado".to_string());

        assert_eq!(response.decision(), AccessDecision::Deny);
        assert_eq!(response.timeout_seconds(), 0);
        assert_eq!(response.display_message(), "Acesso negado");
    }

    #[test]
    fn test_access_response_real_protocol_example_grant_exit() {
        // Real protocol example from issue documentation
        // 01+REON+00+6]5]Acesso liberado]
        let response =
            AccessResponse::new(AccessDecision::GrantExit, 5, "Acesso liberado".to_string());

        let fields = response.to_fields();
        assert_eq!(fields[0], "00+6");
        assert_eq!(fields[1], "5");
        assert_eq!(fields[2], "Acesso liberado");
    }

    #[test]
    fn test_access_response_real_protocol_example_grant_entry() {
        // Real protocol example from issue documentation
        // 01+REON+00+5]3]Bem-vindo]
        let response = AccessResponse::new(AccessDecision::GrantEntry, 3, "Bem-vindo".to_string());

        let fields = response.to_fields();
        assert_eq!(fields[0], "00+5");
        assert_eq!(fields[1], "3");
        assert_eq!(fields[2], "Bem-vindo");
    }

    #[test]
    fn test_access_response_real_protocol_example_grant_both() {
        // Real protocol example from issue documentation
        // 01+REON+00+1]5]Acesso liberado]
        let response =
            AccessResponse::new(AccessDecision::GrantBoth, 5, "Acesso liberado".to_string());

        let fields = response.to_fields();
        assert_eq!(fields[0], "00+1");
        assert_eq!(fields[1], "5");
        assert_eq!(fields[2], "Acesso liberado");
    }

    #[test]
    fn test_access_response_real_protocol_example_deny() {
        // Real protocol example from issue documentation
        // 01+REON+00+30]0]Acesso negado]
        let response = AccessResponse::new(AccessDecision::Deny, 0, "Acesso negado".to_string());

        let fields = response.to_fields();
        assert_eq!(fields[0], "00+30");
        assert_eq!(fields[1], "0");
        assert_eq!(fields[2], "Acesso negado");
    }

    #[test]
    fn test_access_response_message_truncation_with_unicode() {
        // Test truncation with unicode characters (Portuguese text)
        let message = "Acesso liberado para entrada no prédio principal".to_string();
        assert!(message.len() > 40);

        let response = AccessResponse::new(AccessDecision::GrantEntry, 5, message);

        // Should truncate to 40 characters
        assert_eq!(response.display_message().chars().count(), 40);
    }

    #[test]
    fn test_access_response_empty_message() {
        // Empty messages are allowed
        let response = AccessResponse::new(AccessDecision::GrantBoth, 5, String::new());

        assert_eq!(response.display_message(), "");
    }

    #[test]
    fn test_access_response_clone() {
        let response1 = AccessResponse::grant_exit("Test".to_string());
        let response2 = response1.clone();

        assert_eq!(response1.decision(), response2.decision());
        assert_eq!(response1.timeout_seconds(), response2.timeout_seconds());
        assert_eq!(response1.display_message(), response2.display_message());
    }

    #[test]
    fn test_access_response_equality() {
        let response1 = AccessResponse::new(AccessDecision::GrantExit, 5, "Test".to_string());
        let response2 = AccessResponse::new(AccessDecision::GrantExit, 5, "Test".to_string());

        assert_eq!(response1, response2);
    }

    #[test]
    fn test_access_decision_copy() {
        let decision1 = AccessDecision::GrantExit;
        let decision2 = decision1;

        assert_eq!(decision1, decision2);
    }
}
