use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::TemporalValidity;

/// User entity representing a person with access credentials
///
/// This model matches the `colaborador.txt` import format from real Henry
/// equipment, ensuring compatibility with data migration from physical devices.
///
/// # Fields
///
/// * `id` - Auto-increment primary key (technical key for FK performance)
/// * `pis` - PIS (Programa de Integração Social), 11 digits, optional Brazilian social program ID
/// * `nome` - Full name, maximum 100 characters, required
/// * `matricula` - Unique employee registration number (natural key, 3-20 chars)
/// * `cpf` - CPF (Cadastro de Pessoas Físicas), 11 digits, optional Brazilian tax ID
/// * `validade_inicio` - Validity period start date (ISO8601 format)
/// * `validade_fim` - Validity period end date (ISO8601 format)
/// * `ativo` - Whether the user account is active
/// * `allow_card` - Whether RFID/NFC card access is permitted
/// * `allow_bio` - Whether biometric (fingerprint) access is permitted
/// * `allow_keypad` - Whether keypad (PIN code) access is permitted
/// * `codigo` - Numeric access code (required if allow_keypad is true)
/// * `created_at` - Record creation timestamp
/// * `updated_at` - Record last modification timestamp
///
/// # Database Schema
///
/// Maps to the `users` table with the following constraints:
/// - `matricula` must be unique
/// - At least one access method must be enabled (allow_card OR allow_bio OR allow_keypad)
/// - If `allow_keypad` is true, `codigo` must not be null
/// - PIS and CPF must be exactly 11 digits if provided
///
/// # Examples
///
/// ```
/// use turnkey_storage::models::User;
/// use chrono::Utc;
///
/// let user = User {
///     id: 1,
///     pis: Some("12345678901".to_string()),
///     nome: "John Doe".to_string(),
///     matricula: "EMP001".to_string(),
///     cpf: Some("12345678901".to_string()),
///     validade_inicio: None,
///     validade_fim: None,
///     ativo: true,
///     allow_card: true,
///     allow_bio: false,
///     allow_keypad: true,
///     codigo: Some("1234".to_string()),
///     created_at: Utc::now(),
///     updated_at: Utc::now(),
/// };
///
/// // Check if user can access with card
/// assert!(user.can_use_card());
///
/// // Verify PIN code
/// assert!(user.verify_code("1234"));
/// assert!(!user.verify_code("0000"));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    /// Auto-increment primary key (technical key for FK performance)
    pub id: i64,

    /// PIS (Programa de Integração Social) - 11 digits, optional
    pub pis: Option<String>,

    /// Full name (max 100 characters)
    pub nome: String,

    /// Unique employee/user registration number (natural key, 3-20 chars)
    ///
    /// This is the primary identifier used in import files and for lookups.
    pub matricula: String,

    /// CPF (Cadastro de Pessoas Físicas) - 11 digits, optional
    pub cpf: Option<String>,

    /// Start of validity period (ISO8601 format)
    pub validade_inicio: Option<DateTime<Utc>>,

    /// End of validity period (ISO8601 format)
    pub validade_fim: Option<DateTime<Utc>>,

    /// Whether the user is active (can access)
    pub ativo: bool,

    /// Whether RFID/NFC card access is allowed
    pub allow_card: bool,

    /// Whether biometric (fingerprint) access is allowed
    pub allow_bio: bool,

    /// Whether keypad (PIN code) access is allowed
    pub allow_keypad: bool,

    /// Numeric access code (required if allow_keypad is true)
    pub codigo: Option<String>,

    /// Record creation timestamp
    pub created_at: DateTime<Utc>,

    /// Record last update timestamp
    pub updated_at: DateTime<Utc>,
}

impl TemporalValidity for User {
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

impl User {
    /// Check if the user has card access enabled and is valid
    pub fn can_use_card(&self) -> bool {
        self.is_valid() && self.allow_card
    }

    /// Check if the user has biometric access enabled and is valid
    pub fn can_use_biometric(&self) -> bool {
        self.is_valid() && self.allow_bio
    }

    /// Check if the user has keypad access enabled and is valid
    pub fn can_use_keypad(&self) -> bool {
        self.is_valid() && self.allow_keypad && self.codigo.is_some()
    }

    /// Check if the provided PIN code matches the user's code
    ///
    /// This method uses constant-time comparison to prevent timing attacks
    /// that could be used to determine valid PIN codes through response time analysis.
    ///
    /// # Security
    ///
    /// CRITICAL: This implementation uses constant-time comparison via the `subtle` crate
    /// to prevent timing side-channel attacks. Do not replace with standard string comparison.
    ///
    /// # Examples
    ///
    /// ```
    /// # use turnkey_storage::models::User;
    /// # use chrono::Utc;
    /// # let user = User {
    /// #     id: 1, pis: None, nome: "Test".to_string(), matricula: "001".to_string(),
    /// #     cpf: None, validade_inicio: None, validade_fim: None, ativo: true,
    /// #     allow_card: false, allow_bio: false, allow_keypad: true,
    /// #     codigo: Some("1234".to_string()), created_at: Utc::now(), updated_at: Utc::now(),
    /// # };
    /// assert!(user.verify_code("1234"));
    /// assert!(!user.verify_code("9999"));
    /// ```
    pub fn verify_code(&self, code: &str) -> bool {
        use subtle::ConstantTimeEq;

        if !self.can_use_keypad() {
            return false;
        }

        match &self.codigo {
            Some(stored_code) => stored_code.as_bytes().ct_eq(code.as_bytes()).into(),
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn create_test_user() -> User {
        User {
            id: 1,
            pis: Some("12345678901".to_string()),
            nome: "João Silva".to_string(),
            matricula: "EMP001".to_string(),
            cpf: Some("12345678901".to_string()),
            validade_inicio: Some(Utc::now() - Duration::days(1)),
            validade_fim: Some(Utc::now() + Duration::days(30)),
            ativo: true,
            allow_card: true,
            allow_bio: false,
            allow_keypad: true,
            codigo: Some("1234".to_string()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn test_user_is_valid() {
        let user = create_test_user();
        assert!(user.is_valid());
    }

    #[test]
    fn test_user_inactive() {
        let mut user = create_test_user();
        user.ativo = false;
        assert!(!user.is_valid());
    }

    #[test]
    fn test_user_not_yet_valid() {
        let mut user = create_test_user();
        user.validade_inicio = Some(Utc::now() + Duration::days(1));
        assert!(!user.is_valid());
    }

    #[test]
    fn test_user_expired() {
        let mut user = create_test_user();
        user.validade_fim = Some(Utc::now() - Duration::days(1));
        assert!(!user.is_valid());
    }

    #[test]
    fn test_can_use_card() {
        let user = create_test_user();
        assert!(user.can_use_card());
    }

    #[test]
    fn test_cannot_use_biometric() {
        let user = create_test_user();
        assert!(!user.can_use_biometric());
    }

    #[test]
    fn test_can_use_keypad() {
        let user = create_test_user();
        assert!(user.can_use_keypad());
    }

    #[test]
    fn test_verify_code_success() {
        let user = create_test_user();
        assert!(user.verify_code("1234"));
    }

    #[test]
    fn test_verify_code_failure() {
        let user = create_test_user();
        assert!(!user.verify_code("9999"));
    }

    #[test]
    fn test_verify_code_no_code() {
        let mut user = create_test_user();
        user.codigo = None;
        assert!(!user.verify_code("1234"));
    }
}
