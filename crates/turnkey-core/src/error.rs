use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    // Protocol errors
    #[error("Invalid message format: {0}")]
    InvalidMessageFormat(String),

    #[error("Invalid command code: {0}")]
    InvalidCommandCode(String),

    #[error("Checksum mismatch: expected {expected}, got {actual}")]
    ChecksumMismatch { expected: String, actual: String },

    #[error("Missing required field: {0}")]
    MissingField(String),

    // Hardware errors
    #[error("Device not found: {0}")]
    DeviceNotFound(String),

    #[error("Device connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Hardware operation failed: {0}")]
    HardwareError(String),

    #[error("USB error: {0}")]
    UsbError(String),

    #[error("PCSC error: {0}")]
    PcscError(String),

    // Database errors
    #[error("Database error: {0}")]
    Database(String),

    #[error("Record not found: {0}")]
    RecordNotFound(String),

    // Validation errors
    #[error("Validation timeout")]
    ValidationTimeout,

    #[error("Access denied: {reason}")]
    AccessDenied { reason: String },

    #[error("Invalid card format: {0}")]
    InvalidCardFormat(String),

    #[error("Invalid state transition from {from} to {to}")]
    InvalidStateTransition { from: String, to: String },

    // IO errors
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    // Configuration errors
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Missing configuration key: {0}")]
    MissingConfig(String),
}

pub type Result<T> = std::result::Result<T, Error>;
