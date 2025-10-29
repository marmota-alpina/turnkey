use crate::error::{StorageError, StorageResult};
use crate::messages::DisplayMessages;
use crate::models::{AccessLog, Card, Direction, ReaderType, TemporalValidity};
use crate::repositories::{
    AccessLogRepository, CardRepository, SqliteAccessLogRepository, SqliteCardRepository,
    SqliteUserRepository, UserRepository,
};
use chrono::Utc;
use sqlx::SqlitePool;
use std::time::Duration;
use turnkey_core::DeviceId;
use turnkey_network::TcpClient;
use turnkey_protocol::commands::access::{AccessRequest, AccessResponse};
use turnkey_protocol::{CommandCode, FieldData, Message, MessageBuilder};

/// Trait for access validation implementations
///
/// Enables compile-time polymorphic validation through generic code.
/// For runtime selection between validators, use the [`Validator`] enum.
///
/// # Rust 2024 Edition
///
/// This trait uses native async fn in traits (RPITIT) enabled by
/// Rust 1.90 and Edition 2024, eliminating the need for macros.
///
/// # Note on Object Safety
///
/// This trait is not object-safe due to the async fn. For runtime
/// polymorphism, use the [`Validator`] enum which provides zero-cost
/// abstraction over concrete validator types.
///
/// # Examples
///
/// ```no_run
/// use turnkey_storage::AccessValidator;
/// use turnkey_protocol::commands::access::AccessRequest;
///
/// // Generic function works with any validator
/// async fn process_request<V: AccessValidator>(
///     validator: &mut V,
///     request: &AccessRequest
/// ) -> Result<(), Box<dyn std::error::Error>> {
///     let response = validator.validate(request).await?;
///     println!("Access {}", if response.is_grant() { "granted" } else { "denied" });
///     Ok(())
/// }
/// ```
#[allow(async_fn_in_trait)]
pub trait AccessValidator {
    /// Validate an access request
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
    /// Returns error if validation cannot be completed (database failure,
    /// network error, etc.). Validation failures (e.g., card not found)
    /// return `Ok(AccessResponse::deny)`, not errors.
    async fn validate(&mut self, request: &AccessRequest) -> StorageResult<AccessResponse>;
}

/// Runtime-selectable validator
///
/// Provides zero-cost abstraction for choosing between online and offline
/// validation at runtime. This enum is the recommended way to achieve
/// runtime polymorphism in the turnstile emulator.
///
/// # Performance
///
/// This enum uses static dispatch internally, so there is no vtable
/// overhead compared to trait objects. The compiler can often optimize
/// the match away entirely.
///
/// # Examples
///
/// ```no_run
/// use turnkey_storage::{Validator, OnlineValidator, OfflineValidator};
/// use turnkey_storage::{OnlineValidatorConfig, AccessValidator};
/// use turnkey_network::{TcpClient, TcpClientConfig};
/// use turnkey_core::DeviceId;
/// use std::time::Duration;
///
/// # async fn example(
/// #     pool: sqlx::SqlitePool,
/// #     online_mode: bool
/// # ) -> Result<(), Box<dyn std::error::Error>> {
/// // Runtime selection based on configuration
/// let mut validator = if online_mode {
///     let client_config = TcpClientConfig {
///         server_addr: "192.168.0.100:3000".parse()?,
///         timeout: Duration::from_millis(3000),
///     };
///     let client = TcpClient::new(client_config);
///     let device_id = DeviceId::new(15)?;
///
///     Validator::Online(Box::new(OnlineValidator::new(
///         client,
///         device_id,
///         OnlineValidatorConfig::default()
///     )))
/// } else {
///     Validator::Offline(OfflineValidator::new(pool))
/// };
///
/// // Use the same interface regardless of validator type
/// # let request = turnkey_protocol::commands::access::AccessRequest::new(
/// #     "1234567890".to_string(),
/// #     turnkey_core::HenryTimestamp::parse("10/05/2025 12:46:06")?,
/// #     turnkey_core::AccessDirection::Entry,
/// #     turnkey_core::ReaderType::Rfid,
/// # )?;
/// let response = validator.validate(&request).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub enum Validator {
    /// Online validator using TCP communication
    Online(Box<OnlineValidator>),

    /// Offline validator using local database
    Offline(OfflineValidator),
}

