//! Core constants for the Henry protocol implementation.
//!
//! This module defines all protocol-level constants used throughout the Turnkey
//! access control emulator. These constants ensure consistent protocol compliance
//! and provide centralized configuration for protocol behavior.
//!
//! # Protocol Structure
//!
//! The Henry protocol uses a specific message format:
//!
//! ```text
//! <STX>ID+REON+COMMAND]FIELD1]FIELD2]...<ETX>
//! ```
//!
//! Where:
//! - `<STX>` - Start of text marker (0x02)
//! - `ID` - Device identifier (01-99, zero-padded)
//! - `+` - Device/command delimiter
//! - `REON` - Protocol identifier constant
//! - `COMMAND` - Command code (variable length)
//! - `]` - Field delimiter
//! - `<ETX>` - End of text marker (0x03)
//!
//! # Delimiter Semantics
//!
//! The protocol uses five distinct delimiters, each with specific meaning:
//!
//! | Delimiter | Name | Purpose | Example |
//! |-----------|------|---------|---------|
//! | `+` | DELIMITER_DEVICE | Separates device ID, protocol ID, and command | `15+REON+000+0` |
//! | `]` | DELIMITER_FIELD | Separates data fields | `field1]field2]field3` |
//! | `[` | DELIMITER_SUBFIELD | Separates nested subfields | `name[value` |
//! | `{` | DELIMITER_NESTED | Opens array/nested structure | `users{user1` |
//! | `}` | DELIMITER_ARRAY | Closes array/nested structure | `user1}` |
//!
//! # Usage
//!
//! Constants are organized by category for easy discovery:
//!
//! ```
//! use turnkey_core::constants::*;
//!
//! // Protocol identification
//! assert_eq!(PROTOCOL_ID, "REON");
//!
//! // Device ID validation
//! fn validate_device_id(id: u8) -> bool {
//!     id >= MIN_DEVICE_ID && id <= MAX_DEVICE_ID
//! }
//!
//! // Timeout configuration
//! use std::time::Duration;
//! let timeout = Duration::from_millis(DEFAULT_ONLINE_TIMEOUT);
//! ```
//!
//! # Protocol Compliance
//!
//! These constants are derived from the Henry protocol specification used by
//! Brazilian access control equipment (Primme Acesso, Argos, Primme SF).
//! Modifying these values may break protocol compatibility.

// ============================================================================
// Protocol Delimiters
// ============================================================================

/// Device and command separator in protocol messages.
///
/// Separates device ID, protocol identifier, and command components.
///
/// # Examples
///
/// ```
/// use turnkey_core::constants::DELIMITER_DEVICE;
///
/// let message = "15+REON+000+0";
/// let parts: Vec<&str> = message.split(DELIMITER_DEVICE).collect();
/// assert_eq!(parts, vec!["15", "REON", "000", "0"]);
/// ```
pub const DELIMITER_DEVICE: &str = "+";

/// Field separator in protocol messages.
///
/// Separates data fields within a message. Empty fields (consecutive `]]`)
/// have semantic meaning and must be preserved.
///
/// # Examples
///
/// ```
/// use turnkey_core::constants::DELIMITER_FIELD;
///
/// // Normal fields
/// let data = "card]timestamp]direction";
/// let fields: Vec<&str> = data.split(DELIMITER_FIELD).collect();
/// assert_eq!(fields.len(), 3);
///
/// // Empty field (valid in protocol)
/// let data_with_empty = "card]]timestamp";
/// let fields: Vec<&str> = data_with_empty.split(DELIMITER_FIELD).collect();
/// assert_eq!(fields, vec!["card", "", "timestamp"]);
/// ```
pub const DELIMITER_FIELD: &str = "]";

