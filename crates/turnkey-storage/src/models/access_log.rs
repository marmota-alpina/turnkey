use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Access log entry representing an access attempt (granted or denied)
///
/// This table provides complete audit trail for all access events,
/// supporting compliance, security monitoring, and forensic analysis requirements.
/// Logs are created for both successful and denied access attempts.
///
/// # Fields
///
/// * `id` - Auto-increment primary key
/// * `user_id` - User ID (NULL if card not registered or biometric not identified)
/// * `matricula` - User's employee registration number (NULL if not identified)
/// * `card_number` - Card number or credential identifier that was presented
/// * `direction` - Direction of access (0=undefined, 1=entry, 2=exit)
/// * `reader_type` - Type of reader used (1=RFID, 5=biometric)
/// * `granted` - Whether access was granted (true) or denied (false)
/// * `display_message` - Message shown to user (e.g., "Acesso liberado", "Acesso negado")
/// * `timestamp` - When the access attempt occurred (from device/request)
/// * `created_at` - When the log was written to database
///
/// # Database Schema
///
/// Maps to the `access_logs` table with the following characteristics:
/// - Optional user references: `user_id` and `matricula` can be NULL for unregistered cards
/// - Required card number: Always present, even for denied or unregistered attempts
/// - Indexed columns: `user_id`, `card_number`, `timestamp`, `granted` for query performance
/// - Dual timestamp strategy: `timestamp` (event time) vs `created_at` (log time)
///
/// # Security and Compliance
///
/// Access logs are critical for:
/// - **Audit trails**: Complete history of all access attempts
/// - **Security monitoring**: Detecting unauthorized access patterns
/// - **Compliance**: Meeting regulatory requirements (SOX, GDPR, etc.)
/// - **Forensics**: Investigating security incidents
///
/// Logs should be write-only in production and never deleted, only archived.
///
/// # Examples
///
/// ```
/// use turnkey_storage::models::{AccessLog, Direction, ReaderType};
/// use chrono::Utc;
///
/// // Successful card access
/// let granted_log = AccessLog::new(
///     Some(42),                          // user_id
///     Some("EMP001".to_string()),        // matricula
///     "1234567890".to_string(),          // card_number
///     Direction::Entry,                  // direction
///     ReaderType::Rfid,                  // reader_type
///     true,                              // granted
///     Some("Acesso liberado".to_string()), // display_message
///     Utc::now(),                        // timestamp
/// );
///
/// assert!(granted_log.was_granted());
/// assert_eq!(granted_log.get_direction(), Some(Direction::Entry));
///
/// // Denied access for unregistered card
/// let denied_log = AccessLog::new(
///     None,                              // user_id (not found)
///     None,                              // matricula (not found)
///     "9999999999".to_string(),          // card_number
///     Direction::Entry,                  // direction
///     ReaderType::Rfid,                  // reader_type
///     false,                             // granted
///     Some("Cartão não cadastrado".to_string()), // display_message
///     Utc::now(),                        // timestamp
/// );
///
/// assert!(denied_log.was_denied());
/// assert_eq!(denied_log.user_id, None);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AccessLog {
    /// Auto-increment primary key
    pub id: i64,

    /// User ID (NULL if card not registered or biometric not identified)
    ///
    /// This field is optional because access attempts with unregistered
    /// credentials are still logged for security monitoring.
    pub user_id: Option<i64>,

    /// User's matricula (NULL if card not registered or biometric not identified)
    ///
    /// Natural key reference to the user, NULL for unregistered access attempts.
    pub matricula: Option<String>,

    /// Card number or credential identifier that was presented
    ///
    /// Always present, even for denied or unregistered attempts.
    /// This is the raw credential value from the reader device.
    pub card_number: String,

    /// Direction of access attempt
    ///
    /// - 0: Undefined (unknown or not applicable)
    /// - 1: Entry (entrada)
    /// - 2: Exit (saída)
    ///
    /// Use `get_direction()` to convert to the `Direction` enum.
    pub direction: i32,

    /// Type of reader used
    ///
    /// - 1: RFID/NFC card reader
    /// - 5: Biometric reader (fingerprint)
    ///
    /// Use `get_reader_type()` to convert to the `ReaderType` enum.
    pub reader_type: i32,

    /// Whether access was granted (true) or denied (false)
    ///
    /// False indicates a security event that may require investigation.
    pub granted: bool,

    /// Message displayed to user (e.g., "Acesso liberado", "Acesso negado")
    ///
    /// Optional but recommended for audit trail clarity.
    pub display_message: Option<String>,

    /// Timestamp when the access attempt occurred (from device/request)
    ///
    /// This is the event time from the access control device, not the database write time.
    pub timestamp: DateTime<Utc>,

    /// Record creation timestamp (when logged to database)
    ///
    /// This may differ from `timestamp` due to network delays or offline queueing.
    pub created_at: DateTime<Utc>,
}