impl Validator {
    /// Validate an access request using the selected validator
    ///
    /// This method delegates to the underlying validator implementation
    /// based on the enum variant.
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
    /// Returns error if validation cannot be completed (database failure,
    /// network error, etc.). Validation failures (e.g., card not found)
    /// return `Ok(AccessResponse::deny)`, not errors.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use turnkey_storage::{Validator, OfflineValidator};
    /// use turnkey_protocol::commands::access::AccessRequest;
    /// use turnkey_core::{AccessDirection, HenryTimestamp, ReaderType};
    ///
    /// # async fn example(pool: sqlx::SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
    /// let mut validator = Validator::Offline(OfflineValidator::new(pool));
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
    pub async fn validate(&mut self, request: &AccessRequest) -> StorageResult<AccessResponse> {
        match self {
            Validator::Online(v) => v.validate(request).await,
            Validator::Offline(v) => v.validate(request).await,
        }
    }
}

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
/// use turnkey_storage::AccessValidator;
/// use turnkey_protocol::commands::access::AccessRequest;
/// use turnkey_core::{AccessDirection, HenryTimestamp, ReaderType};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let config = DatabaseConfig::new("turnkey.db");
/// let db = Database::new(config).await?;
/// let mut validator = OfflineValidator::new(db.pool().clone());
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

impl std::fmt::Debug for OfflineValidator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OfflineValidator").finish_non_exhaustive()
    }
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
    /// This is the internal implementation. The public API is available
    /// through the [`AccessValidator`] trait implementation.
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
    async fn validate_internal(&self, request: &AccessRequest) -> StorageResult<AccessResponse> {
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

/// Implement AccessValidator trait for OfflineValidator
impl AccessValidator for OfflineValidator {
    async fn validate(&mut self, request: &AccessRequest) -> StorageResult<AccessResponse> {
        // Delegate to internal implementation
        // Note: internal method uses &self, trait requires &mut self for consistency
        self.validate_internal(request).await
    }
}

/// Configuration for online validator
///
/// Controls retry behavior and fallback strategy for network-based validation.
///
/// # Examples
///
/// ```
/// use turnkey_storage::OnlineValidatorConfig;
/// use std::time::Duration;
///
/// // Default configuration
/// let config = OnlineValidatorConfig::default();
/// assert_eq!(config.max_retries, 2);
/// assert_eq!(config.retry_delay, Duration::from_millis(500));
/// assert!(!config.fallback_to_offline);
///
/// // Custom configuration
/// let config = OnlineValidatorConfig {
///     max_retries: 1,
///     retry_delay: Duration::from_millis(1000),
///     fallback_to_offline: true,
/// };
/// ```
#[derive(Debug, Clone)]
pub struct OnlineValidatorConfig {
    /// Maximum retry attempts on network failure (default: 2)
    ///
    /// Total attempts will be `1 + max_retries`. For example:
    /// - max_retries = 0: Try once, no retries
    /// - max_retries = 2: Try 3 times total (1 initial + 2 retries)
    pub max_retries: usize,

    /// Delay between retry attempts (default: 500ms)
    ///
    /// Fixed delay between retries. Simple and predictable for
    /// development/testing environments.
    pub retry_delay: Duration,

    /// Enable fallback to offline mode on failure (default: false)
    ///
    /// If true, validator will attempt offline validation after
    /// all network retries are exhausted.
    pub fallback_to_offline: bool,
}

impl Default for OnlineValidatorConfig {
    fn default() -> Self {
        Self {
            max_retries: 2,
            retry_delay: Duration::from_millis(500),
            fallback_to_offline: false,
        }
    }
}

/// Online validator for access control requests
///
/// Validates access requests by sending them to a remote validation server
/// via TCP and waiting for responses. This validator is used when the
/// turnstile emulator operates in ONLINE mode.
///
/// # Validation Flow
///
/// 1. Connect to server (if not already connected)
/// 2. Convert AccessRequest → Henry protocol Message
/// 3. Send message to server
/// 4. Receive response (with timeout)
/// 5. Convert response Message → AccessResponse
/// 6. On failure: retry up to max_retries times
/// 7. If all retries fail and fallback enabled: use OfflineValidator
///
/// # Retry Logic
///
/// Simple fixed-delay retry strategy suitable for emulator use:
/// - Configurable max retry attempts (default: 2)
/// - Fixed delay between retries (default: 500ms)
/// - Clear error messages on persistent failure
///
/// # Examples
///
/// ## Basic Online Validation
///
/// ```ignore
/// use turnkey_storage::{OnlineValidator, OnlineValidatorConfig, AccessValidator};
/// use turnkey_network::{TcpClient, TcpClientConfig};
/// use turnkey_core::DeviceId;
/// use std::time::Duration;
///
/// // Create TCP client
/// let client_config = TcpClientConfig {
///     server_addr: "192.168.0.100:3000".parse()?,
///     timeout: Duration::from_millis(3000),
/// };
/// let tcp_client = TcpClient::new(client_config);
///
/// // Create online validator
/// let device_id = DeviceId::new(15)?;
/// let mut validator = OnlineValidator::new(
///     tcp_client,
///     device_id,
///     OnlineValidatorConfig::default()
/// );
///
/// let response = validator.validate(&request).await?;
/// ```
///
/// ## Hybrid Mode with Offline Fallback
///
/// ```ignore
/// use turnkey_storage::{OnlineValidator, OnlineValidatorConfig, OfflineValidator, AccessValidator};
/// use turnkey_network::{TcpClient, TcpClientConfig};
/// use turnkey_core::DeviceId;
///
/// let config = OnlineValidatorConfig {
///     max_retries: 2,
///     fallback_to_offline: true,
///     ..Default::default()
/// };
///
/// let offline = OfflineValidator::new(pool);
/// let mut validator = OnlineValidator::with_fallback(
///     tcp_client,
///     device_id,
///     config,
///     offline
/// );
///
/// // If network fails after retries, automatically falls back to offline
/// let response = validator.validate(&request).await?;
/// ```
pub struct OnlineValidator {
    tcp_client: TcpClient,
    device_id: DeviceId,
    config: OnlineValidatorConfig,
    offline_fallback: Option<OfflineValidator>,
}

impl std::fmt::Debug for OnlineValidator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OnlineValidator")
            .field("device_id", &self.device_id)
            .field("config", &self.config)
            .field("has_offline_fallback", &self.offline_fallback.is_some())
            .finish_non_exhaustive()
    }
}