/// Subfield separator for nested data structures.
///
/// Used to separate components within a single field, typically for
/// structured data like user records or configuration entries.
///
/// # Examples
///
/// ```
/// use turnkey_core::constants::DELIMITER_SUBFIELD;
///
/// let user_data = "John[Doe[12345";
/// let parts: Vec<&str> = user_data.split(DELIMITER_SUBFIELD).collect();
/// assert_eq!(parts, vec!["John", "Doe", "12345"]);
/// ```
pub const DELIMITER_SUBFIELD: &str = "[";

/// Array closing delimiter.
///
/// Marks the end of an array or nested structure in protocol messages.
/// Must be paired with [`DELIMITER_NESTED`] which opens the structure.
pub const DELIMITER_ARRAY: &str = "}";

/// Array opening delimiter.
///
/// Marks the beginning of an array or nested structure in protocol messages.
/// Must be paired with [`DELIMITER_ARRAY`] which closes the structure.
///
/// # Examples
///
/// ```
/// use turnkey_core::constants::{DELIMITER_NESTED, DELIMITER_ARRAY};
///
/// let array_data = format!("users{}user1,user2{}", DELIMITER_NESTED, DELIMITER_ARRAY);
/// assert!(array_data.starts_with("users{"));
/// assert!(array_data.ends_with("}"));
/// ```
pub const DELIMITER_NESTED: &str = "{";

// ============================================================================
// Protocol Identification
// ============================================================================

/// Protocol identifier constant.
///
/// This constant appears in every Henry protocol message immediately after
/// the device ID. It identifies the message as belonging to the REON protocol
/// family used by Brazilian access control systems.
///
/// # Protocol Position
///
/// ```text
/// ID+REON+COMMAND...
///    ^^^^
///    Protocol identifier
/// ```
///
/// # Examples
///
/// ```
/// use turnkey_core::constants::PROTOCOL_ID;
///
/// let message = "15+REON+000+0]card_number]";
/// assert!(message.contains(PROTOCOL_ID));
///
/// // Validation
/// fn is_valid_protocol(msg: &str) -> bool {
///     msg.split('+').nth(1) == Some(PROTOCOL_ID)
/// }
/// ```
pub const PROTOCOL_ID: &str = "REON";

// ============================================================================
// Message Framing
// ============================================================================

/// Start of text marker (STX).
///
/// ASCII control character marking the beginning of a protocol message frame.
/// This is the STX character (0x02) from the C0 control codes.
///
/// # Protocol Position
///
/// ```text
/// <STX>15+REON+000+0]...<ETX>
/// ^^^^^
/// Start marker
/// ```
pub const START_BYTE: u8 = 0x02; // STX

/// End of text marker (ETX).
///
/// ASCII control character marking the end of a protocol message frame.
/// This is the ETX character (0x03) from the C0 control codes.
///
/// # Protocol Position
///
/// ```text
/// <STX>15+REON+000+0]...<ETX>
///                       ^^^^^
///                       End marker
/// ```
pub const END_BYTE: u8 = 0x03; // ETX

/// Frame overhead in bytes.
///
/// Total bytes used for frame markers ([`START_BYTE`] + [`END_BYTE`]).
/// This is used when calculating maximum payload capacity.
///
/// # Examples
///
/// ```
/// use turnkey_core::constants::FRAME_OVERHEAD;
///
/// const MAX_MESSAGE_SIZE: usize = 1024;
/// const MAX_PAYLOAD_SIZE: usize = MAX_MESSAGE_SIZE - FRAME_OVERHEAD;
/// ```
pub const FRAME_OVERHEAD: usize = 2;

// ============================================================================
// Message Structure Components
// ============================================================================

/// Device ID field length in protocol messages.
///
/// Device IDs are always zero-padded to exactly 2 digits (01-99).
/// This ensures consistent parsing and formatting.
///
/// # Examples
///
/// ```
/// use turnkey_core::constants::DEVICE_ID_LENGTH;
///
/// fn format_device_id(id: u8) -> String {
///     format!("{:0width$}", id, width = DEVICE_ID_LENGTH)
/// }
///
/// assert_eq!(format_device_id(5), "05");
/// assert_eq!(format_device_id(99), "99");
/// ```
pub const DEVICE_ID_LENGTH: usize = 2;

