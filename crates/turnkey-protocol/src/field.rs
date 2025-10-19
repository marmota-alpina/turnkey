use crate::validation::validate_field;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use std::fmt;
use std::str::FromStr;
use turnkey_core::{Error, Result};

/// Type-safe wrapper for protocol message field data
///
/// Ensures field content does not contain protocol delimiters that could
/// cause protocol injection vulnerabilities. All fields are validated at
/// construction time, providing compile-time safety guarantees.
///
/// # Protocol Safety
///
/// The Henry protocol uses specific characters as delimiters:
/// - `]` - Field separator
/// - `+` - Device/command separator
/// - `[` - Subfield separator
///
/// Fields containing these characters would break protocol parsing and could
/// lead to injection vulnerabilities. FieldData enforces this invariant at
/// the type level.
///
/// # Example
/// ```
/// use turnkey_protocol::FieldData;
///
/// // Valid field creation
/// let field = FieldData::new("12345678".to_string()).unwrap();
/// assert_eq!(field.as_str(), "12345678");
///
/// // Invalid field with delimiter is rejected
/// let result = FieldData::new("field]with]delimiter".to_string());
/// assert!(result.is_err());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldData(String);

impl FieldData {
    /// Create new field data with validation
    ///
    /// Validates that the field does not contain any protocol delimiters
    /// that could cause parsing errors or injection vulnerabilities.
    ///
    /// # Errors
    /// Returns `Error::InvalidFieldFormat` if field contains reserved delimiters (], +, or [)
    ///
    /// # Example
    /// ```
    /// use turnkey_protocol::FieldData;
    ///
    /// let field = FieldData::new("valid_field".to_string()).unwrap();
    /// assert_eq!(field.as_str(), "valid_field");
    /// ```
    pub fn new(value: String) -> Result<Self> {
        validate_field(&value)?;
        Ok(FieldData(value))
    }

    /// Get field data as string slice
    ///
    /// # Example
    /// ```
    /// use turnkey_protocol::FieldData;
    ///
    /// let field = FieldData::new("test".to_string()).unwrap();
    /// assert_eq!(field.as_str(), "test");
    /// ```
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Convert field data into owned String
    ///
    /// # Example
    /// ```
    /// use turnkey_protocol::FieldData;
    ///
    /// let field = FieldData::new("test".to_string()).unwrap();
    /// let owned: String = field.into_string();
    /// assert_eq!(owned, "test");
    /// ```
    pub fn into_string(self) -> String {
        self.0
    }

    /// Create field data from trusted source without validation
    ///
    /// # Safety
    ///
    /// This function bypasses validation and should only be used when you are
    /// absolutely certain the value does not contain protocol delimiters.
    /// Invalid usage can lead to protocol injection vulnerabilities.
    ///
    /// Only use this for:
    /// - Constants known at compile time to be valid
    /// - Values already validated through other means
    /// - Internal protocol implementation where safety is guaranteed
    ///
    /// # Example
    /// ```
    /// use turnkey_protocol::FieldData;
    ///
    /// // Safe: We know this constant is valid
    /// let field = unsafe { FieldData::new_unchecked("CONSTANT".to_string()) };
    /// assert_eq!(field.as_str(), "CONSTANT");
    /// ```
    pub unsafe fn new_unchecked(value: String) -> Self {
        FieldData(value)
    }
}

impl FromStr for FieldData {
    type Err = Error;

    /// Parse a string slice into FieldData with validation
    ///
    /// # Errors
    /// Returns `Error::InvalidFieldFormat` if field contains reserved delimiters
    ///
    /// # Example
    /// ```
    /// use turnkey_protocol::FieldData;
    /// use std::str::FromStr;
    ///
    /// let field = FieldData::from_str("12345678").unwrap();
    /// assert_eq!(field.as_str(), "12345678");
    ///
    /// let invalid = FieldData::from_str("bad]field");
    /// assert!(invalid.is_err());
    /// ```
    fn from_str(s: &str) -> Result<Self> {
        Self::new(s.to_string())
    }
}

impl TryFrom<&str> for FieldData {
    type Error = Error;

    /// Try to create FieldData from a string slice with validation
    ///
    /// This provides a more ergonomic API for creating FieldData from string literals
    /// and borrowed strings, following Rust standard library patterns.
    ///
    /// # Errors
    /// Returns `Error::InvalidFieldFormat` if field contains reserved delimiters
    ///
    /// # Example
    /// ```
    /// use turnkey_protocol::FieldData;
    /// use std::convert::TryFrom;
    ///
    /// // Using try_from
    /// let field = FieldData::try_from("12345678").unwrap();
    /// assert_eq!(field.as_str(), "12345678");
    ///
    /// // Using try_into with type annotation
    /// let field: FieldData = "test_value".try_into().unwrap();
    /// assert_eq!(field.as_str(), "test_value");
    ///
    /// // Invalid field is rejected
    /// let invalid = FieldData::try_from("bad]field");
    /// assert!(invalid.is_err());
    /// ```
    fn try_from(s: &str) -> Result<Self> {
        Self::new(s.to_string())
    }
}

impl fmt::Display for FieldData {
    /// Display the field data as a string
    ///
    /// # Example
    /// ```
    /// use turnkey_protocol::FieldData;
    ///
    /// let field = FieldData::new("test".to_string()).unwrap();
    /// assert_eq!(format!("{}", field), "test");
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// Implement AsRef<str> for convenient conversion to &str
impl AsRef<str> for FieldData {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_field_creation() {
        let field = FieldData::new("12345678".to_string()).unwrap();
        assert_eq!(field.as_str(), "12345678");

