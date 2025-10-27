//! Transaction-aware repository operations for atomic multistep operations.
//!
//! These functions accept a SQLite transaction reference, allowing multiple
//! repository operations to be grouped into a single atomic transaction.
//! This is critical for maintaining data consistency during bulk imports
//! and complex multi-table operations.
//!
//! # When to Use Transactions
//!
//! Use transactions when you need to:
//! - **Bulk Import**: Load multiple users/cards from files atomically
//! - **User Creation**: Create user + assign cards in one operation
//! - **Data Migration**: Transfer data between tables with consistency
//! - **Referential Integrity**: Ensure foreign keys remain valid
//!
//! # Usage Pattern
//!
//! ```no_run
//! use turnkey_storage::{Database, DatabaseConfig};
//! use turnkey_storage::transaction;
//! use turnkey_storage::models::{User, Card};
//! use chrono::Utc;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let config = DatabaseConfig::new("turnkey.db");
//! let db = Database::new(config).await?;
//!
//! # // Create example user and card for documentation
//! # let user = User {
//! #     id: 0,
//! #     pis: None,
//! #     nome: "Test User".to_string(),
//! #     matricula: "EMP001".to_string(),
//! #     cpf: None,
//! #     validade_inicio: None,
//! #     validade_fim: None,
//! #     ativo: true,
//! #     allow_card: true,
//! #     allow_bio: false,
//! #     allow_keypad: false,
//! #     codigo: None,
//! #     created_at: Utc::now(),
//! #     updated_at: Utc::now(),
//! # };
//! # let card = Card {
//! #     id: 0,
//! #     numero_cartao: "1234567890".to_string(),
//! #     matricula: "EMP001".to_string(),
//! #     user_id: 1,
//! #     validade_inicio: None,
//! #     validade_fim: None,
//! #     ativo: true,
//! #     created_at: Utc::now(),
//! #     updated_at: Utc::now(),
//! # };
//! // Begin transaction
//! let mut tx = db.pool().begin().await?;
//!
//! // Perform multiple operations atomically
//! let user_id = transaction::create_user(&mut tx, &user).await?;
//! transaction::create_card(&mut tx, &card).await?;
//!
//! // Commit transaction - both operations succeed or both fail
//! tx.commit().await?;
//! # Ok(())
//! # }
//! ```
//!
//! # Atomic Guarantees
//!
//! All operations within a transaction are guaranteed to either all succeed
//! or all fail. If any operation returns an error, the transaction should be
//! rolled back by dropping it or calling `rollback()`.

use crate::error::StorageResult;
use crate::models::{AccessLog, Card, User};
use sqlx::{Sqlite, Transaction};

/// Create a new user within a transaction
///
/// # Arguments
///
/// * `tx` - Mutable reference to an active SQLite transaction
/// * `user` - User entity to create
///
/// # Returns
///
/// Returns the auto-generated user ID on success
///
/// # Errors
///
/// Returns error if:
/// - Unique constraint violation (duplicate matricula)
/// - Database constraints violated
/// - Transaction is already committed or rolled back
pub async fn create_user(tx: &mut Transaction<'_, Sqlite>, user: &User) -> StorageResult<i64> {
    let result = sqlx::query(
        r#"
        INSERT INTO users (
            pis, nome, matricula, cpf,
            validade_inicio, validade_fim, ativo,
            allow_card, allow_bio, allow_keypad, codigo
        )
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&user.pis)
    .bind(&user.nome)
    .bind(&user.matricula)
    .bind(&user.cpf)
    .bind(user.validade_inicio)
    .bind(user.validade_fim)
    .bind(user.ativo)
    .bind(user.allow_card)
    .bind(user.allow_bio)
    .bind(user.allow_keypad)
    .bind(&user.codigo)
    .execute(&mut **tx)
    .await?;

    Ok(result.last_insert_rowid())
}