/// Protocol identifier field length.
///
/// The protocol identifier ([`PROTOCOL_ID`]) is always exactly 4 characters.
pub const PROTOCOL_ID_LENGTH: usize = 4; // "REON"

/// Number of base delimiters in message header.
///
/// Every message has exactly 2 delimiter characters (`+`) in the header
/// separating device ID, protocol ID, and command: `ID+REON+CMD`.
pub const BASE_DELIMITER_COUNT: usize = 2;

// ============================================================================
// Timeout Configuration
// ============================================================================

/// Default timeout for online validation requests (milliseconds).
///
/// When operating in online mode, the emulator waits this long for a
/// validation response from the server before falling back to offline
/// mode (if configured) or denying access.
///
/// # Value: 3000ms (3 seconds)
///
/// This provides a reasonable balance between user experience and
/// network reliability for typical LAN deployments.
///
/// # Examples
///
/// ```
/// use turnkey_core::constants::DEFAULT_ONLINE_TIMEOUT;
/// use std::time::Duration;
///
/// let timeout = Duration::from_millis(DEFAULT_ONLINE_TIMEOUT);
/// assert_eq!(timeout.as_secs(), 3);
/// ```
pub const DEFAULT_ONLINE_TIMEOUT: u64 = 3000;

/// Minimum allowed online validation timeout (milliseconds).
///
/// Values below this threshold may cause spurious timeouts even on
/// fast local networks due to processing overhead.
///
/// # Value: 500ms
pub const MIN_ONLINE_TIMEOUT: u64 = 500;

/// Maximum allowed online validation timeout (milliseconds).
///
/// Values above this threshold degrade user experience, as users must
/// wait too long for access denial feedback.
///
/// # Value: 10000ms (10 seconds)
pub const MAX_ONLINE_TIMEOUT: u64 = 10000;

// ============================================================================
// Card Format Constraints
// ============================================================================

/// Minimum card number length (characters).
///
/// Card numbers shorter than this are rejected as invalid per protocol rules.
///
/// # Value: 3 characters
pub const MIN_CARD_LENGTH: usize = 3;

/// Maximum card number length (characters).
///
/// Card numbers longer than this are rejected as invalid per protocol rules.
///
/// # Value: 20 characters
pub const MAX_CARD_LENGTH: usize = 20;

/// Padded card number length for protocol transmission.
///
/// When transmitting card numbers in certain protocol commands, they must
/// be zero-padded to this length for consistency.
///
/// # Value: 20 characters
///
/// # Examples
///
/// ```
/// use turnkey_core::constants::CARD_PADDED_LENGTH;
///
/// fn format_card_padded(card: &str) -> String {
///     format!("{:0width$}", card, width = CARD_PADDED_LENGTH)
/// }
/// ```
pub const CARD_PADDED_LENGTH: usize = 20;

// ============================================================================
// Device Identification
// ============================================================================

/// Minimum valid device ID.
///
/// Device IDs below this value are invalid per protocol specification.
///
/// # Value: 1
pub const MIN_DEVICE_ID: u8 = 1;

/// Maximum valid device ID.
///
/// Device IDs above this value are invalid per protocol specification.
/// With [`DEVICE_ID_LENGTH`] = 2, this allows 99 unique devices (01-99).
///
/// # Value: 99
pub const MAX_DEVICE_ID: u8 = 99;

// ============================================================================
// Display Configuration
// ============================================================================

/// Maximum length for display messages (characters).
///
/// Messages longer than this must be truncated or scrolled for display
/// on the turnstile's LCD screen. This limit is based on typical 2-line
/// LCD displays with 20 characters per line.
///
/// # Value: 40 characters (2 lines × 20 chars)
///
/// # Examples
///
/// ```
/// use turnkey_core::constants::MAX_DISPLAY_MESSAGE_LENGTH;
///
/// fn truncate_display_message(msg: &str) -> &str {
///     if msg.len() > MAX_DISPLAY_MESSAGE_LENGTH {
///         &msg[..MAX_DISPLAY_MESSAGE_LENGTH]
///     } else {
///         msg
///     }
/// }
/// ```
pub const MAX_DISPLAY_MESSAGE_LENGTH: usize = 40;

