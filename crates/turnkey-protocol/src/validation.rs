//! Field validation utilities for Henry protocol safety.
//!
//! This module provides validation functions to ensure field data does not
//! contain reserved protocol delimiters that could cause protocol injection
//! vulnerabilities or parsing errors.
//!
//! # Protocol Delimiters
//!
//! The Henry protocol uses three reserved delimiters:
//! - `]` - Field delimiter (separates data fields)
//! - `+` - Device/command delimiter (separates message components)
//! - `[` - Subfield delimiter (separates nested data)
//!
//! # Security
//!
//! All validation functions enforce strict delimiter checking to prevent:
//! - **Protocol injection attacks**: Malicious data embedding control characters
//! - **Message corruption**: Embedded delimiters breaking frame parsing
//! - **Parser state confusion**: Invalid state transitions in stream parser
//!
//! # Design Decision: Error Message Verbosity
//!
//! **IMPORTANT**: Error messages in this module include complete field content
//! without sanitization. This is an intentional architectural decision because:
//!
//! - **Turnkey is an emulator** for development and testing of protocol integrations
//! - **Production systems** use proprietary embedded hardware, not this emulator
//! - **Test data only** - this code processes synthetic data, not real user PII
//! - **Developer productivity** - complete error messages accelerate debugging
//!
//! ## Example Error Messages
//!
//! When validation fails, you will see complete, unredacted error messages:
//!
//! ```text
//! // Protocol delimiter detected
//! Error: Field 'user]admin' contains reserved protocol delimiters (], +, or [)
//!
//! // Field too long
//! Error: Field 2 exceeds maximum length 256 (got 300 bytes)
//!
//! // Invalid card number length
//! Error: Card number length must be 3-20 characters, got 2 characters
//! ```
//!
//! These messages expose the **complete field content** without truncation or
//! sanitization. This is intentional to help developers debug protocol issues
//! quickly during development and testing.
//!
//! If adapting this code for production use with real user data, implement
//! appropriate field sanitization to comply with privacy regulations (LGPD/GDPR).
//!
//! # Examples
//!
//! ```
//! use turnkey_protocol::validate_field;
//!
//! // Valid field passes
//! assert!(validate_field("valid_data_123").is_ok());
//! assert!(validate_field("NORMAL-TEXT").is_ok());
//!
//! // Fields with reserved delimiters fail
//! assert!(validate_field("invalid]data").is_err());  // field delimiter
//! assert!(validate_field("bad+input").is_err());     // device delimiter
//! assert!(validate_field("wrong[value").is_err());   // subfield delimiter
//! ```

use turnkey_core::{Error, Result, constants::*};

/// Validate field value for protocol safety
///
/// Ensures field does not contain reserved protocol delimiters that could
/// cause protocol injection vulnerabilities.
///
/// # Error Messages and Data Exposure
///
/// NOTE: This function includes the full field content in error messages
/// to facilitate debugging during development and testing. This is an
/// intentional design decision for the following reasons:
///
/// 1. **Emulator Context**: Turnkey is an access control emulator for
///    development and testing. In production deployments, actual hardware
///    turnstiles use proprietary embedded systems, not this emulator.
///
/// 2. **Developer Experience**: Complete error messages significantly improve
///    debugging efficiency when testing protocol integrations.
///
/// 3. **Test Data**: This emulator processes test data, not real user PII
///    (Personally Identifiable Information) in production scenarios.
///
/// If this code is adapted for production use with real user data, consider
/// implementing field sanitization (e.g., truncating to first 8 characters)
/// to comply with LGPD/GDPR requirements.
///
/// # Example Error Message
///
/// When a field contains protocol delimiters, the complete field is exposed:
///
/// ```text
/// Error: Field 'user]admin' contains reserved protocol delimiters (], +, or [)
/// ```
///
/// This shows the **exact field content** including the problematic delimiter,
/// making it immediately clear what needs to be fixed.
///
/// # Errors
///
/// Returns `Error::InvalidFieldFormat` if field contains protocol delimiters
pub fn validate_field(field: &str) -> Result<()> {
    if field.contains(DELIMITER_FIELD)
        || field.contains(DELIMITER_DEVICE)
        || field.contains(DELIMITER_SUBFIELD)
    {
        return Err(Error::InvalidFieldFormat {
            message: format!(
                "Field '{}' contains reserved protocol delimiters (], +, or [)",
                field
            ),
        });
    }
    Ok(())
}

