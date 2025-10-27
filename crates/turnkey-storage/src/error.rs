use thiserror::Error;

/// Storage-specific error types for the Turnkey access control system.
///
/// These errors represent failures in database operations, validation,
/// and data integrity checks during offline validation.
#[derive(Debug, Error)]
pub enum StorageError {
    /// Database connection or query execution failed
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    /// Migration execution failed
    #[error("Migration error: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),

    /// Entity not found in database
    #[error("Entity not found: {entity_type} with {field}={value}")]
    NotFound {
        entity_type: String,
        field: String,
        value: String,
    },

    /// Data validation failed
    #[error("Validation error: {0}")]
    Validation(String),

    /// Date/time parsing or formatting error
    #[error("DateTime error: {0}")]
    DateTime(String),

    /// Referential integrity violation
    #[error("Referential integrity error: {0}")]
    ReferentialIntegrity(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Generic internal error
    #[error("Internal error: {0}")]
    Internal(String),
}

/// Specialized result type for storage operations
pub type StorageResult<T> = Result<T, StorageError>;
