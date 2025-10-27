//! Display messages for access control validation results
//!
//! This module provides constants for all display messages shown to users
//! during access control operations. All messages are currently in Portuguese
//! (Brazilian) as required by the Henry protocol equipment market.
//!
//! # Internationalization
//!
//! Future versions may support multiple languages through a message catalog
//! or i18n framework. For now, all messages are compile-time constants.
//!
//! # Usage
//!
//! ```
//! use turnkey_storage::messages::DisplayMessages;
//!
//! let message = DisplayMessages::ACCESS_GRANTED;
//! println!("{}", message); // "Acesso liberado"
//! ```

/// Display messages for access control validation (Portuguese/Brazilian)
///
/// This struct provides constants for all user-facing messages in the system.
/// Messages are displayed on turnstile LCD screens (typically 2 lines × 40 columns).
///
/// # Message Format
///
/// - Maximum 40 characters per line (hardware limitation)
/// - ASCII characters only (no UTF-8 accents to ensure hardware compatibility)
/// - Portuguese language (Brazilian market requirement)
///
/// # Extending Messages
///
/// To add new messages:
/// 1. Add a new constant following the naming pattern
/// 2. Keep messages under 40 characters
/// 3. Use ASCII-only characters (no accents)
/// 4. Add corresponding test in `test_messages_are_non_empty()`
pub struct DisplayMessages;

impl DisplayMessages {
    /// Card not found in database
    ///
    /// Returned when `numero_cartao` does not exist in cards table.
    pub const CARD_NOT_FOUND: &'static str = "Cartao nao cadastrado";

    /// Card is inactive (ativo = false)
    ///
    /// Returned when card exists but `ativo` field is false.
    pub const CARD_INACTIVE: &'static str = "Cartao inativo";

    /// Card validity period expired
    ///
    /// Returned when current time is outside card's validity window
    /// (`validade_inicio` to `validade_fim`).
    pub const CARD_EXPIRED: &'static str = "Cartao fora do periodo de validade";

    /// User not found in database
    ///
    /// Returned when card's `matricula` does not exist in users table.
    pub const USER_NOT_FOUND: &'static str = "Usuario nao encontrado";

    /// User is inactive (ativo = false)
    ///
    /// Returned when user exists but `ativo` field is false.
    pub const USER_INACTIVE: &'static str = "Usuario inativo";

    /// User validity period expired
    ///
    /// Returned when current time is outside user's validity window
    /// (`validade_inicio` to `validade_fim`).
    pub const USER_EXPIRED: &'static str = "Usuario fora do periodo de validade";

    /// User does not have permission to use RFID card access
    ///
    /// Returned when user's `allow_card` field is false.
    pub const CARD_ACCESS_DENIED: &'static str = "Acesso por cartao nao permitido";

    /// User does not have permission to use biometric access
    ///
    /// Returned when user's `allow_bio` field is false.
    pub const BIO_ACCESS_DENIED: &'static str = "Acesso biometrico nao permitido";

    /// Access granted successfully
    ///
    /// Returned when all validation checks pass.
    pub const ACCESS_GRANTED: &'static str = "Acesso liberado";

    /// Anti-passback violation detected
    ///
    /// Returned when user attempts entry-after-entry or exit-after-exit
    /// within the anti-passback time window (default: 5 minutes).
    pub const ANTI_PASSBACK: &'static str = "Bloqueio por anti-dupla";
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Ensures all message constants have content (no empty strings)
    ///
    /// This prevents accidentally shipping empty messages to production
    /// which would result in blank LCD displays on turnstiles.
    #[test]
    #[allow(clippy::const_is_empty)]
    fn test_messages_are_non_empty() {
        assert!(!DisplayMessages::CARD_NOT_FOUND.is_empty());
        assert!(!DisplayMessages::CARD_INACTIVE.is_empty());
        assert!(!DisplayMessages::CARD_EXPIRED.is_empty());
        assert!(!DisplayMessages::USER_NOT_FOUND.is_empty());
        assert!(!DisplayMessages::USER_INACTIVE.is_empty());
        assert!(!DisplayMessages::USER_EXPIRED.is_empty());
        assert!(!DisplayMessages::CARD_ACCESS_DENIED.is_empty());
        assert!(!DisplayMessages::BIO_ACCESS_DENIED.is_empty());
        assert!(!DisplayMessages::ACCESS_GRANTED.is_empty());
        assert!(!DisplayMessages::ANTI_PASSBACK.is_empty());
    }

    /// Verifies messages are in Portuguese (Brazilian market requirement)
    ///
    /// Checks for key Portuguese words to ensure messages weren't
    /// accidentally changed to English or other languages.
    #[test]
    fn test_messages_in_portuguese() {
        assert!(DisplayMessages::CARD_NOT_FOUND.contains("Cartao"));
        assert!(DisplayMessages::ACCESS_GRANTED.contains("Acesso"));
        assert!(DisplayMessages::USER_INACTIVE.contains("Usuario"));
    }
}
