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
/// # Errors
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