/// Create a new card within a transaction
///
/// # Arguments
///
/// * `tx` - Mutable reference to an active SQLite transaction
/// * `card` - Card entity to create
///
/// # Returns
///
/// Returns the auto-generated card ID on success
///
/// # Errors
///
/// Returns error if:
/// - Unique constraint violation (duplicate numero_cartao)
/// - Foreign key constraint violation (invalid user_id or matricula)
/// - Dual-key consistency check fails
/// - Transaction is already committed or rolled back
pub async fn create_card(tx: &mut Transaction<'_, Sqlite>, card: &Card) -> StorageResult<i64> {
    let result = sqlx::query(
        r#"
        INSERT INTO cards (
            numero_cartao, matricula, user_id,
            validade_inicio, validade_fim, ativo
        )
        VALUES (?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&card.numero_cartao)
    .bind(&card.matricula)
    .bind(card.user_id)
    .bind(card.validade_inicio)
    .bind(card.validade_fim)
    .bind(card.ativo)
    .execute(&mut **tx)
    .await?;

    Ok(result.last_insert_rowid())
}

/// Create a new access log entry within a transaction
///
/// # Arguments
///
/// * `tx` - Mutable reference to an active SQLite transaction
/// * `log` - AccessLog entity to create
///
/// # Returns
///
/// Returns the auto-generated log ID on success
///
/// # Errors
///
/// Returns error if:
/// - Foreign key constraint violation (invalid user_id or matricula)
/// - Transaction is already committed or rolled back
pub async fn create_access_log(
    tx: &mut Transaction<'_, Sqlite>,
    log: &AccessLog,
) -> StorageResult<i64> {
    let result = sqlx::query(
        r#"
        INSERT INTO access_logs (
            user_id, matricula, card_number, direction,
            reader_type, granted, display_message, timestamp
        )
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(log.user_id)
    .bind(&log.matricula)
    .bind(&log.card_number)
    .bind(log.direction)
    .bind(log.reader_type)
    .bind(log.granted)
    .bind(&log.display_message)
    .bind(log.timestamp)
    .execute(&mut **tx)
    .await?;

    Ok(result.last_insert_rowid())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::Database;
    use chrono::Utc;

    async fn setup_test_db() -> Database {
        Database::in_memory().await.unwrap()
    }

    #[tokio::test]
    async fn test_transaction_commit() {
        let db = setup_test_db().await;
        let mut tx = db.pool().begin().await.unwrap();

        let user = User {
            id: 0,
            pis: None,
            nome: "Transaction Test".to_string(),
            matricula: "TX001".to_string(),
            cpf: None,
            validade_inicio: None,
            validade_fim: None,
            ativo: true,
            allow_card: true,
            allow_bio: false,
            allow_keypad: false,
            codigo: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let user_id = create_user(&mut tx, &user).await.unwrap();
        assert!(user_id > 0);

        tx.commit().await.unwrap();

        // Verify user was persisted
        let found: Option<(i64,)> =
            sqlx::query_as("SELECT id FROM users WHERE matricula = 'TX001'")
                .fetch_optional(db.pool())
                .await
                .unwrap();

        assert!(found.is_some());
    }

    #[tokio::test]
    async fn test_transaction_rollback() {
        let db = setup_test_db().await;
        let mut tx = db.pool().begin().await.unwrap();

        let user = User {
            id: 0,
            pis: None,
            nome: "Rollback Test".to_string(),
            matricula: "TX002".to_string(),
            cpf: None,
            validade_inicio: None,
            validade_fim: None,
            ativo: true,
            allow_card: true,
            allow_bio: false,
            allow_keypad: false,
            codigo: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        create_user(&mut tx, &user).await.unwrap();

        // Explicitly rollback
        tx.rollback().await.unwrap();

        // Verify user was not persisted
        let found: Option<(i64,)> =
            sqlx::query_as("SELECT id FROM users WHERE matricula = 'TX002'")
                .fetch_optional(db.pool())
                .await
                .unwrap();

        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_transaction_multiple_operations() {
        let db = setup_test_db().await;
        let mut tx = db.pool().begin().await.unwrap();

        let user = User {
            id: 0,
            pis: None,
            nome: "Multi Op Test".to_string(),
            matricula: "TX003".to_string(),
            cpf: None,
            validade_inicio: None,
            validade_fim: None,
            ativo: true,
            allow_card: true,
            allow_bio: false,
            allow_keypad: false,
            codigo: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let user_id = create_user(&mut tx, &user).await.unwrap();

        let card = Card {
            id: 0,
            numero_cartao: "TX003CARD".to_string(),
            matricula: "TX003".to_string(),
            user_id,
            validade_inicio: None,
            validade_fim: None,
            ativo: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let card_id = create_card(&mut tx, &card).await.unwrap();
        assert!(card_id > 0);

        tx.commit().await.unwrap();

        // Verify both entities persisted
        let user_found: Option<(i64,)> =
            sqlx::query_as("SELECT id FROM users WHERE matricula = 'TX003'")
                .fetch_optional(db.pool())
                .await
                .unwrap();

        let card_found: Option<(i64,)> =
            sqlx::query_as("SELECT id FROM cards WHERE numero_cartao = 'TX003CARD'")
                .fetch_optional(db.pool())
                .await
                .unwrap();

        assert!(user_found.is_some());
        assert!(card_found.is_some());
    }
}
