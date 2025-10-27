#![allow(async_fn_in_trait)]

use crate::error::StorageResult;
use crate::models::AccessLog;
use chrono::{DateTime, Utc};
use sqlx::SqlitePool;

/// Repository trait for AccessLog entity operations
///
/// This trait defines the contract for access log data access, supporting
/// audit trail and security monitoring requirements.
///
/// # Implementation Note
///
/// This trait uses native async trait methods (Edition 2024 feature),
/// eliminating the need for the async-trait crate while maintaining
/// full async/await support in trait methods.
pub trait AccessLogRepository: Send + Sync {
    /// Create a new access log entry
    async fn create(&self, log: &AccessLog) -> StorageResult<i64>;

    /// Find access logs by user ID
    async fn find_by_user_id(&self, user_id: i64, limit: i64) -> StorageResult<Vec<AccessLog>>;

    /// Find access logs by card number
    async fn find_by_card_number(
        &self,
        card_number: &str,
        limit: i64,
    ) -> StorageResult<Vec<AccessLog>>;

    /// Find recent denied accesses (security monitoring)
    async fn find_recent_denied(&self, limit: i64) -> StorageResult<Vec<AccessLog>>;

    /// Find recent granted accesses
    async fn find_recent_granted(&self, limit: i64) -> StorageResult<Vec<AccessLog>>;

    /// Find all access logs within a time range
    async fn find_by_time_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> StorageResult<Vec<AccessLog>>;

    /// Count total access attempts in a time range
    async fn count_by_time_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> StorageResult<i64>;

    /// Count denied access attempts for a specific card
    async fn count_denied_by_card(
        &self,
        card_number: &str,
        since: DateTime<Utc>,
    ) -> StorageResult<i64>;
}

/// SQLite implementation of AccessLogRepository
pub struct SqliteAccessLogRepository {
    pool: SqlitePool,
}

