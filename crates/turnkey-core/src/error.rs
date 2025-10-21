use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    // Protocol errors
    #[error("Invalid message format: {message}")]
    InvalidMessageFormat { message: String },

    #[error("Invalid command code: {code}")]
    InvalidCommandCode { code: String },

    #[error("Checksum mismatch: expected {expected}, got {actual}. {context}")]
    ChecksumMismatch {
        expected: String,
        actual: String,
        context: String,
    },

    #[error("Missing required field: {0}")]
    MissingField(String),

    // Hardware errors
    #[error("Device not found: {device_type} at {location}")]
    DeviceNotFound {
        device_type: String,
        location: String,
    },

    #[error("Device connection failed for {device}: {reason}")]
    ConnectionFailed { device: String, reason: String },

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

    #[error("Invalid field format: {message}")]
    InvalidFieldFormat { message: String },

    #[error("Invalid state transition from {from} to {to}")]
    InvalidStateTransition { from: String, to: String },

    #[error("Frame too large: {size} bytes exceeds maximum {max_size} bytes")]
    FrameTooLarge { size: usize, max_size: usize },

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
