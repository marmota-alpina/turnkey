use crate::error::StorageResult;
use crate::messages::DisplayMessages;
use crate::models::{AccessLog, Card, Direction, ReaderType, TemporalValidity};
use crate::repositories::{
    AccessLogRepository, CardRepository, SqliteAccessLogRepository, SqliteCardRepository,
    SqliteUserRepository, UserRepository,
};
use chrono::Utc;
use sqlx::SqlitePool;
use turnkey_protocol::commands::access::{AccessRequest, AccessResponse};

/// Anti-passback window in seconds
///
/// Prevents security violations where users attempt to:
/// - Enter twice without exiting (tailgating)
/// - Exit twice without entering (reverse tailgating)
///
/// # How it works
///
/// When a user requests access, the system checks their last granted access:
/// - If last was Entry and current is Entry → DENY (must exit first)
/// - If last was Exit and current is Exit → DENY (must enter first)
/// - If more than 300 seconds elapsed → ALLOW (window expired)
///
/// # Configuration
///
/// Default: 300 seconds (5 minutes)
/// Future: Should be configurable per-device or per-user
const ANTI_PASSBACK_WINDOW_SECS: i64 = 300;

/// Offline validator for access control requests
///
/// Implements the complete validation flow for offline access control,
/// validating requests against the local SQLite database without
/// requiring network communication.
///
/// # Validation Flow
///
/// The validator executes a strict sequence of checks, failing fast at the first denial:
///
/// 1. **Card Lookup**: Query `cards` table by `numero_cartao`
/// 2. **Card Existence**: Deny if card not found → `CARD_NOT_FOUND`
/// 3. **Card Active**: Deny if `card.ativo = false` → `CARD_INACTIVE`
/// 4. **Card Validity**: Deny if outside validity period → `CARD_EXPIRED`
/// 5. **User Lookup**: Query `users` table by `card.matricula`
/// 6. **User Active**: Deny if `user.ativo = false` → `USER_INACTIVE`
/// 7. **User Validity**: Deny if outside validity period → `USER_EXPIRED`
/// 8. **Access Method**: Deny if user lacks permission → `CARD_ACCESS_DENIED`
/// 9. **Anti-Passback**: Deny if entry-after-entry or exit-after-exit → `ANTI_PASSBACK`
/// 10. **Grant**: All checks passed → `ACCESS_GRANTED`
/// 11. **Logging**: Record attempt (granted or denied) to `access_logs`
///
/// # Security Features
///
/// - **Anti-Passback**: Prevents tailgating and double-entry (5-minute window)
/// - **Temporal Validation**: Cards and users have independent validity periods
/// - **Method Permissions**: Users can restrict access to card/bio/keypad
/// - **Audit Trail**: All access attempts logged with timestamp and reason
///
/// # Examples
///
/// ```no_run
/// use turnkey_storage::validator::OfflineValidator;
/// use turnkey_storage::connection::{Database, DatabaseConfig};
/// use turnkey_protocol::commands::access::AccessRequest;
/// use turnkey_core::{AccessDirection, HenryTimestamp, ReaderType};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let config = DatabaseConfig::new("turnkey.db");
/// let db = Database::new(config).await?;
/// let validator = OfflineValidator::new(db.pool().clone());
///
/// let timestamp = HenryTimestamp::parse("10/05/2025 12:46:06")?;
/// let request = AccessRequest::new(
///     "1234567890".to_string(),
///     timestamp,
///     AccessDirection::Entry,
///     ReaderType::Rfid,
/// )?;
///
/// let response = validator.validate(&request).await?;
/// println!("Access {}", if response.is_grant() { "granted" } else { "denied" });
/// # Ok(())
/// # }
/// ```
pub struct OfflineValidator {
    user_repo: SqliteUserRepository,
    card_repo: SqliteCardRepository,
    log_repo: SqliteAccessLogRepository,
}