impl OnlineValidator {
    /// Create a new online validator without offline fallback
    ///
    /// # Arguments
    ///
    /// * `tcp_client` - TCP client for server communication
    /// * `device_id` - Device ID to use in protocol messages
    /// * `config` - Validator configuration
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use turnkey_storage::{OnlineValidator, OnlineValidatorConfig};
    /// use turnkey_network::{TcpClient, TcpClientConfig};
    /// use turnkey_core::DeviceId;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let tcp_client = TcpClient::new(TcpClientConfig::default());
    /// let device_id = DeviceId::new(1)?;
    /// let validator = OnlineValidator::new(
    ///     tcp_client,
    ///     device_id,
    ///     OnlineValidatorConfig::default()
    /// );
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(tcp_client: TcpClient, device_id: DeviceId, config: OnlineValidatorConfig) -> Self {
        Self {
            tcp_client,
            device_id,
            config,
            offline_fallback: None,
        }
    }

    /// Create a new online validator with offline fallback support
    ///
    /// If network validation fails after all retries, the offline validator
    /// will be used as a fallback.
    ///
    /// # Arguments
    ///
    /// * `tcp_client` - TCP client for server communication
    /// * `device_id` - Device ID to use in protocol messages
    /// * `config` - Validator configuration (fallback_to_offline should be true)
    /// * `offline_validator` - Offline validator for fallback
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use turnkey_storage::{OnlineValidator, OnlineValidatorConfig, OfflineValidator};
    /// use turnkey_network::{TcpClient, TcpClientConfig};
    /// use turnkey_core::DeviceId;
    ///
    /// let config = OnlineValidatorConfig {
    ///     fallback_to_offline: true,
    ///     ..Default::default()
    /// };
    ///
    /// let offline = OfflineValidator::new(pool);
    /// let validator = OnlineValidator::with_fallback(
    ///     tcp_client,
    ///     device_id,
    ///     config,
    ///     offline
    /// );
    /// ```
    pub fn with_fallback(
        tcp_client: TcpClient,
        device_id: DeviceId,
        config: OnlineValidatorConfig,
        offline_validator: OfflineValidator,
    ) -> Self {
        Self {
            tcp_client,
            device_id,
            config,
            offline_fallback: Some(offline_validator),
        }
    }