/// Validate field lengths to prevent DoS attacks.
///
/// Checks that all fields up to `count` do not exceed the maximum
/// allowed length. This prevents memory exhaustion attacks from
/// maliciously crafted oversized fields.
///
/// # Arguments
///
/// * `fields` - The fields slice to validate
/// * `count` - Number of fields to validate from the start
///
/// # Returns
///
/// Returns `Ok(())` if all fields are within limits.
///
/// # Errors
///
/// Returns `Error::InvalidFieldFormat` if any field exceeds `MAX_FIELD_LENGTH`.
///
/// # Example Error Message
///
/// When a field exceeds the maximum length:
///
/// ```text
/// Error: Field 2 exceeds maximum length 256 (got 300 bytes)
/// ```
///
/// This clearly indicates **which field** (by index) exceeded the limit and
/// by **how much**, helping developers quickly identify problematic inputs.
///
/// # Examples
///
/// ```
/// use turnkey_protocol::validation::validate_field_lengths;
///
/// let fields = vec!["short".to_string(), "also short".to_string()];
/// assert!(validate_field_lengths(&fields, 2).is_ok());
///
/// let oversized = vec!["A".repeat(300)];
/// assert!(validate_field_lengths(&oversized, 1).is_err());
/// ```
pub fn validate_field_lengths(fields: &[String], count: usize) -> Result<()> {
    for (i, field) in fields.iter().take(count).enumerate() {
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
    Ok(())
}

/// Validate card number according to protocol specification.
///
/// Card numbers must be between 3-20 characters per the Henry protocol
/// and must not contain protocol delimiters (], +, [) which could cause
/// protocol injection attacks.
///
/// Empty strings are treated as valid (representing no card).
///
/// # Arguments
///
/// * `card` - The card number string to validate
///
/// # Returns
///
/// Returns `Ok(&str)` with a reference to the validated card number.
///
/// # Errors
///
/// Returns error if:
/// - Card number contains protocol delimiters (], +, [)
/// - Card number length is not in the range [3, 20] (excluding empty strings)
///
/// # Example Error Messages
///
/// When validation fails, complete card number is shown:
///
/// ```text
/// // Delimiter detected
/// Error: Field '1234]567' contains reserved protocol delimiters (], +, or [)
///
/// // Too short
/// Error: Card number length must be 3-20 characters, got 2 characters
///
/// // Too long
/// Error: Card number length must be 3-20 characters, got 21 characters
/// ```
///
/// These messages show the **exact card number** and **what's wrong** with it,
/// making it trivial to fix validation issues during development.
///
/// # Examples
///
/// ```
/// use turnkey_protocol::validation::validate_card_number;
///
/// assert!(validate_card_number("").is_ok());
/// assert!(validate_card_number("123").is_ok());
/// assert!(validate_card_number("12345678901234567890").is_ok());
/// assert!(validate_card_number("12").is_err()); // Too short
/// assert!(validate_card_number("123456789012345678901").is_err()); // Too long
/// assert!(validate_card_number("1234]567").is_err()); // Contains delimiter
/// ```
///
/// # Protocol Reference
///
/// Henry Protocol Section 4.1 - Card numbers must be alphanumeric
/// without control characters.
pub fn validate_card_number(card: &str) -> Result<&str> {
    if card.is_empty() {
        return Ok("");
    }

    // Check for protocol delimiters first (security-critical)
    validate_field(card)?;

    let len = card.len();
    if !(MIN_CARD_LENGTH..=MAX_CARD_LENGTH).contains(&len) {
        return Err(Error::InvalidCardFormat(format!(
            "Card number length must be {}-{} characters, got {} characters",
            MIN_CARD_LENGTH, MAX_CARD_LENGTH, len
        )));
    }

    Ok(card)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests for validate_field_lengths

    #[test]
    fn test_validate_field_lengths_ok() {
        let fields = vec![
            "short".to_string(),
            "medium length field".to_string(),
            "x".repeat(256), // Exactly at limit
        ];
        assert!(validate_field_lengths(&fields, 3).is_ok());
    }

    #[test]
    fn test_validate_field_lengths_exceeds_limit() {
        let fields = vec!["A".repeat(257)]; // 257 > 256
        let result = validate_field_lengths(&fields, 1);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_field_lengths_multiple_oversized() {
        let fields = vec!["A".repeat(300), "B".repeat(300), "C".repeat(300)];
        let result = validate_field_lengths(&fields, 3);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_field_lengths_partial_check() {
        // Only validates first 2 fields, third is oversized but not checked
        let fields = vec![
            "short".to_string(),
            "also short".to_string(),
            "X".repeat(300), // Oversized but not checked
        ];
        assert!(validate_field_lengths(&fields, 2).is_ok());
    }

    // Tests for validate_card_number

    #[test]
    fn test_validate_card_number_empty() {
        assert_eq!(validate_card_number("").unwrap(), "");
    }

    #[test]
    fn test_validate_card_number_min_length() {
        assert_eq!(validate_card_number("123").unwrap(), "123");
    }

    #[test]
    fn test_validate_card_number_max_length() {
        let card = "12345678901234567890"; // 20 chars
        assert_eq!(validate_card_number(card).unwrap(), card);
    }

    #[test]
    fn test_validate_card_number_with_delimiter() {
        // Card numbers containing protocol delimiters should be rejected
        assert!(validate_card_number("1234]567").is_err());
        assert!(validate_card_number("1234+567").is_err());
        assert!(validate_card_number("1234[567").is_err());
    }

    #[test]
    fn test_validate_card_number_too_short() {
        let result = validate_card_number("12"); // 2 chars
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_card_number_too_long() {
        let result = validate_card_number("123456789012345678901"); // 21 chars
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_card_number_valid_range() {
        for len in MIN_CARD_LENGTH..=MAX_CARD_LENGTH {
            let card = "1".repeat(len);
            assert!(validate_card_number(&card).is_ok());
        }
    }

    #[test]
    fn test_validate_card_number_alphanumeric() {
        assert!(validate_card_number("ABC123XYZ").is_ok());
        assert!(validate_card_number("USER-001").is_ok());
    }
}
