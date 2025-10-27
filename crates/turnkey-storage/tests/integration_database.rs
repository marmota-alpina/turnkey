//! Integration tests for database connection and pooling
//!
//! These tests require SQLite in-memory database and validate
//! connection pooling, transactions, and concurrent access patterns.
//!
//! Run with: cargo test --package turnkey-storage --test integration_database

use std::sync::Arc;
use tokio::sync::Barrier;
use turnkey_storage::connection::Database;

#[tokio::test]
async fn test_in_memory_database() {
    let db = Database::in_memory().await.unwrap();
    db.health_check().await.unwrap();
    db.close().await;
}

#[tokio::test]
async fn test_concurrent_access_validation() {
    let db = Database::in_memory().await.unwrap();

    const NUM_CONCURRENT_TASKS: usize = 10;
    let barrier = Arc::new(Barrier::new(NUM_CONCURRENT_TASKS));

    let mut handles = vec![];

    for i in 0..NUM_CONCURRENT_TASKS {
        let db_clone = db.clone();
        let barrier_clone = barrier.clone();

        let handle = tokio::spawn(async move {
            barrier_clone.wait().await;

            let result: Result<(i64,), sqlx::Error> = sqlx::query_as("SELECT ?")
                .bind(i as i64)
                .fetch_one(db_clone.pool())
                .await;

            result.unwrap()
        });

        handles.push(handle);
    }

    let results: Vec<_> = futures::future::join_all(handles).await;

    assert_eq!(results.len(), NUM_CONCURRENT_TASKS);
    for (i, result) in results.into_iter().enumerate() {
        let value = result.unwrap();
        assert_eq!(value.0, i as i64);
    }

    db.close().await;
}

#[tokio::test]
async fn test_migration_idempotency() {
    let db = Database::in_memory().await.unwrap();

    db.migrate().await.unwrap();

    db.migrate().await.unwrap();

    let result: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='users'")
            .fetch_one(db.pool())
            .await
            .unwrap();

    assert_eq!(result.0, 1);

    db.close().await;
}

#[tokio::test]
async fn test_database_health_check() {
    let db = Database::in_memory().await.unwrap();

    assert!(db.health_check().await.is_ok());

    db.health_check().await.unwrap();
    db.health_check().await.unwrap();

    db.close().await;
}

#[tokio::test]
async fn test_sequential_transactions() {
    let db = Database::in_memory().await.unwrap();

    let mut tx1 = db.pool().begin().await.unwrap();
    sqlx::query(
        "INSERT INTO users (pis, nome, matricula, cpf, ativo, allow_card, allow_bio, allow_keypad)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(None::<String>)
    .bind("User 1")
    .bind("INT_SEQ_TX_001")
    .bind(None::<String>)
    .bind(true)
    .bind(true)
    .bind(false)
    .bind(false)
    .execute(&mut *tx1)
    .await
    .unwrap();
    tx1.commit().await.unwrap();

    let mut tx2 = db.pool().begin().await.unwrap();
    sqlx::query(
        "INSERT INTO users (pis, nome, matricula, cpf, ativo, allow_card, allow_bio, allow_keypad)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(None::<String>)
    .bind("User 2")
    .bind("INT_SEQ_TX_002")
    .bind(None::<String>)
    .bind(true)
    .bind(true)
    .bind(false)
    .bind(false)
    .execute(&mut *tx2)
    .await
    .unwrap();
    tx2.commit().await.unwrap();

    let count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM users WHERE matricula LIKE 'INT_SEQ_TX_%'")
            .fetch_one(db.pool())
            .await
            .unwrap();

    assert_eq!(count.0, 2);

    db.close().await;
}