// ============================================================================
// Protocol Field Limits
// ============================================================================

/// Maximum length for any single protocol field (bytes).
///
/// This limit provides DoS protection by preventing unbounded memory allocation
/// while accommodating the longest legitimate fields in the Henry protocol.
///
/// # Value: 256 bytes
///
/// # Rationale
///
/// 1. **DoS Protection**: Prevents attackers from sending messages with extremely
///    long fields that could exhaust memory or processing resources.
///
/// 2. **Protocol Analysis**: Real-world Henry protocol deployments rarely exceed
///    100 bytes per field. The 256-byte limit provides 2.5x safety margin while
///    remaining practical.
///
/// 3. **Performance**: 256-byte comparison is cache-friendly on modern CPUs
///    (fits within L1 cache line which is typically 64-256 bytes).
///
/// # Note
///
/// This is an implementation-specific limit, not a protocol specification limit.
/// The Henry protocol does not define maximum field lengths explicitly.
///
/// # Examples
///
/// ```
/// use turnkey_core::constants::MAX_FIELD_LENGTH;
///
/// fn validate_field_length(field: &str) -> Result<(), String> {
///     if field.len() > MAX_FIELD_LENGTH {
///         Err(format!("Field exceeds maximum length of {} bytes", MAX_FIELD_LENGTH))
///     } else {
///         Ok(())
///     }
/// }
/// ```
pub const MAX_FIELD_LENGTH: usize = 256;

// ============================================================================
// Response Display Timeouts
// ============================================================================

/// Default display duration for access grant messages (seconds).
///
/// When access is granted, the success message is displayed for this duration
/// before waiting for turnstile rotation. This gives users time to read the
/// message before proceeding.
///
/// # Value: 5 seconds
///
/// # Examples
///
/// ```
/// use turnkey_core::constants::DEFAULT_GRANT_TIMEOUT_SECONDS;
/// use std::time::Duration;
///
/// let display_duration = Duration::from_secs(DEFAULT_GRANT_TIMEOUT_SECONDS as u64);
/// ```
pub const DEFAULT_GRANT_TIMEOUT_SECONDS: u8 = 5;

/// Default display duration for access denial messages (seconds).
///
/// When access is denied, the denial message is displayed for this duration.
/// A value of 0 means the message is shown briefly and the system returns
/// to idle immediately.
///
/// # Value: 0 seconds (instant return to idle)
pub const DEFAULT_DENY_TIMEOUT_SECONDS: u8 = 0;

// ============================================================================
// Default Display Messages (Portuguese)
// ============================================================================

/// Default message for successful access grant.
///
/// Displayed when the server grants access to the user.
/// This message appears in Brazilian Portuguese as per protocol specification.
///
/// # Value: "Acesso liberado" (Access granted)
pub const MSG_ACCESS_GRANTED: &str = "Acesso liberado";

/// Default message for access denial.
///
/// Displayed when the server denies access to the user.
/// This message appears in Brazilian Portuguese as per protocol specification.
///
/// # Value: "Acesso negado" (Access denied)
pub const MSG_ACCESS_DENIED: &str = "Acesso negado";

/// Default message while waiting for validation.
///
/// Displayed while the system is waiting for server response during
/// online validation.
///
/// # Value: "Aguardando..." (Waiting...)
pub const MSG_WAITING: &str = "Aguardando...";

/// Default message for timeout conditions.
///
/// Displayed when validation timeout expires or turnstile rotation timeout occurs.
///
/// # Value: "Tempo esgotado" (Time expired)
pub const MSG_TIMEOUT: &str = "Tempo esgotado";