/// Direction of access (entry or exit)
///
/// Represents the direction of movement through an access control point.
/// This enum maps directly to the Henry protocol direction codes.
///
/// # Protocol Mapping
///
/// - `0` - Undefined: Unknown or not applicable (e.g., configuration commands)
/// - `1` - Entry: Movement into the controlled area (entrada)
/// - `2` - Exit: Movement out of the controlled area (saída)
///
/// # Examples
///
/// ```
/// use turnkey_storage::models::Direction;
///
/// let dir = Direction::Entry;
/// assert_eq!(dir.display_name(), "Entrada");
/// assert_eq!(i32::from(dir), 1);
///
/// let parsed = Direction::from_i32(1);
/// assert_eq!(parsed, Some(Direction::Entry));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i32)]
pub enum Direction {
    /// Unknown or not applicable direction
    Undefined = 0,
    /// Entry into the controlled area (entrada)
    Entry = 1,
    /// Exit from the controlled area (saída)
    Exit = 2,
}

impl Direction {
    /// Convert integer to Direction enum
    ///
    /// # Arguments
    ///
    /// * `value` - Integer direction code from Henry protocol
    ///
    /// # Returns
    ///
    /// Returns `Some(Direction)` if the value is valid (0, 1, or 2), `None` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use turnkey_storage::models::Direction;
    ///
    /// assert_eq!(Direction::from_i32(1), Some(Direction::Entry));
    /// assert_eq!(Direction::from_i32(2), Some(Direction::Exit));
    /// assert_eq!(Direction::from_i32(99), None);
    /// ```
    pub fn from_i32(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::Undefined),
            1 => Some(Self::Entry),
            2 => Some(Self::Exit),
            _ => None,
        }
    }

    /// Get display name for direction in Portuguese
    ///
    /// Returns a human-readable string suitable for displaying to users.
    ///
    /// # Examples
    ///
    /// ```
    /// use turnkey_storage::models::Direction;
    ///
    /// assert_eq!(Direction::Entry.display_name(), "Entrada");
    /// assert_eq!(Direction::Exit.display_name(), "Saída");
    /// ```
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Undefined => "Indefinido",
            Self::Entry => "Entrada",
            Self::Exit => "Saída",
        }
    }
}

impl From<Direction> for i32 {
    fn from(dir: Direction) -> i32 {
        dir as i32
    }
}

/// Type of reader device used for access control
///
/// Represents the hardware device type that captured the access credential.
/// This enum maps directly to the Henry protocol reader type codes.
///
/// # Protocol Mapping
///
/// - `1` - RFID: RFID/NFC card reader (proximity cards, Mifare, etc.)
/// - `5` - Biometric: Fingerprint biometric reader
///
/// # Note
///
/// The protocol reserves additional codes for other reader types (keypad=0, barcode=2, etc.),
/// but this implementation currently supports only RFID and biometric readers.
///
/// # Examples
///
/// ```
/// use turnkey_storage::models::ReaderType;
///
/// let reader = ReaderType::Rfid;
/// assert_eq!(reader.display_name(), "RFID");
/// assert_eq!(i32::from(reader), 1);
///
/// let parsed = ReaderType::from_i32(5);
/// assert_eq!(parsed, Some(ReaderType::Biometric));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i32)]
pub enum ReaderType {
    /// RFID/NFC card reader (proximity cards, Mifare, etc.)
    Rfid = 1,
    /// Fingerprint biometric reader
    Biometric = 5,
}

impl ReaderType {
    /// Convert integer to ReaderType enum
    ///
    /// # Arguments
    ///
    /// * `value` - Integer reader type code from Henry protocol
    ///
    /// # Returns
    ///
    /// Returns `Some(ReaderType)` if the value is valid (1 or 5), `None` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use turnkey_storage::models::ReaderType;
    ///
    /// assert_eq!(ReaderType::from_i32(1), Some(ReaderType::Rfid));
    /// assert_eq!(ReaderType::from_i32(5), Some(ReaderType::Biometric));
    /// assert_eq!(ReaderType::from_i32(2), None); // Barcode not supported
    /// ```
    pub fn from_i32(value: i32) -> Option<Self> {
        match value {
            1 => Some(Self::Rfid),
            5 => Some(Self::Biometric),
            _ => None,
        }
    }

