use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::TemporalValidity;

/// Card entity representing an RFID/NFC access card
///
/// This model matches the `cartoes.txt` import format from real Henry
/// equipment, ensuring compatibility with data migration from physical devices.
/// Multiple cards can be associated with a single user for redundancy.
///
/// # Fields
///
/// * `id` - Auto-increment primary key (technical key for FK performance)
/// * `numero_cartao` - Unique card number (3-20 chars, decimal or hex format)
/// * `matricula` - User's employee registration number (natural key FK)
/// * `user_id` - User's ID (technical key FK for performance)
/// * `validade_inicio` - Validity period start date (ISO8601 format)
/// * `validade_fim` - Validity period end date (ISO8601 format)
/// * `ativo` - Whether the card is active
/// * `created_at` - Record creation timestamp
/// * `updated_at` - Record last modification timestamp
///
/// # Database Schema
///
/// Maps to the `cards` table with the following constraints:
/// - `numero_cartao` must be unique
/// - Both `matricula` (TEXT) and `user_id` (INTEGER) must reference the same user
/// - Dual-key consistency enforced by database triggers
/// - Card numbers can be 3-20 characters (decimal or hexadecimal)
///
/// ## Dual-Key Strategy
///
/// The card table uses both natural and technical keys:
/// - `matricula`: TEXT foreign key for import file compatibility
/// - `user_id`: INTEGER foreign key for optimal query performance
/// - Database triggers ensure both keys always reference the same user
///
/// # Examples
///
/// ```
/// use turnkey_storage::models::{Card, TemporalValidity};
/// use chrono::Utc;
///
/// let card = Card {
///     id: 1,
///     numero_cartao: "1234567890".to_string(),
///     matricula: "EMP001".to_string(),
///     user_id: 42,
///     validade_inicio: None,
///     validade_fim: None,
///     ativo: true,
///     created_at: Utc::now(),
///     updated_at: Utc::now(),
/// };
///
/// // Check if card is currently valid
/// assert!(card.is_valid());
///
/// // Normalize card number for comparison
/// let normalized = Card::normalize_card_number("  a1b2c3d4  ");
/// assert_eq!(normalized, "A1B2C3D4");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Card {
    /// Auto-increment primary key (technical key for FK performance)
    pub id: i64,

    /// Card number (unique identifier, 3-20 chars)
    ///
    /// Can be decimal (e.g., "1234567890") or hexadecimal (e.g., "A1B2C3D4").
    /// Use `normalize_card_number()` for consistent case-insensitive comparison.
    pub numero_cartao: String,

    /// User's matricula (employee ID) - natural key FK
    ///
    /// This is the primary identifier used in import files and for lookups.
    /// Must reference a valid user in the users table.
    pub matricula: String,

    /// User's ID - technical key FK for performance
    ///
    /// Both `matricula` and `user_id` must reference the same user.
    /// Database triggers enforce this consistency constraint.
    pub user_id: i64,

    /// Start of validity period (ISO8601 format)
    ///
    /// If set, the card cannot be used before this date.
    pub validade_inicio: Option<DateTime<Utc>>,

    /// End of validity period (ISO8601 format)
    ///
    /// If set, the card cannot be used after this date.
    pub validade_fim: Option<DateTime<Utc>>,

    /// Whether the card is active
    ///
    /// Inactive cards are rejected during validation even if within validity period.
    pub ativo: bool,

    /// Record creation timestamp
    pub created_at: DateTime<Utc>,

    /// Record last update timestamp
    pub updated_at: DateTime<Utc>,
}

impl TemporalValidity for Card {
    fn is_active(&self) -> bool {
        self.ativo
    }

    fn validity_start(&self) -> Option<DateTime<Utc>> {
        self.validade_inicio
    }

    fn validity_end(&self) -> Option<DateTime<Utc>> {
        self.validade_fim
    }
}

impl Card {
    /// Normalize card number to uppercase for consistent comparison
    ///
    /// This helps handle case-insensitive card numbers (e.g., hex values).
    pub fn normalize_card_number(numero: &str) -> String {
        numero.trim().to_uppercase()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn create_test_card() -> Card {
        Card {
            id: 1,
            numero_cartao: "1234567890".to_string(),
            matricula: "EMP001".to_string(),
            user_id: 1,
            validade_inicio: Some(Utc::now() - Duration::days(1)),
            validade_fim: Some(Utc::now() + Duration::days(30)),
            ativo: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn test_card_is_valid() {
        let card = create_test_card();
        assert!(card.is_valid());
    }

    #[test]
    fn test_card_inactive() {
        let mut card = create_test_card();
        card.ativo = false;
        assert!(!card.is_valid());
    }

    #[test]
    fn test_card_not_yet_valid() {
        let mut card = create_test_card();
        card.validade_inicio = Some(Utc::now() + Duration::days(1));
        assert!(!card.is_valid());
    }

    #[test]
    fn test_card_expired() {
        let mut card = create_test_card();
        card.validade_fim = Some(Utc::now() - Duration::days(1));
        assert!(!card.is_valid());
    }

    #[test]
    fn test_normalize_card_number() {
        assert_eq!(Card::normalize_card_number("  a1b2c3d4  "), "A1B2C3D4");
        assert_eq!(Card::normalize_card_number("1234567890"), "1234567890");
    }

    #[test]
    fn test_card_with_no_validity_dates() {
        let mut card = create_test_card();
        card.validade_inicio = None;
        card.validade_fim = None;
        assert!(card.is_valid());
    }
}
