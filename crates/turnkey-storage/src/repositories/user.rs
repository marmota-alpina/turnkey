#![allow(async_fn_in_trait)]

use crate::error::{StorageError, StorageResult};
use crate::models::User;
use sqlx::SqlitePool;

/// Repository trait for User entity operations
///
/// This trait defines the contract for user data access, enabling
/// testability through mock implementations and separation of concerns.
///
/// # Implementation Note
///
/// This trait uses native async trait methods (Edition 2024 feature),
/// eliminating the need for the async-trait crate while maintaining
/// full async/await support in trait methods.
pub trait UserRepository: Send + Sync {
    /// Find a user by their matricula (employee ID)
    async fn find_by_matricula(&self, matricula: &str) -> StorageResult<Option<User>>;

    /// Find a user by their ID
    async fn find_by_id(&self, id: i64) -> StorageResult<Option<User>>;

    /// Find a user by their PIN code
    async fn find_by_code(&self, code: &str) -> StorageResult<Option<User>>;

    /// Get all active users
    async fn find_all_active(&self) -> StorageResult<Vec<User>>;

    /// Create a new user
    async fn create(&self, user: &User) -> StorageResult<i64>;

    /// Update an existing user
    async fn update(&self, user: &User) -> StorageResult<()>;

    /// Delete a user by ID
    async fn delete(&self, id: i64) -> StorageResult<()>;

    /// Check if a matricula already exists
    async fn exists_by_matricula(&self, matricula: &str) -> StorageResult<bool>;
}

/// SQLite implementation of UserRepository
pub struct SqliteUserRepository {
    pool: SqlitePool,
}

impl SqliteUserRepository {
    /// Create a new SQLite user repository
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

impl UserRepository for SqliteUserRepository {
    async fn find_by_matricula(&self, matricula: &str) -> StorageResult<Option<User>> {
        let user = sqlx::query_as::<_, User>(
            r#"
            SELECT id, pis, nome, matricula, cpf,
                   validade_inicio, validade_fim, ativo,
                   allow_card, allow_bio, allow_keypad, codigo,
                   created_at, updated_at
            FROM users
            WHERE matricula = ?
            "#,
        )
        .bind(matricula)
        .fetch_optional(&self.pool)
        .await?;

        Ok(user)
    }