    /// Attempt validation with retry logic
    ///
    /// Retries network operations up to `max_retries` times with
    /// fixed delays. If all retries fail and fallback is enabled,
    /// attempts offline validation.
    async fn validate_with_retry(
        &mut self,
        request: &AccessRequest,
    ) -> StorageResult<AccessResponse> {
        let mut attempts = 0;
        let mut last_error = None;

        while attempts <= self.config.max_retries {
            match self.validate_once(request).await {
                Ok(response) => return Ok(response),
                Err(e) => {
                    last_error = Some(e);
                    attempts += 1;

                    if attempts <= self.config.max_retries {
                        tokio::time::sleep(self.config.retry_delay).await;
                    }
                }
            }
        }

        // If fallback enabled, try offline validation
        if self.config.fallback_to_offline
            && let Some(ref mut offline) = self.offline_fallback
        {
            return offline.validate(request).await;
        }

        // Otherwise, return error
        Err(crate::error::StorageError::ValidationFailed(
            self.config.max_retries,
            last_error
                .map(|e| e.to_string())
                .unwrap_or_else(|| "Unknown error".to_string()),
        ))
    }

    /// Single validation attempt without retry
    ///
    /// Performs one complete validation cycle:
    /// 1. Connect if not connected
    /// 2. Convert request to message
    /// 3. Send message
    /// 4. Receive response
    /// 5. Convert message to response
    async fn validate_once(&mut self, request: &AccessRequest) -> StorageResult<AccessResponse> {
        // Step 1: Connect if not connected
        if !self.tcp_client.is_connected() {
            self.tcp_client.connect().await.map_err(|e| {
                crate::error::StorageError::NetworkError(format!("Connection failed: {}", e))
            })?;
        }

        // Step 2: Convert AccessRequest → Message
        let message = Self::request_to_message(request, self.device_id)?;

        // Step 3: Send request
        self.tcp_client
            .send(message)
            .await
            .map_err(|e| crate::error::StorageError::NetworkError(format!("Send failed: {}", e)))?;

        // Step 4: Receive response (with timeout from TcpClient)
        let response_msg = self.tcp_client.recv().await.map_err(|e| {
            crate::error::StorageError::NetworkError(format!("Receive failed: {}", e))
        })?;

        // Step 5: Convert Message → AccessResponse
        Self::message_to_response(&response_msg)
    }

    /// Create a FieldData with improved error messages
    ///
    /// Helper method to create FieldData with context-specific error messages.
    ///
    /// # Arguments
    ///
    /// * `value` - The value to convert to FieldData
    /// * `context` - Description of the field for error messages
    ///
    /// # Errors
    ///
    /// Returns `ProtocolError` if the value contains reserved delimiters
    fn field_data(value: impl ToString, context: &str) -> StorageResult<FieldData> {
        FieldData::new(value.to_string())
            .map_err(|e| StorageError::ProtocolError(format!("{}: {}", context, e)))
    }

    /// Convert AccessRequest to Henry protocol Message
    ///
    /// Builds a Henry protocol access request message (command 000+0)
    /// with the following fields:
    /// - Field 1: Card number
    /// - Field 2: Timestamp
    /// - Field 3: Direction (0=undefined, 1=entry, 2=exit)
    /// - Field 4: Reader type (0=RFID, 1=biometric)
    fn request_to_message(request: &AccessRequest, device_id: DeviceId) -> StorageResult<Message> {
        let direction_value = match request.direction() {
            turnkey_core::AccessDirection::Undefined => "0",
            turnkey_core::AccessDirection::Entry => "1",
            turnkey_core::AccessDirection::Exit => "2",
        };

        let reader_value = match request.reader_type() {
            turnkey_core::ReaderType::Rfid => "0",
            turnkey_core::ReaderType::Biometric => "1",
        };

        MessageBuilder::new(device_id, CommandCode::AccessRequest)
            .field(Self::field_data(
                request.card_number(),
                "Invalid card number",
            )?)
            .field(Self::field_data(request.timestamp(), "Invalid timestamp")?)
            .field(Self::field_data(direction_value, "Invalid direction")?)
            .field(Self::field_data(reader_value, "Invalid reader type")?)
            .build()
            .map_err(|e| StorageError::ProtocolError(format!("Failed to build message: {}", e)))
    }