impl OfflineValidator {
    /// Create a new offline validator with the given database pool
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            user_repo: SqliteUserRepository::new(pool.clone()),
            card_repo: SqliteCardRepository::new(pool.clone()),
            log_repo: SqliteAccessLogRepository::new(pool),
        }
    }

    /// Validate an access request against the local database
    ///
    /// Executes the complete 9-step offline validation flow and returns
    /// an access response (grant or deny).
    ///
    /// # Arguments
    ///
    /// * `request` - The access request to validate
    ///
    /// # Returns
    ///
    /// Returns `Ok(AccessResponse)` with grant or deny decision.
    ///
    /// # Errors
    ///
    /// Returns error if database operations fail. Note that validation
    /// failures (e.g., card not found) return `Ok(deny_response)`, not errors.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use turnkey_storage::validator::OfflineValidator;
    /// # use turnkey_protocol::commands::access::AccessRequest;
    /// # use turnkey_core::{AccessDirection, HenryTimestamp, ReaderType};
    /// # async fn example(validator: OfflineValidator) -> Result<(), Box<dyn std::error::Error>> {
    /// let timestamp = HenryTimestamp::parse("10/05/2025 12:46:06")?;
    /// let request = AccessRequest::new(
    ///     "1234567890".to_string(),
    ///     timestamp,
    ///     AccessDirection::Entry,
    ///     ReaderType::Rfid,
    /// )?;
    ///
    /// let response = validator.validate(&request).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn validate(&self, request: &AccessRequest) -> StorageResult<AccessResponse> {
        let card_number = Card::normalize_card_number(request.card_number());

        // Step 1: Lookup card by number
        let card = self.card_repo.find_by_number(&card_number).await?;

        // Step 2: Check if card exists
        let card = match card {
            Some(c) => c,
            None => {
                return self
                    .deny_with_log(
                        None,
                        None,
                        &card_number,
                        request,
                        DisplayMessages::CARD_NOT_FOUND,
                    )
                    .await;
            }
        };

        // Step 3: Check if card is active and valid
        if !card.is_valid() {
            let message = if !card.ativo {
                DisplayMessages::CARD_INACTIVE
            } else {
                DisplayMessages::CARD_EXPIRED
            };

            return self
                .deny_with_log(
                    Some(card.user_id),
                    Some(&card.matricula),
                    &card_number,
                    request,
                    message,
                )
                .await;
        }

        // Step 4: Lookup user by matricula
        let user = self.user_repo.find_by_matricula(&card.matricula).await?;

        let user = match user {
            Some(u) => u,
            None => {
                return self
                    .deny_with_log(
                        Some(card.user_id),
                        Some(&card.matricula),
                        &card_number,
                        request,
                        DisplayMessages::USER_NOT_FOUND,
                    )
                    .await;
            }
        };

        // Step 5: Check if user is active and valid
        if !user.is_valid() {
            let message = if !user.ativo {
                DisplayMessages::USER_INACTIVE
            } else {
                DisplayMessages::USER_EXPIRED
            };

            return self
                .deny_with_log(
                    Some(user.id),
                    Some(&user.matricula),
                    &card_number,
                    request,
                    message,
                )
                .await;
        }

        // Step 6: Check access method permission
        // For RFID readers, check allow_card permission
        if request.is_rfid() && !user.allow_card {
            return self
                .deny_with_log(
                    Some(user.id),
                    Some(&user.matricula),
                    &card_number,
                    request,
                    DisplayMessages::CARD_ACCESS_DENIED,
                )
                .await;
        }

        // For biometric readers, check allow_bio permission
        if request.is_biometric() && !user.allow_bio {
            return self
                .deny_with_log(
                    Some(user.id),
                    Some(&user.matricula),
                    &card_number,
                    request,
                    DisplayMessages::BIO_ACCESS_DENIED,
                )
                .await;
        }

        // Step 7: Anti-passback validation
        // Check if user's last access direction conflicts with current request
        let last_access = self
            .log_repo
            .find_by_user_id(user.id, 1)
            .await?
            .first()
            .cloned();

        if let Some(last_log) = last_access {
            // Only check granted accesses for anti-passback
            if last_log.granted {
                let is_entry_after_entry =
                    last_log.direction == Direction::Entry as i32 && request.is_entry();

                let is_exit_after_exit =
                    last_log.direction == Direction::Exit as i32 && request.is_exit();

                // Check if within anti-passback window
                if (is_entry_after_entry || is_exit_after_exit)
                    && !self.is_anti_passback_expired(&last_log)
                {
                    return self
                        .deny_with_log(
                            Some(user.id),
                            Some(&user.matricula),
                            &card_number,
                            request,
                            DisplayMessages::ANTI_PASSBACK,
                        )
                        .await;
                }
            }
        }

        // Step 8: All validations passed - grant access
        // Step 9: Log the successful access
        self.log_access_granted(
            user.id,
            &user.matricula,
            &card_number,
            request,
            DisplayMessages::ACCESS_GRANTED,
        )
        .await?;

        // Step 9: Return grant response based on direction
        let response = if request.is_entry() {
            AccessResponse::grant_entry(DisplayMessages::ACCESS_GRANTED.to_string())
        } else if request.is_exit() {
            AccessResponse::grant_exit(DisplayMessages::ACCESS_GRANTED.to_string())
        } else {
            // Undefined direction - grant both
            AccessResponse::grant_both(DisplayMessages::ACCESS_GRANTED.to_string())
        };

        Ok(response)
    }

    /// Log a granted access attempt
    async fn log_access_granted(
        &self,
        user_id: i64,
        matricula: &str,
        card_number: &str,
        request: &AccessRequest,
        message: &str,
    ) -> StorageResult<()> {
        let direction = self.map_direction(request.direction());
        let reader_type = self.map_reader_type(request.reader_type());

        let log = AccessLog::new(
            Some(user_id),
            Some(matricula.to_string()),
            card_number.to_string(),
            direction,
            reader_type,
            true, // granted
            Some(message.to_string()),
            Utc::now(),
        );

        self.log_repo.create(&log).await?;
        Ok(())
    }

    /// Log a denied access attempt
    async fn log_access_denied(
        &self,
        user_id: Option<i64>,
        matricula: Option<&str>,
        card_number: &str,
        request: &AccessRequest,
        message: &str,
    ) -> StorageResult<()> {
        let direction = self.map_direction(request.direction());
        let reader_type = self.map_reader_type(request.reader_type());

        let log = AccessLog::new(
            user_id,
            matricula.map(|s| s.to_string()),
            card_number.to_string(),
            direction,
            reader_type,
            false, // denied
            Some(message.to_string()),
            Utc::now(),
        );

        self.log_repo.create(&log).await?;
        Ok(())
    }

    /// Helper method to log denied access and return deny response
    ///
    /// This method encapsulates the common pattern of logging a denied access
    /// attempt and returning the corresponding deny response. It ensures
    /// consistent error handling throughout the validation flow.
    ///
    /// # Arguments
    ///
    /// * `user_id` - Optional user ID (None if user not found)
    /// * `matricula` - Optional user matricula (None if user not found)
    /// * `card_number` - Card number that was presented
    /// * `request` - The access request being validated
    /// * `message` - The deny reason message
    ///
    /// # Returns
    ///
    /// Returns `Ok(AccessResponse::deny)` with the provided message.
    ///
    /// # Errors
    ///
    /// Returns error only if database logging fails.
    async fn deny_with_log(
        &self,
        user_id: Option<i64>,
        matricula: Option<&str>,
        card_number: &str,
        request: &AccessRequest,
        message: &str,
    ) -> StorageResult<AccessResponse> {
        self.log_access_denied(user_id, matricula, card_number, request, message)
            .await?;
        Ok(AccessResponse::deny(message.to_string()))
    }

    /// Map turnkey_core::AccessDirection to storage Direction
    fn map_direction(&self, dir: turnkey_core::AccessDirection) -> Direction {
        match dir {
            turnkey_core::AccessDirection::Undefined => Direction::Undefined,
            turnkey_core::AccessDirection::Entry => Direction::Entry,
            turnkey_core::AccessDirection::Exit => Direction::Exit,
        }
    }

    /// Map turnkey_core::ReaderType to storage ReaderType
    fn map_reader_type(&self, reader: turnkey_core::ReaderType) -> ReaderType {
        match reader {
            turnkey_core::ReaderType::Rfid => ReaderType::Rfid,
            turnkey_core::ReaderType::Biometric => ReaderType::Biometric,
        }
    }

    /// Check if anti-passback window has expired
    ///
    /// Returns true if enough time has passed since the last access event,
    /// allowing the user to access again in the same direction.
    ///
    /// # Arguments
    ///
    /// * `last_log` - The most recent access log entry for the user
    ///
    /// # Returns
    ///
    /// Returns true if the anti-passback window has expired (enough time has passed),
    /// false if the user is still within the anti-passback window.
    fn is_anti_passback_expired(&self, last_log: &AccessLog) -> bool {
        let elapsed = Utc::now().signed_duration_since(last_log.timestamp);
        elapsed.num_seconds() > ANTI_PASSBACK_WINDOW_SECS
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::Database;
    use crate::models::User;
    use chrono::Duration;
    use turnkey_core::{AccessDirection, HenryTimestamp};

    async fn setup_test_db() -> Database {
        Database::in_memory().await.unwrap()
    }

    async fn create_test_user(db: &Database, matricula: &str) -> i64 {
        let user = User {
            id: 0,
            pis: None,
            nome: "Test User".to_string(),
            matricula: matricula.to_string(),
            cpf: None,
            validade_inicio: Some(Utc::now() - Duration::days(1)),
            validade_fim: Some(Utc::now() + Duration::days(30)),
            ativo: true,
            allow_card: true,
            allow_bio: false,
            allow_keypad: false,
            codigo: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let repo = SqliteUserRepository::new(db.pool().clone());
        repo.create(&user).await.unwrap()
    }

    async fn create_test_card(db: &Database, numero: &str, matricula: &str, user_id: i64) {
        let card = Card {
            id: 0,
            numero_cartao: numero.to_string(),
            matricula: matricula.to_string(),
            user_id,
            validade_inicio: Some(Utc::now() - Duration::days(1)),
            validade_fim: Some(Utc::now() + Duration::days(30)),
            ativo: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let repo = SqliteCardRepository::new(db.pool().clone());
        repo.create(&card).await.unwrap();
    }

    fn create_access_request(card_number: &str, direction: AccessDirection) -> AccessRequest {
        let timestamp = HenryTimestamp::parse("10/05/2025 12:46:06").unwrap();
        AccessRequest::new(
            card_number.to_string(),
            timestamp,
            direction,
            turnkey_core::ReaderType::Rfid,
        )
        .unwrap()
    }

    #[tokio::test]
    async fn test_validate_grant_entry() {
        let db = setup_test_db().await;
        let user_id = create_test_user(&db, "EMP001").await;
        create_test_card(&db, "1234567890", "EMP001", user_id).await;

        let validator = OfflineValidator::new(db.pool().clone());
        let request = create_access_request("1234567890", AccessDirection::Entry);

        let response = validator.validate(&request).await.unwrap();
        assert!(response.is_grant());
        assert_eq!(response.display_message(), "Acesso liberado");
    }

    #[tokio::test]
    async fn test_validate_grant_exit() {
        let db = setup_test_db().await;
        let user_id = create_test_user(&db, "EMP002").await;
        create_test_card(&db, "2222222222", "EMP002", user_id).await;

        let validator = OfflineValidator::new(db.pool().clone());
        let request = create_access_request("2222222222", AccessDirection::Exit);

        let response = validator.validate(&request).await.unwrap();
        assert!(response.is_grant());
    }

    #[tokio::test]
    async fn test_validate_card_not_found() {
        let db = setup_test_db().await;
        let validator = OfflineValidator::new(db.pool().clone());
        let request = create_access_request("9999999999", AccessDirection::Entry);

        let response = validator.validate(&request).await.unwrap();
        assert!(response.is_deny());
        assert_eq!(response.display_message(), "Cartao nao cadastrado");
    }

    #[tokio::test]
    async fn test_validate_card_inactive() {
        let db = setup_test_db().await;
        let user_id = create_test_user(&db, "EMP003").await;

        // Create inactive card
        let card = Card {
            id: 0,
            numero_cartao: "3333333333".to_string(),
            matricula: "EMP003".to_string(),
            user_id,
            validade_inicio: None,
            validade_fim: None,
            ativo: false, // Inactive
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let repo = SqliteCardRepository::new(db.pool().clone());
        repo.create(&card).await.unwrap();

        let validator = OfflineValidator::new(db.pool().clone());
        let request = create_access_request("3333333333", AccessDirection::Entry);

        let response = validator.validate(&request).await.unwrap();
        assert!(response.is_deny());
        assert_eq!(response.display_message(), "Cartao inativo");
    }

    #[tokio::test]
    async fn test_validate_card_expired() {
        let db = setup_test_db().await;
        let user_id = create_test_user(&db, "EMP004").await;

        // Create expired card
        let card = Card {
            id: 0,
            numero_cartao: "4444444444".to_string(),
            matricula: "EMP004".to_string(),
            user_id,
            validade_inicio: Some(Utc::now() - Duration::days(60)),
            validade_fim: Some(Utc::now() - Duration::days(1)), // Expired
            ativo: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let repo = SqliteCardRepository::new(db.pool().clone());
        repo.create(&card).await.unwrap();

        let validator = OfflineValidator::new(db.pool().clone());
        let request = create_access_request("4444444444", AccessDirection::Entry);

        let response = validator.validate(&request).await.unwrap();
        assert!(response.is_deny());
        assert_eq!(
            response.display_message(),
            "Cartao fora do periodo de validade"
        );
    }

    #[tokio::test]
    async fn test_validate_user_inactive() {
        let db = setup_test_db().await;

        // Create inactive user
        let user = User {
            id: 0,
            pis: None,
            nome: "Inactive User".to_string(),
            matricula: "EMP005".to_string(),
            cpf: None,
            validade_inicio: None,
            validade_fim: None,
            ativo: false, // Inactive
            allow_card: true,
            allow_bio: false,
            allow_keypad: false,
            codigo: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let user_repo = SqliteUserRepository::new(db.pool().clone());
        let user_id = user_repo.create(&user).await.unwrap();

        create_test_card(&db, "5555555555", "EMP005", user_id).await;

        let validator = OfflineValidator::new(db.pool().clone());
        let request = create_access_request("5555555555", AccessDirection::Entry);

        let response = validator.validate(&request).await.unwrap();
        assert!(response.is_deny());
        assert_eq!(response.display_message(), "Usuario inativo");
    }

    #[tokio::test]
    async fn test_validate_user_card_not_allowed() {
        let db = setup_test_db().await;

        // Create user with card access disabled
        let user = User {
            id: 0,
            pis: None,
            nome: "No Card Access".to_string(),
            matricula: "EMP006".to_string(),
            cpf: None,
            validade_inicio: None,
            validade_fim: None,
            ativo: true,
            allow_card: false, // Card access not allowed
            allow_bio: true,
            allow_keypad: false,
            codigo: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let user_repo = SqliteUserRepository::new(db.pool().clone());
        let user_id = user_repo.create(&user).await.unwrap();

        create_test_card(&db, "6666666666", "EMP006", user_id).await;

        let validator = OfflineValidator::new(db.pool().clone());
        let request = create_access_request("6666666666", AccessDirection::Entry);

        let response = validator.validate(&request).await.unwrap();
        assert!(response.is_deny());
        assert_eq!(
            response.display_message(),
            "Acesso por cartao nao permitido"
        );
    }

    #[tokio::test]
    async fn test_validate_logs_granted_access() {
        let db = setup_test_db().await;
        let user_id = create_test_user(&db, "EMP007").await;
        create_test_card(&db, "7777777777", "EMP007", user_id).await;

        let validator = OfflineValidator::new(db.pool().clone());
        let request = create_access_request("7777777777", AccessDirection::Entry);

        validator.validate(&request).await.unwrap();

        // Verify log was created
        let log_repo = SqliteAccessLogRepository::new(db.pool().clone());
        let logs = log_repo
            .find_by_card_number("7777777777", 10)
            .await
            .unwrap();

        assert_eq!(logs.len(), 1);
        assert!(logs[0].granted);
        assert_eq!(logs[0].display_message.as_deref(), Some("Acesso liberado"));
    }

    #[tokio::test]
    async fn test_validate_logs_denied_access() {
        let db = setup_test_db().await;
        let validator = OfflineValidator::new(db.pool().clone());
        let request = create_access_request("8888888888", AccessDirection::Entry);

        validator.validate(&request).await.unwrap();

        // Verify log was created
        let log_repo = SqliteAccessLogRepository::new(db.pool().clone());
        let logs = log_repo
            .find_by_card_number("8888888888", 10)
            .await
            .unwrap();

        assert_eq!(logs.len(), 1);
        assert!(!logs[0].granted);
        assert_eq!(
            logs[0].display_message.as_deref(),
            Some("Cartao nao cadastrado")
        );
    }

    #[tokio::test]
    async fn test_validate_normalizes_card_number() {
        let db = setup_test_db().await;
        let user_id = create_test_user(&db, "EMP009").await;
        create_test_card(&db, "ABCDEF1234", "EMP009", user_id).await;

        let validator = OfflineValidator::new(db.pool().clone());
        // Request with lowercase card number
        let request = create_access_request("abcdef1234", AccessDirection::Entry);

        let response = validator.validate(&request).await.unwrap();
        assert!(response.is_grant());
    }
}
