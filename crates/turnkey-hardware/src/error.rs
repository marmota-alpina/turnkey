//! Error types for hardware operations.
//!
//! This module defines error types specific to hardware device operations,
//! covering various failure scenarios such as device disconnection, timeouts,
//! protocol errors, and unsupported operations.

/// Result type alias for hardware operations.
pub type Result<T> = std::result::Result<T, HardwareError>;

/// Errors that can occur during hardware device operations.
#[derive(Debug, thiserror::Error)]
pub enum HardwareError {
    /// Device is not connected or has been disconnected.
    #[error("Device disconnected: {device}")]
    Disconnected { device: String },

    /// Operation timed out after specified duration.
    #[error("Operation timeout after {duration_ms}ms")]
    Timeout { duration_ms: u64 },

    /// Operation is not supported by this device.
    #[error("Unsupported operation: {operation}")]
    Unsupported { operation: String },

    /// Device communication error.
    #[error("Communication error: {message}")]
    CommunicationError { message: String },

    /// Invalid data received from device.
    #[error("Invalid data: {message}")]
    InvalidData { message: String },

    /// Device initialization failed.
    #[error("Initialization failed: {message}")]
    InitializationFailed { message: String },

    /// Device configuration error.
    #[error("Configuration error: {message}")]
    ConfigurationError { message: String },

    /// Card reading error.
    #[error("Card read error: {message}")]
    CardReadError { message: String },

    /// Biometric capture error.
    #[error("Biometric capture error: {message}")]
    BiometricCaptureError { message: String },

    /// Biometric verification error.
    #[error("Biometric verification error: {message}")]
    BiometricVerificationError { message: String },

    /// Generic I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Generic error with custom message.
    #[error("{0}")]
    Other(String),
}

impl HardwareError {
    /// Create a new disconnected error.
    pub fn disconnected(device: impl Into<String>) -> Self {
        Self::Disconnected {
            device: device.into(),
        }
    }

    /// Create a new timeout error.
    pub fn timeout(duration_ms: u64) -> Self {
        Self::Timeout { duration_ms }
    }

    /// Create a new unsupported operation error.
    pub fn unsupported(operation: impl Into<String>) -> Self {
        Self::Unsupported {
            operation: operation.into(),
        }
    }

    /// Create a new communication error.
    pub fn communication(message: impl Into<String>) -> Self {
        Self::CommunicationError {
            message: message.into(),
        }
    }

    /// Create a new invalid data error.
    pub fn invalid_data(message: impl Into<String>) -> Self {
        Self::InvalidData {
            message: message.into(),
        }
    }

    /// Create a new initialization failed error.
    pub fn initialization_failed(message: impl Into<String>) -> Self {
        Self::InitializationFailed {
            message: message.into(),
        }
    }

    /// Create a new configuration error.
    pub fn configuration(message: impl Into<String>) -> Self {
        Self::ConfigurationError {
            message: message.into(),
        }
    }

    /// Create a new card read error.
    pub fn card_read(message: impl Into<String>) -> Self {
        Self::CardReadError {
            message: message.into(),
        }
    }

    /// Create a new biometric capture error.
    pub fn biometric_capture(message: impl Into<String>) -> Self {
        Self::BiometricCaptureError {
            message: message.into(),
        }
    }

    /// Create a new biometric verification error.
    pub fn biometric_verification(message: impl Into<String>) -> Self {
        Self::BiometricVerificationError {
            message: message.into(),
        }
    }

    /// Create a generic error with custom message.
    pub fn other(message: impl Into<String>) -> Self {
        Self::Other(message.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_disconnected_error() {
        let error = HardwareError::disconnected("ACR122U");
        assert!(matches!(error, HardwareError::Disconnected { .. }));
        assert_eq!(error.to_string(), "Device disconnected: ACR122U");
    }

    #[test]
    fn test_timeout_error() {
        let error = HardwareError::timeout(3000);
        assert!(matches!(error, HardwareError::Timeout { .. }));
        assert_eq!(error.to_string(), "Operation timeout after 3000ms");
    }

    #[test]
    fn test_unsupported_error() {
        let error = HardwareError::unsupported("set_led");
        assert!(matches!(error, HardwareError::Unsupported { .. }));
        assert_eq!(error.to_string(), "Unsupported operation: set_led");
    }

    #[test]
    fn test_communication_error() {
        let error = HardwareError::communication("Serial port closed");
        assert!(matches!(error, HardwareError::CommunicationError { .. }));
        assert_eq!(error.to_string(), "Communication error: Serial port closed");
    }

    #[test]
    fn test_invalid_data_error() {
        let error = HardwareError::invalid_data("Invalid UID format");
        assert!(matches!(error, HardwareError::InvalidData { .. }));
        assert_eq!(error.to_string(), "Invalid data: Invalid UID format");
    }

    #[test]
    fn test_error_display() {
        let errors = vec![
            HardwareError::disconnected("Device1"),
            HardwareError::timeout(1000),
            HardwareError::unsupported("operation"),
        ];

        for error in errors {
            let _ = format!("{}", error);
            let _ = format!("{:?}", error);
        }
    }
}