    /// Convert Henry protocol Message to AccessResponse
    ///
    /// Parses the response message and creates an appropriate
    /// AccessResponse (grant or deny).
    ///
    /// Expected response format:
    /// - Grant entry: 00+5]seconds]message
    /// - Grant exit: 00+6]seconds]message
    /// - Grant both: 00+1]seconds]message
    /// - Deny: 00+30]seconds]message
    fn message_to_response(message: &Message) -> StorageResult<AccessResponse> {
        // Check command code to determine grant/deny
        let is_grant = matches!(
            message.command,
            CommandCode::GrantBoth | CommandCode::GrantEntry | CommandCode::GrantExit
        );

        let is_deny = matches!(message.command, CommandCode::DenyAccess);

        // Extract display message from fields
        // Typical format: field[0]=seconds, field[1]=message
        let display_message = if message.fields.len() >= 2 {
            message.fields[1].to_string()
        } else if message.fields.len() == 1 {
            message.fields[0].to_string()
        } else {
            "Unknown".to_string()
        };

        if is_grant {
            match message.command {
                CommandCode::GrantEntry => Ok(AccessResponse::grant_entry(display_message)),
                CommandCode::GrantExit => Ok(AccessResponse::grant_exit(display_message)),
                CommandCode::GrantBoth => Ok(AccessResponse::grant_both(display_message)),
                _ => Ok(AccessResponse::deny(display_message)),
            }
        } else if is_deny {
            Ok(AccessResponse::deny(display_message))
        } else {
            Err(crate::error::StorageError::ProtocolError(format!(
                "Unexpected command code in response: {:?}",
                message.command
            )))
        }
    }
}

/// Implement AccessValidator trait for OnlineValidator
impl AccessValidator for OnlineValidator {
    async fn validate(&mut self, request: &AccessRequest) -> StorageResult<AccessResponse> {
        self.validate_with_retry(request).await
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

        let mut validator = OfflineValidator::new(db.pool().clone());
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

        let mut validator = OfflineValidator::new(db.pool().clone());
        let request = create_access_request("2222222222", AccessDirection::Exit);

        let response = validator.validate(&request).await.unwrap();
        assert!(response.is_grant());
    }

    #[tokio::test]
    async fn test_validate_card_not_found() {
        let db = setup_test_db().await;
        let mut validator = OfflineValidator::new(db.pool().clone());
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

        let mut validator = OfflineValidator::new(db.pool().clone());
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

        let mut validator = OfflineValidator::new(db.pool().clone());
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

        let mut validator = OfflineValidator::new(db.pool().clone());
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

        let mut validator = OfflineValidator::new(db.pool().clone());
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

        let mut validator = OfflineValidator::new(db.pool().clone());
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
        let mut validator = OfflineValidator::new(db.pool().clone());
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

        let mut validator = OfflineValidator::new(db.pool().clone());
        // Request with lowercase card number
        let request = create_access_request("abcdef1234", AccessDirection::Entry);

        let response = validator.validate(&request).await.unwrap();
        assert!(response.is_grant());
    }

    // OnlineValidator tests
    use turnkey_network::TcpClientConfig;

    #[test]
    fn test_online_validator_config_default() {
        let config = OnlineValidatorConfig::default();
        assert_eq!(config.max_retries, 2);
        assert_eq!(config.retry_delay, std::time::Duration::from_millis(500));
        assert!(!config.fallback_to_offline);
    }

    #[test]
    fn test_online_validator_creation() {
        let client_config = TcpClientConfig::default();
        let tcp_client = TcpClient::new(client_config);
        let device_id = DeviceId::new(1).unwrap();

        let validator =
            OnlineValidator::new(tcp_client, device_id, OnlineValidatorConfig::default());

        assert!(validator.offline_fallback.is_none());
    }