    async fn find_by_id(&self, id: i64) -> StorageResult<Option<User>> {
        let user = sqlx::query_as::<_, User>(
            r#"
            SELECT id, pis, nome, matricula, cpf,
                   validade_inicio, validade_fim, ativo,
                   allow_card, allow_bio, allow_keypad, codigo,
                   created_at, updated_at
            FROM users
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(user)
    }

    async fn find_by_code(&self, code: &str) -> StorageResult<Option<User>> {
        let user = sqlx::query_as::<_, User>(
            r#"
            SELECT id, pis, nome, matricula, cpf,
                   validade_inicio, validade_fim, ativo,
                   allow_card, allow_bio, allow_keypad, codigo,
                   created_at, updated_at
            FROM users
            WHERE codigo = ? AND allow_keypad = 1
            "#,
        )
        .bind(code)
        .fetch_optional(&self.pool)
        .await?;

        Ok(user)
    }

    async fn find_all_active(&self) -> StorageResult<Vec<User>> {
        let users = sqlx::query_as::<_, User>(
            r#"
            SELECT id, pis, nome, matricula, cpf,
                   validade_inicio, validade_fim, ativo,
                   allow_card, allow_bio, allow_keypad, codigo,
                   created_at, updated_at
            FROM users
            WHERE ativo = 1
            ORDER BY nome
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(users)
    }

    async fn create(&self, user: &User) -> StorageResult<i64> {
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
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    async fn update(&self, user: &User) -> StorageResult<()> {
        let result = sqlx::query(
            r#"
            UPDATE users
            SET pis = ?, nome = ?, matricula = ?, cpf = ?,
                validade_inicio = ?, validade_fim = ?, ativo = ?,
                allow_card = ?, allow_bio = ?, allow_keypad = ?,
                codigo = ?, updated_at = datetime('now')
            WHERE id = ?
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
        .bind(user.id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(StorageError::NotFound {
                entity_type: "User".to_string(),
                field: "id".to_string(),
                value: user.id.to_string(),
            });
        }

        Ok(())
    }

    async fn delete(&self, id: i64) -> StorageResult<()> {
        let result = sqlx::query("DELETE FROM users WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(StorageError::NotFound {
                entity_type: "User".to_string(),
                field: "id".to_string(),
                value: id.to_string(),
            });
        }

        Ok(())
    }

    async fn exists_by_matricula(&self, matricula: &str) -> StorageResult<bool> {
        let result: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE matricula = ?")
            .bind(matricula)
            .fetch_one(&self.pool)
            .await?;

        Ok(result.0 > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::Database;
    use chrono::{Duration, Utc};

    async fn setup_test_db() -> Database {
        Database::in_memory().await.unwrap()
    }

    fn create_test_user(matricula: &str) -> User {
        User {
            id: 0,
            pis: Some("12345678901".to_string()),
            nome: "Test User".to_string(),
            matricula: matricula.to_string(),
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

    #[tokio::test]
    async fn test_create_and_find_user() {
        let db = setup_test_db().await;
        let repo = SqliteUserRepository::new(db.pool().clone());

        let user = create_test_user("EMP001");
        let id = repo.create(&user).await.unwrap();
        assert!(id > 0);

        let found = repo.find_by_matricula("EMP001").await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().nome, "Test User");
    }

    #[tokio::test]
    async fn test_find_by_id() {
        let db = setup_test_db().await;
        let repo = SqliteUserRepository::new(db.pool().clone());

        let user = create_test_user("EMP002");
        let id = repo.create(&user).await.unwrap();

        let found = repo.find_by_id(id).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().matricula, "EMP002");
    }

    #[tokio::test]
    async fn test_find_by_code() {
        let db = setup_test_db().await;
        let repo = SqliteUserRepository::new(db.pool().clone());

        // Create user with unique code to avoid collision with seed data
        let mut user = create_test_user("EMP003");
        user.codigo = Some("TEST1234".to_string()); // Unique code not in seed data
        repo.create(&user).await.unwrap();

        let found = repo.find_by_code("TEST1234").await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().matricula, "EMP003");
    }

    #[tokio::test]
    async fn test_find_all_active() {
        let db = setup_test_db().await;
        let repo = SqliteUserRepository::new(db.pool().clone());

        repo.create(&create_test_user("EMP004")).await.unwrap();
        repo.create(&create_test_user("EMP005")).await.unwrap();

        let users = repo.find_all_active().await.unwrap();
        assert!(users.len() >= 2);
    }

    #[tokio::test]
    async fn test_update_user() {
        let db = setup_test_db().await;
        let repo = SqliteUserRepository::new(db.pool().clone());

        let user = create_test_user("EMP006");
        let id = repo.create(&user).await.unwrap();

        let mut updated_user = repo.find_by_id(id).await.unwrap().unwrap();
        updated_user.nome = "Updated Name".to_string();

        repo.update(&updated_user).await.unwrap();

        let found = repo.find_by_id(id).await.unwrap().unwrap();
        assert_eq!(found.nome, "Updated Name");
    }

    #[tokio::test]
    async fn test_delete_user() {
        let db = setup_test_db().await;
        let repo = SqliteUserRepository::new(db.pool().clone());

        let user = create_test_user("EMP007");
        let id = repo.create(&user).await.unwrap();

        repo.delete(id).await.unwrap();

        let found = repo.find_by_id(id).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_exists_by_matricula() {
        let db = setup_test_db().await;
        let repo = SqliteUserRepository::new(db.pool().clone());

        let user = create_test_user("EMP008");
        repo.create(&user).await.unwrap();

        assert!(repo.exists_by_matricula("EMP008").await.unwrap());
        assert!(!repo.exists_by_matricula("EMP999").await.unwrap());
    }
}
