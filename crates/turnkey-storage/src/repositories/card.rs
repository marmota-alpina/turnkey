#![allow(async_fn_in_trait)]

use crate::error::{StorageError, StorageResult};
use crate::models::Card;
use sqlx::SqlitePool;

/// Repository trait for Card entity operations
///
/// This trait defines the contract for card data access, enabling
/// testability through mock implementations and separation of concerns.
///
/// # Implementation Note
///
/// This trait uses native async trait methods (Edition 2024 feature),
/// eliminating the need for the async-trait crate while maintaining
/// full async/await support in trait methods.
pub trait CardRepository: Send + Sync {
    /// Find a card by its number
    async fn find_by_number(&self, numero_cartao: &str) -> StorageResult<Option<Card>>;

    /// Find all cards for a specific user (by matricula)
    async fn find_by_matricula(&self, matricula: &str) -> StorageResult<Vec<Card>>;

    /// Find all cards for a specific user (by user_id)
    async fn find_by_user_id(&self, user_id: i64) -> StorageResult<Vec<Card>>;

    /// Get all active cards
    async fn find_all_active(&self) -> StorageResult<Vec<Card>>;

    /// Create a new card
    async fn create(&self, card: &Card) -> StorageResult<i64>;

    /// Update an existing card
    async fn update(&self, card: &Card) -> StorageResult<()>;

    /// Delete a card by ID
    async fn delete(&self, id: i64) -> StorageResult<()>;

    /// Check if a card number already exists
    async fn exists_by_number(&self, numero_cartao: &str) -> StorageResult<bool>;
}

/// SQLite implementation of CardRepository
pub struct SqliteCardRepository {
    pool: SqlitePool,
}

impl SqliteCardRepository {
    /// Create a new SQLite card repository
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

impl CardRepository for SqliteCardRepository {
    async fn find_by_number(&self, numero_cartao: &str) -> StorageResult<Option<Card>> {
        let card = sqlx::query_as::<_, Card>(
            r#"
            SELECT id, numero_cartao, matricula, user_id,
                   validade_inicio, validade_fim, ativo,
                   created_at, updated_at
            FROM cards
            WHERE numero_cartao = ?
            "#,
        )
        .bind(numero_cartao)
        .fetch_optional(&self.pool)
        .await?;

        Ok(card)
    }

    async fn find_by_matricula(&self, matricula: &str) -> StorageResult<Vec<Card>> {
        let cards = sqlx::query_as::<_, Card>(
            r#"
            SELECT id, numero_cartao, matricula, user_id,
                   validade_inicio, validade_fim, ativo,
                   created_at, updated_at
            FROM cards
            WHERE matricula = ?
            ORDER BY created_at DESC
            "#,
        )
        .bind(matricula)
        .fetch_all(&self.pool)
        .await?;

        Ok(cards)
    }

