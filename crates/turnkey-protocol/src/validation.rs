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