    #[tokio::test]
    async fn test_online_validator_with_fallback() {
        let db = setup_test_db().await;
        let offline = OfflineValidator::new(db.pool().clone());

        let client_config = TcpClientConfig::default();
        let tcp_client = TcpClient::new(client_config);
        let device_id = DeviceId::new(1).unwrap();

        let config = OnlineValidatorConfig {
            fallback_to_offline: true,
            ..Default::default()
        };

        let validator = OnlineValidator::with_fallback(tcp_client, device_id, config, offline);

        assert!(validator.offline_fallback.is_some());
        assert!(validator.config.fallback_to_offline);
    }

    #[test]
    fn test_request_to_message_entry() {
        let timestamp = HenryTimestamp::parse("10/05/2025 12:46:06").unwrap();
        let request = AccessRequest::new(
            "1234567890".to_string(),
            timestamp,
            AccessDirection::Entry,
            turnkey_core::ReaderType::Rfid,
        )
        .unwrap();

        let device_id = DeviceId::new(15).unwrap();
        let message = OnlineValidator::request_to_message(&request, device_id).unwrap();

        assert_eq!(message.device_id, device_id);
        assert_eq!(message.command, CommandCode::AccessRequest);
        assert_eq!(message.fields.len(), 4);
        assert_eq!(message.fields[0].as_str(), "1234567890");
        assert_eq!(message.fields[1].as_str(), "10/05/2025 12:46:06");
        assert_eq!(message.fields[2].as_str(), "1"); // Entry
        assert_eq!(message.fields[3].as_str(), "0"); // RFID
    }

    #[test]
    fn test_request_to_message_exit() {
        let timestamp = HenryTimestamp::parse("10/05/2025 12:46:06").unwrap();
        let request = AccessRequest::new(
            "9876543210".to_string(),
            timestamp,
            AccessDirection::Exit,
            turnkey_core::ReaderType::Biometric,
        )
        .unwrap();

        let device_id = DeviceId::new(1).unwrap();
        let message = OnlineValidator::request_to_message(&request, device_id).unwrap();

        assert_eq!(message.device_id, device_id);
        assert_eq!(message.fields[2].as_str(), "2"); // Exit
        assert_eq!(message.fields[3].as_str(), "1"); // Biometric
    }

    #[test]
    fn test_message_to_response_grant_entry() {
        let device_id = DeviceId::new(15).unwrap();
        let message = MessageBuilder::new(device_id, CommandCode::GrantEntry)
            .field(FieldData::new("5".to_string()).unwrap())
            .field(FieldData::new("Acesso liberado".to_string()).unwrap())
            .build()
            .unwrap();

        let response = OnlineValidator::message_to_response(&message).unwrap();

        assert!(response.is_grant());
        assert_eq!(response.display_message(), "Acesso liberado");
    }

    #[test]
    fn test_message_to_response_grant_exit() {
        let device_id = DeviceId::new(15).unwrap();
        let message = MessageBuilder::new(device_id, CommandCode::GrantExit)
            .field(FieldData::new("5".to_string()).unwrap())
            .field(FieldData::new("Acesso liberado saida".to_string()).unwrap())
            .build()
            .unwrap();

        let response = OnlineValidator::message_to_response(&message).unwrap();

        assert!(response.is_grant());
        assert_eq!(response.display_message(), "Acesso liberado saida");
    }

    #[test]
    fn test_message_to_response_deny() {
        let device_id = DeviceId::new(15).unwrap();
        let message = MessageBuilder::new(device_id, CommandCode::DenyAccess)
            .field(FieldData::new("0".to_string()).unwrap())
            .field(FieldData::new("Acesso negado".to_string()).unwrap())
            .build()
            .unwrap();

        let response = OnlineValidator::message_to_response(&message).unwrap();

        assert!(response.is_deny());
        assert_eq!(response.display_message(), "Acesso negado");
    }

    #[test]
    fn test_message_to_response_invalid_command() {
        let device_id = DeviceId::new(15).unwrap();
        let message = MessageBuilder::new(device_id, CommandCode::QueryStatus)
            .build()
            .unwrap();

        let result = OnlineValidator::message_to_response(&message);

        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(crate::error::StorageError::ProtocolError(_))
        ));
    }
}