    async fn find_by_user_id(&self, user_id: i64) -> StorageResult<Vec<Card>> {
        let cards = sqlx::query_as::<_, Card>(
            r#"
            SELECT id, numero_cartao, matricula, user_id,
                   validade_inicio, validade_fim, ativo,
                   created_at, updated_at
            FROM cards
            WHERE user_id = ?
            ORDER BY created_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(cards)
    }

    async fn find_all_active(&self) -> StorageResult<Vec<Card>> {
        let cards = sqlx::query_as::<_, Card>(
            r#"
            SELECT id, numero_cartao, matricula, user_id,
                   validade_inicio, validade_fim, ativo,
                   created_at, updated_at
            FROM cards
            WHERE ativo = 1
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(cards)
    }

    async fn create(&self, card: &Card) -> StorageResult<i64> {
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
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    async fn update(&self, card: &Card) -> StorageResult<()> {
        let result = sqlx::query(
            r#"
            UPDATE cards
            SET numero_cartao = ?, matricula = ?, user_id = ?,
                validade_inicio = ?, validade_fim = ?, ativo = ?,
                updated_at = datetime('now')
            WHERE id = ?
            "#,
        )
        .bind(&card.numero_cartao)
        .bind(&card.matricula)
        .bind(card.user_id)
        .bind(card.validade_inicio)
        .bind(card.validade_fim)
        .bind(card.ativo)
        .bind(card.id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(StorageError::NotFound {
                entity_type: "Card".to_string(),
                field: "id".to_string(),
                value: card.id.to_string(),
            });
        }

        Ok(())
    }

    async fn delete(&self, id: i64) -> StorageResult<()> {
        let result = sqlx::query("DELETE FROM cards WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(StorageError::NotFound {
                entity_type: "Card".to_string(),
                field: "id".to_string(),
                value: id.to_string(),
            });
        }

        Ok(())
    }

    async fn exists_by_number(&self, numero_cartao: &str) -> StorageResult<bool> {
        let result: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM cards WHERE numero_cartao = ?")
            .bind(numero_cartao)
            .fetch_one(&self.pool)
            .await?;

        Ok(result.0 > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::Database;
    use crate::models::User;
    use crate::repositories::user::{SqliteUserRepository, UserRepository};
    use chrono::{Duration, Utc};

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

    fn create_test_card(numero: &str, matricula: &str, user_id: i64) -> Card {
        Card {
            id: 0,
            numero_cartao: numero.to_string(),
            matricula: matricula.to_string(),
            user_id,
            validade_inicio: Some(Utc::now() - Duration::days(1)),
            validade_fim: Some(Utc::now() + Duration::days(30)),
            ativo: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_create_and_find_card() {
        let db = setup_test_db().await;
        let user_id = create_test_user(&db, "EMP001").await;

        let repo = SqliteCardRepository::new(db.pool().clone());
        let card = create_test_card("1234567890", "EMP001", user_id);

        let id = repo.create(&card).await.unwrap();
        assert!(id > 0);

        let found = repo.find_by_number("1234567890").await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().matricula, "EMP001");
    }

    #[tokio::test]
    async fn test_find_by_matricula() {
        let db = setup_test_db().await;
        let user_id = create_test_user(&db, "EMP002").await;

        let repo = SqliteCardRepository::new(db.pool().clone());
        repo.create(&create_test_card("1111111111", "EMP002", user_id))
            .await
            .unwrap();
        repo.create(&create_test_card("2222222222", "EMP002", user_id))
            .await
            .unwrap();

        let cards = repo.find_by_matricula("EMP002").await.unwrap();
        assert_eq!(cards.len(), 2);
    }

    #[tokio::test]
    async fn test_find_by_user_id() {
        let db = setup_test_db().await;
        let user_id = create_test_user(&db, "EMP003").await;

        let repo = SqliteCardRepository::new(db.pool().clone());
        repo.create(&create_test_card("3333333333", "EMP003", user_id))
            .await
            .unwrap();

        let cards = repo.find_by_user_id(user_id).await.unwrap();
        assert_eq!(cards.len(), 1);
    }

    #[tokio::test]
    async fn test_find_all_active() {
        let db = setup_test_db().await;
        let user_id = create_test_user(&db, "EMP004").await;

        let repo = SqliteCardRepository::new(db.pool().clone());
        repo.create(&create_test_card("4444444444", "EMP004", user_id))
            .await
            .unwrap();

        let cards = repo.find_all_active().await.unwrap();
        assert!(!cards.is_empty());
    }

    #[tokio::test]
    async fn test_update_card() {
        let db = setup_test_db().await;
        let user_id = create_test_user(&db, "EMP005").await;

        let repo = SqliteCardRepository::new(db.pool().clone());
        let card = create_test_card("5555555555", "EMP005", user_id);
        let _id = repo.create(&card).await.unwrap();

        let mut updated_card = repo.find_by_number("5555555555").await.unwrap().unwrap();
        updated_card.ativo = false;

        repo.update(&updated_card).await.unwrap();

        let found = repo.find_by_number("5555555555").await.unwrap().unwrap();
        assert!(!found.ativo);
    }

    #[tokio::test]
    async fn test_delete_card() {
        let db = setup_test_db().await;
        let user_id = create_test_user(&db, "EMP006").await;

        let repo = SqliteCardRepository::new(db.pool().clone());
        let card = create_test_card("6666666666", "EMP006", user_id);
        let id = repo.create(&card).await.unwrap();

        repo.delete(id).await.unwrap();

        let found = repo.find_by_number("6666666666").await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_exists_by_number() {
        let db = setup_test_db().await;
        let user_id = create_test_user(&db, "EMP007").await;

        let repo = SqliteCardRepository::new(db.pool().clone());
        let card = create_test_card("7777777777", "EMP007", user_id);
        repo.create(&card).await.unwrap();

        assert!(repo.exists_by_number("7777777777").await.unwrap());
        assert!(!repo.exists_by_number("9999999999").await.unwrap());
    }
}
