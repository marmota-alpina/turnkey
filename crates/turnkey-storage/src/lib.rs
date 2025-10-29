//! Storage layer for the Turnkey access control system.
//!
//! This crate provides SQLite-backed persistence for users, cards, and access logs,
//! along with an offline validator that implements the complete Henry protocol
//! validation flow without requiring network connectivity.
//!
//! # Architecture
//!
//! The storage layer uses a repository pattern with the following components:
//!
//! - [`Database`] - Connection pool manager with automatic migrations
//! - [`UserRepository`], [`CardRepository`], [`AccessLogRepository`] - Data access traits
//! - [`OfflineValidator`] - 9-step validation flow implementation
//! - [`transaction`] - Transaction-aware operations for atomic multi-step operations
//!
//! # Core Concepts
//!
//! ## Dual-Key Strategy
//!
//! The database schema uses both natural keys (matricula as TEXT) and technical keys
//! (user_id as INTEGER) for optimal performance and compatibility with import formats:
//!
//! - TEXT keys (matricula, CPF, PIS) maintain compatibility with colaborador.txt format
//! - INTEGER keys (user_id, card_id) provide fast foreign key lookups
//! - Database triggers enforce consistency between both key types
//!
//! ## Repository Pattern
//!
//! All data access goes through repository traits, enabling:
//! - Easy mocking for unit tests
//! - Separation of business logic from persistence
//! - Transaction support for atomic operations
//!
//! # Examples
//!
//! ## Basic Setup and Offline Validation
//!
//! ```no_run
//! use turnkey_storage::{Database, DatabaseConfig, OfflineValidator, AccessValidator};
//! use turnkey_protocol::commands::access::AccessRequest;
//! use turnkey_core::{AccessDirection, HenryTimestamp, ReaderType};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Initialize database with automatic migrations
//! let config = DatabaseConfig::new("turnkey.db")
//!     .max_connections(10)
//!     .auto_migrate(true);
//!
//! let db = Database::new(config).await?;
//!
//! // Create offline validator
//! let mut validator = OfflineValidator::new(db.pool().clone());
//!
//! // Validate access request
//! let timestamp = HenryTimestamp::parse("27/10/2025 14:30:00")?;
//! let request = AccessRequest::new(
//!     "1234567890".to_string(),
//!     timestamp,
//!     AccessDirection::Entry,
//!     ReaderType::Rfid,
//! )?;
//!
//! let response = validator.validate(&request).await?;
//!
//! if response.is_grant() {
//!     println!("Access granted: {}", response.display_message());
//! } else {
//!     println!("Access denied: {}", response.display_message());
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Using Repositories Directly
//!
//! ```no_run
//! use turnkey_storage::{Database, DatabaseConfig};
//! use turnkey_storage::repositories::{UserRepository, SqliteUserRepository};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let config = DatabaseConfig::new("turnkey.db");
//! let db = Database::new(config).await?;
//!
//! let user_repo = SqliteUserRepository::new(db.pool().clone());
//!
//! // Find user by matricula
//! if let Some(user) = user_repo.find_by_matricula("EMP001").await? {
//!     println!("Found user: {} (active: {})", user.nome, user.ativo);
//!
//!     if user.can_use_card() {
//!         println!("User has card access enabled");
//!     }
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Transaction Support for Bulk Operations
//!
//! ```no_run
//! use turnkey_storage::{Database, DatabaseConfig, transaction};
//! use turnkey_storage::models::{User, Card};
//! use chrono::Utc;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let config = DatabaseConfig::new("turnkey.db");
//! let db = Database::new(config).await?;
//!
//! // Begin transaction for atomic multistep operation
//! let mut tx = db.pool().begin().await?;
//!
//! // Create user
//! let user = User {
//!     id: 0,
//!     pis: Some("12345678901".to_string()),
//!     nome: "John Doe".to_string(),
//!     matricula: "EMP001".to_string(),
//!     cpf: Some("12345678901".to_string()),
//!     validade_inicio: None,
//!     validade_fim: None,
//!     ativo: true,
//!     allow_card: true,
//!     allow_bio: false,
//!     allow_keypad: false,
//!     codigo: None,
//!     created_at: Utc::now(),
//!     updated_at: Utc::now(),
//! };
//!
//! let user_id = transaction::create_user(&mut tx, &user).await?;
//!
//! // Create card for the user
//! let card = Card {
//!     id: 0,
//!     numero_cartao: "1234567890".to_string(),
//!     matricula: "EMP001".to_string(),
//!     user_id,
//!     validade_inicio: None,
//!     validade_fim: None,
//!     ativo: true,
//!     created_at: Utc::now(),
//!     updated_at: Utc::now(),
//! };
//!
//! transaction::create_card(&mut tx, &card).await?;
//!
//! // Commit transaction (both operations succeed or both fail)
//! tx.commit().await?;
//! # Ok(())
//! # }
//! ```
//!
//! # Security Considerations
//!
//! ## Timing Attack Protection
//!
//! PIN code verification uses constant-time comparison via the `subtle` crate
//! to prevent timing side-channel attacks.
//!
//! ## SQL Injection Prevention
//!
//! All queries use parameterized statements via SQLx, providing compile-time
//! verification and preventing SQL injection attacks.
//!
//! ## Migration Path Security
//!
//! Migration paths are resolved at compile time using absolute paths to prevent
//! directory traversal attacks.
//!
//! # Performance
//!
//! - Connection pooling with configurable limits (default: 10 max, 2 min)
//! - WAL mode for better concurrent read/write performance
//! - Prepared statement caching
//! - Dual-key strategy for optimal foreign key performance
//! - Indexed columns for frequently queried fields
//!
//! # Database Schema Compatibility
//!
//! The schema is designed for full compatibility with Henry protocol import formats:
//! - Users table matches colaborador.txt format (issues #39)
//! - Cards table matches cartoes.txt format (issue #40)
//! - Biometric templates match biometria.txt format (issue #41)
//!
//! This ensures future import features can be implemented without schema migrations.

pub mod connection;
pub mod error;
pub mod messages;
pub mod models;
pub mod repositories;
pub mod transaction;
pub mod validator;

pub use connection::{Database, DatabaseConfig};
pub use error::{StorageError, StorageResult};
pub use messages::DisplayMessages;
pub use models::{AccessLog, Card, Direction, ReaderType, User};
pub use repositories::{
    AccessLogRepository, CardRepository, SqliteAccessLogRepository, SqliteCardRepository,
    SqliteUserRepository, UserRepository,
};
pub use validator::{
    AccessValidator, OfflineValidator, OnlineValidator, OnlineValidatorConfig, Validator,
};