        let field = FieldData::new("10/05/2025 12:46:06".to_string()).unwrap();
        assert_eq!(field.as_str(), "10/05/2025 12:46:06");

        let field = FieldData::new("Acesso liberado".to_string()).unwrap();
        assert_eq!(field.as_str(), "Acesso liberado");
    }

    #[test]
    fn test_reject_field_delimiter() {
        let result = FieldData::new("field]with]delimiter".to_string());
        assert!(result.is_err());

        if let Err(Error::InvalidFieldFormat { message }) = result {
            assert!(message.contains("reserved protocol delimiters"));
            assert!(message.contains("]"));
        } else {
            panic!("Expected InvalidFieldFormat error");
        }
    }

    #[test]
    fn test_reject_device_delimiter() {
        let result = FieldData::new("field+with+plus".to_string());
        assert!(result.is_err());

        if let Err(Error::InvalidFieldFormat { message }) = result {
            assert!(message.contains("reserved protocol delimiters"));
            assert!(message.contains("+"));
        } else {
            panic!("Expected InvalidFieldFormat error");
        }
    }

    #[test]
    fn test_reject_subfield_delimiter() {
        let result = FieldData::new("field[with[bracket".to_string());
        assert!(result.is_err());

        if let Err(Error::InvalidFieldFormat { message }) = result {
            assert!(message.contains("reserved protocol delimiters"));
            assert!(message.contains("["));
        } else {
            panic!("Expected InvalidFieldFormat error");
        }
    }

    #[test]
    fn test_from_str_valid() {
        let field = FieldData::from_str("test_value").unwrap();
        assert_eq!(field.as_str(), "test_value");
    }

    #[test]
    fn test_from_str_invalid() {
        let result = FieldData::from_str("invalid]value");
        assert!(result.is_err());
    }

    #[test]
    fn test_display_trait() {
        let field = FieldData::new("display_test".to_string()).unwrap();
        assert_eq!(format!("{}", field), "display_test");
    }

    #[test]
    fn test_as_ref() {
        let field = FieldData::new("reference".to_string()).unwrap();
        let reference: &str = field.as_ref();
        assert_eq!(reference, "reference");
    }

    #[test]
    fn test_into_string() {
        let field = FieldData::new("owned".to_string()).unwrap();
        let owned = field.into_string();
        assert_eq!(owned, "owned");
    }

    #[test]
    fn test_clone() {
        let field1 = FieldData::new("clone_me".to_string()).unwrap();
        let field2 = field1.clone();
        assert_eq!(field1, field2);
        assert_eq!(field1.as_str(), field2.as_str());
    }

    #[test]
    fn test_debug() {
        let field = FieldData::new("debug_test".to_string()).unwrap();
        let debug_str = format!("{:?}", field);
        assert!(debug_str.contains("debug_test"));
    }

    #[test]
    fn test_equality() {
        let field1 = FieldData::new("equal".to_string()).unwrap();
        let field2 = FieldData::new("equal".to_string()).unwrap();
        let field3 = FieldData::new("different".to_string()).unwrap();

        assert_eq!(field1, field2);
        assert_ne!(field1, field3);
    }

    #[test]
    fn test_empty_field() {
        // Empty fields are valid in the protocol (used for optional parameters)
        let field = FieldData::new("".to_string()).unwrap();
        assert_eq!(field.as_str(), "");
    }

    #[test]
    fn test_numeric_field() {
        let field = FieldData::new("12345".to_string()).unwrap();
        assert_eq!(field.as_str(), "12345");
    }

    #[test]
    fn test_special_chars_allowed() {
        // These characters are allowed (not protocol delimiters)
        let field = FieldData::new("test-value_123!@#$%^&*()".to_string()).unwrap();
        assert_eq!(field.as_str(), "test-value_123!@#$%^&*()");
    }

    #[test]
    fn test_new_unchecked() {
        // Safe usage with known valid value
        let field = unsafe { FieldData::new_unchecked("valid".to_string()) };
        assert_eq!(field.as_str(), "valid");
    }

    #[test]
    fn test_new_unchecked_bypasses_validation() {
        // new_unchecked truly bypasses validation - this is why it's unsafe
        // The caller is responsible for ensuring the value is valid
        let field = unsafe { FieldData::new_unchecked("has]delimiter".to_string()) };
        assert_eq!(field.as_str(), "has]delimiter");

        // Note: This field would cause protocol errors if used in actual messages.
        // Only use new_unchecked with values you know are valid.
    }

    #[test]
    fn test_try_from_valid() {
        use std::convert::TryFrom;

        let field = FieldData::try_from("test_value").unwrap();
        assert_eq!(field.as_str(), "test_value");

        let field = FieldData::try_from("12345678").unwrap();
        assert_eq!(field.as_str(), "12345678");
    }

    #[test]
    fn test_try_from_invalid() {
        use std::convert::TryFrom;

        let result = FieldData::try_from("invalid]field");
        assert!(result.is_err());

        let result = FieldData::try_from("invalid+field");
        assert!(result.is_err());

        let result = FieldData::try_from("invalid[field");
        assert!(result.is_err());
    }

    #[test]
    fn test_try_into_with_type_annotation() {
        use std::convert::TryInto;

        let field: Result<FieldData> = "valid_field".try_into();
        assert!(field.is_ok());
        assert_eq!(field.unwrap().as_str(), "valid_field");

        let field: Result<FieldData> = "invalid]field".try_into();
        assert!(field.is_err());
    }

    #[test]
    fn test_try_from_empty_string() {
        use std::convert::TryFrom;

        let field = FieldData::try_from("").unwrap();
        assert_eq!(field.as_str(), "");
    }
}