impl SqliteAccessLogRepository {
    /// Create a new SQLite access log repository
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

impl AccessLogRepository for SqliteAccessLogRepository {
    async fn create(&self, log: &AccessLog) -> StorageResult<i64> {
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
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    async fn find_by_user_id(&self, user_id: i64, limit: i64) -> StorageResult<Vec<AccessLog>> {
        let logs = sqlx::query_as::<_, AccessLog>(
            r#"
            SELECT id, user_id, matricula, card_number,
                   direction, reader_type, granted,
                   display_message, timestamp, created_at
            FROM access_logs
            WHERE user_id = ?
            ORDER BY timestamp DESC
            LIMIT ?
            "#,
        )
        .bind(user_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(logs)
    }

    async fn find_by_card_number(
        &self,
        card_number: &str,
        limit: i64,
    ) -> StorageResult<Vec<AccessLog>> {
        let logs = sqlx::query_as::<_, AccessLog>(
            r#"
            SELECT id, user_id, matricula, card_number,
                   direction, reader_type, granted,
                   display_message, timestamp, created_at
            FROM access_logs
            WHERE card_number = ?
            ORDER BY timestamp DESC
            LIMIT ?
            "#,
        )
        .bind(card_number)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(logs)
    }

    async fn find_recent_denied(&self, limit: i64) -> StorageResult<Vec<AccessLog>> {
        let logs = sqlx::query_as::<_, AccessLog>(
            r#"
            SELECT id, user_id, matricula, card_number,
                   direction, reader_type, granted,
                   display_message, timestamp, created_at
            FROM access_logs
            WHERE granted = 0
            ORDER BY timestamp DESC
            LIMIT ?
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(logs)
    }

    async fn find_recent_granted(&self, limit: i64) -> StorageResult<Vec<AccessLog>> {
        let logs = sqlx::query_as::<_, AccessLog>(
            r#"
            SELECT id, user_id, matricula, card_number,
                   direction, reader_type, granted,
                   display_message, timestamp, created_at
            FROM access_logs
            WHERE granted = 1
            ORDER BY timestamp DESC
            LIMIT ?
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(logs)
    }

    async fn find_by_time_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> StorageResult<Vec<AccessLog>> {
        let logs = sqlx::query_as::<_, AccessLog>(
            r#"
            SELECT id, user_id, matricula, card_number,
                   direction, reader_type, granted,
                   display_message, timestamp, created_at
            FROM access_logs
            WHERE timestamp >= ? AND timestamp <= ?
            ORDER BY timestamp DESC
            "#,
        )
        .bind(start)
        .bind(end)
        .fetch_all(&self.pool)
        .await?;

        Ok(logs)
    }

    async fn count_by_time_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> StorageResult<i64> {
        let result: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM access_logs WHERE timestamp >= ? AND timestamp <= ?",
        )
        .bind(start)
        .bind(end)
        .fetch_one(&self.pool)
        .await?;

        Ok(result.0)
    }

    async fn count_denied_by_card(
        &self,
        card_number: &str,
        since: DateTime<Utc>,
    ) -> StorageResult<i64> {
        let result: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM access_logs WHERE card_number = ? AND granted = 0 AND timestamp >= ?",
        )
        .bind(card_number)
        .bind(since)
        .fetch_one(&self.pool)
        .await?;

        Ok(result.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::Database;
    use crate::models::{Card, Direction, ReaderType, User};
    use crate::repositories::card::{CardRepository, SqliteCardRepository};
    use crate::repositories::user::{SqliteUserRepository, UserRepository};
    use chrono::Duration;

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

        let repo = SqliteUserRepository::new(db.pool().clone());
        repo.create(&user).await.unwrap()
    }

    async fn create_test_card(db: &Database, numero: &str, matricula: &str, user_id: i64) {
        let card = Card {
            id: 0,
            numero_cartao: numero.to_string(),
            matricula: matricula.to_string(),
            user_id,
            validade_inicio: None,
            validade_fim: None,
            ativo: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let repo = SqliteCardRepository::new(db.pool().clone());
        repo.create(&card).await.unwrap();
    }

    fn create_test_log(
        user_id: i64,
        matricula: &str,
        card_number: &str,
        granted: bool,
    ) -> AccessLog {
        AccessLog::new(
            Some(user_id),
            Some(matricula.to_string()),
            card_number.to_string(),
            Direction::Entry,
            ReaderType::Rfid,
            granted,
            Some(if granted {
                "Acesso liberado".to_string()
            } else {
                "Acesso negado".to_string()
            }),
            Utc::now(),
        )
    }

    #[tokio::test]
    async fn test_create_access_log() {
        let db = setup_test_db().await;
        let user_id = create_test_user(&db, "EMP001").await;
        create_test_card(&db, "1234567890", "EMP001", user_id).await;

        let repo = SqliteAccessLogRepository::new(db.pool().clone());
        let log = create_test_log(user_id, "EMP001", "1234567890", true);

        let id = repo.create(&log).await.unwrap();
        assert!(id > 0);
    }

    #[tokio::test]
    async fn test_find_by_user_id() {
        let db = setup_test_db().await;
        let user_id = create_test_user(&db, "EMP002").await;
        create_test_card(&db, "2222222222", "EMP002", user_id).await;

        let repo = SqliteAccessLogRepository::new(db.pool().clone());
        repo.create(&create_test_log(user_id, "EMP002", "2222222222", true))
            .await
            .unwrap();
        repo.create(&create_test_log(user_id, "EMP002", "2222222222", false))
            .await
            .unwrap();

        let logs = repo.find_by_user_id(user_id, 10).await.unwrap();
        assert_eq!(logs.len(), 2);
    }

    #[tokio::test]
    async fn test_find_by_card_number() {
        let db = setup_test_db().await;
        let user_id = create_test_user(&db, "EMP003").await;
        create_test_card(&db, "3333333333", "EMP003", user_id).await;

        let repo = SqliteAccessLogRepository::new(db.pool().clone());
        repo.create(&create_test_log(user_id, "EMP003", "3333333333", true))
            .await
            .unwrap();

        let logs = repo.find_by_card_number("3333333333", 10).await.unwrap();
        assert_eq!(logs.len(), 1);
    }

    #[tokio::test]
    async fn test_find_recent_denied() {
        let db = setup_test_db().await;
        let user_id = create_test_user(&db, "EMP004").await;
        create_test_card(&db, "4444444444", "EMP004", user_id).await;

        let repo = SqliteAccessLogRepository::new(db.pool().clone());
        repo.create(&create_test_log(user_id, "EMP004", "4444444444", false))
            .await
            .unwrap();

        let logs = repo.find_recent_denied(10).await.unwrap();
        assert!(!logs.is_empty());
        assert!(!logs[0].granted);
    }

    #[tokio::test]
    async fn test_find_recent_granted() {
        let db = setup_test_db().await;
        let user_id = create_test_user(&db, "EMP005").await;
        create_test_card(&db, "5555555555", "EMP005", user_id).await;

        let repo = SqliteAccessLogRepository::new(db.pool().clone());
        repo.create(&create_test_log(user_id, "EMP005", "5555555555", true))
            .await
            .unwrap();

        let logs = repo.find_recent_granted(10).await.unwrap();
        assert!(!logs.is_empty());
        assert!(logs[0].granted);
    }

    #[tokio::test]
    async fn test_find_by_time_range() {
        let db = setup_test_db().await;
        let user_id = create_test_user(&db, "EMP006").await;
        create_test_card(&db, "6666666666", "EMP006", user_id).await;

        let repo = SqliteAccessLogRepository::new(db.pool().clone());
        repo.create(&create_test_log(user_id, "EMP006", "6666666666", true))
            .await
            .unwrap();

        let start = Utc::now() - Duration::hours(1);
        let end = Utc::now() + Duration::hours(1);
        let logs = repo.find_by_time_range(start, end).await.unwrap();
        assert!(!logs.is_empty());
    }

    #[tokio::test]
    async fn test_count_by_time_range() {
        let db = setup_test_db().await;
        let user_id = create_test_user(&db, "EMP007").await;
        create_test_card(&db, "7777777777", "EMP007", user_id).await;

        let repo = SqliteAccessLogRepository::new(db.pool().clone());
        repo.create(&create_test_log(user_id, "EMP007", "7777777777", true))
            .await
            .unwrap();

        let start = Utc::now() - Duration::hours(1);
        let end = Utc::now() + Duration::hours(1);
        let count = repo.count_by_time_range(start, end).await.unwrap();
        assert!(count >= 1);
    }

    #[tokio::test]
    async fn test_count_denied_by_card() {
        let db = setup_test_db().await;
        let user_id = create_test_user(&db, "EMP008").await;
        create_test_card(&db, "8888888888", "EMP008", user_id).await;

        let repo = SqliteAccessLogRepository::new(db.pool().clone());
        repo.create(&create_test_log(user_id, "EMP008", "8888888888", false))
            .await
            .unwrap();

        let since = Utc::now() - Duration::hours(1);
        let count = repo
            .count_denied_by_card("8888888888", since)
            .await
            .unwrap();
        assert_eq!(count, 1);
    }
}