    /// Get display name for reader type in Portuguese
    ///
    /// Returns a human-readable string suitable for displaying to users.
    ///
    /// # Examples
    ///
    /// ```
    /// use turnkey_storage::models::ReaderType;
    ///
    /// assert_eq!(ReaderType::Rfid.display_name(), "RFID");
    /// assert_eq!(ReaderType::Biometric.display_name(), "Biométrico");
    /// ```
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Rfid => "RFID",
            Self::Biometric => "Biométrico",
        }
    }
}

impl From<ReaderType> for i32 {
    fn from(reader: ReaderType) -> i32 {
        reader as i32
    }
}

impl AccessLog {
    /// Create a new access log entry
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        user_id: Option<i64>,
        matricula: Option<String>,
        card_number: String,
        direction: Direction,
        reader_type: ReaderType,
        granted: bool,
        display_message: Option<String>,
        timestamp: DateTime<Utc>,
    ) -> Self {
        Self {
            id: 0, // Will be set by database
            user_id,
            matricula,
            card_number,
            direction: direction.into(),
            reader_type: reader_type.into(),
            granted,
            display_message,
            timestamp,
            created_at: Utc::now(),
        }
    }

    /// Get the direction as an enum
    pub fn get_direction(&self) -> Option<Direction> {
        Direction::from_i32(self.direction)
    }

    /// Get the reader type as an enum
    pub fn get_reader_type(&self) -> Option<ReaderType> {
        ReaderType::from_i32(self.reader_type)
    }

    /// Check if this was a successful access (granted)
    pub fn was_granted(&self) -> bool {
        self.granted
    }

    /// Check if this was a denied access
    pub fn was_denied(&self) -> bool {
        !self.granted
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_direction_from_i32() {
        assert_eq!(Direction::from_i32(0), Some(Direction::Undefined));
        assert_eq!(Direction::from_i32(1), Some(Direction::Entry));
        assert_eq!(Direction::from_i32(2), Some(Direction::Exit));
        assert_eq!(Direction::from_i32(3), None);
    }

    #[test]
    fn test_direction_display_name() {
        assert_eq!(Direction::Undefined.display_name(), "Indefinido");
        assert_eq!(Direction::Entry.display_name(), "Entrada");
        assert_eq!(Direction::Exit.display_name(), "Saída");
    }

    #[test]
    fn test_reader_type_from_i32() {
        assert_eq!(ReaderType::from_i32(1), Some(ReaderType::Rfid));
        assert_eq!(ReaderType::from_i32(5), Some(ReaderType::Biometric));
        assert_eq!(ReaderType::from_i32(2), None);
    }

    #[test]
    fn test_reader_type_display_name() {
        assert_eq!(ReaderType::Rfid.display_name(), "RFID");
        assert_eq!(ReaderType::Biometric.display_name(), "Biométrico");
    }

    #[test]
    fn test_access_log_new() {
        let log = AccessLog::new(
            Some(1),
            Some("EMP001".to_string()),
            "1234567890".to_string(),
            Direction::Entry,
            ReaderType::Rfid,
            true,
            Some("Acesso liberado".to_string()),
            Utc::now(),
        );

        assert_eq!(log.user_id, Some(1));
        assert_eq!(log.matricula.as_deref(), Some("EMP001"));
        assert_eq!(log.card_number, "1234567890");
        assert_eq!(log.direction, 1);
        assert_eq!(log.reader_type, 1);
        assert!(log.granted);
        assert_eq!(log.display_message.as_deref(), Some("Acesso liberado"));
    }

    #[test]
    fn test_access_log_getters() {
        let log = AccessLog::new(
            None,
            None,
            "9999999999".to_string(),
            Direction::Exit,
            ReaderType::Biometric,
            false,
            Some("Acesso negado".to_string()),
            Utc::now(),
        );

        assert_eq!(log.get_direction(), Some(Direction::Exit));
        assert_eq!(log.get_reader_type(), Some(ReaderType::Biometric));
        assert!(log.was_denied());
        assert!(!log.was_granted());
    }
}
