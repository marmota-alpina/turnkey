use crate::error::{StorageError, StorageResult};
use sqlx::ConnectOptions;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;

/// Database connection configuration for SQLite
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    /// Path to the SQLite database file
    pub database_path: String,

    /// Maximum number of connections in the pool
    pub max_connections: u32,

    /// Minimum number of idle connections to maintain
    pub min_connections: u32,

    /// Maximum lifetime of a connection before it's closed
    pub max_lifetime: Duration,

    /// Timeout for acquiring a connection from the pool
    pub acquire_timeout: Duration,

    /// Whether to create the database file if it doesn't exist
    pub create_if_missing: bool,

    /// Whether to run migrations on connection
    pub auto_migrate: bool,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            database_path: "turnkey.db".to_string(),
            max_connections: 10,
            min_connections: 2,
            max_lifetime: Duration::from_secs(1800), // 30 minutes
            acquire_timeout: Duration::from_secs(30),
            create_if_missing: true,
            auto_migrate: true,
        }
    }
}

impl DatabaseConfig {
    /// Create a new database configuration with the given path
    pub fn new(database_path: impl Into<String>) -> Self {
        Self {
            database_path: database_path.into(),
            ..Default::default()
        }
    }

    /// Set the maximum number of connections in the pool
    pub fn max_connections(mut self, max: u32) -> Self {
        self.max_connections = max;
        self
    }

    /// Set the minimum number of idle connections
    pub fn min_connections(mut self, min: u32) -> Self {
        self.min_connections = min;
        self
    }

    /// Set whether to create the database if it doesn't exist
    pub fn create_if_missing(mut self, create: bool) -> Self {
        self.create_if_missing = create;
        self
    }

    /// Set whether to run migrations automatically
    pub fn auto_migrate(mut self, migrate: bool) -> Self {
        self.auto_migrate = migrate;
        self
    }
}

/// Database connection pool wrapper
#[derive(Debug, Clone)]
pub struct Database {
    pool: SqlitePool,
}

impl Database {
    /// Create a new database connection pool with the given configuration
    ///
    /// # Example
    ///
    /// ```no_run
    /// use turnkey_storage::connection::{Database, DatabaseConfig};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = DatabaseConfig::new("turnkey.db")
    ///     .max_connections(10)
    ///     .auto_migrate(true);
    ///
    /// let db = Database::new(config).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new(config: DatabaseConfig) -> StorageResult<Self> {
        // Create parent directory if it doesn't exist
        if let Some(parent) = Path::new(&config.database_path).parent()
            && !parent.exists()
        {
            std::fs::create_dir_all(parent).map_err(|e| {
                StorageError::Configuration(format!("Failed to create database directory: {}", e))
            })?;
        }

        // Configure SQLite connection options
        let options = SqliteConnectOptions::from_str(&format!("sqlite://{}", config.database_path))
            .map_err(|e| StorageError::Configuration(format!("Invalid database path: {}", e)))?
            .create_if_missing(config.create_if_missing)
            .foreign_keys(true) // Enable foreign key constraints
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal) // Use WAL for better concurrency
            .synchronous(sqlx::sqlite::SqliteSynchronous::Normal) // Balance performance and safety
            .busy_timeout(Duration::from_secs(10)) // Wait up to 10s for locks
            .disable_statement_logging(); // Disable logging for connection attempts

        // Create connection pool
        let pool = SqlitePoolOptions::new()
            .max_connections(config.max_connections)
            .min_connections(config.min_connections)
            .max_lifetime(Some(config.max_lifetime))
            .acquire_timeout(config.acquire_timeout)
            .connect_with(options)
            .await?;

        let db = Self { pool };

        // Run migrations if enabled
        if config.auto_migrate {
            db.migrate().await?;
        }

        Ok(db)
    }

    /// Create an in-memory database (primarily for testing)
    ///
    /// # Example
    ///
    /// ```no_run
    /// use turnkey_storage::connection::Database;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let db = Database::in_memory().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn in_memory() -> StorageResult<Self> {
        let options = SqliteConnectOptions::from_str("sqlite::memory:")?
            .foreign_keys(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);

        let pool = SqlitePoolOptions::new()
            .max_connections(1) // In-memory databases should use single connection
            .connect_with(options)
            .await?;

        let db = Self { pool };
        db.migrate().await?;

        Ok(db)
    }

    /// Run database migrations
    ///
    /// Executes all SQL migration files in the `migrations/` directory.
    ///
    /// # Security
    ///
    /// The migration path is resolved at compile time by the `sqlx::migrate!` macro
    /// using a relative path from the crate root. The macro validates and embeds
    /// migration files at compile time, preventing runtime path manipulation and
    /// directory traversal attacks. The path is fixed in the binary.
    ///
    /// # Errors
    ///
    /// Returns error if migrations fail to execute or if the migration directory
    /// structure is invalid.
    pub async fn migrate(&self) -> StorageResult<()> {
        sqlx::migrate!("../../migrations").run(&self.pool).await?;
        Ok(())
    }

    /// Get a reference to the underlying connection pool
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// Close the database connection pool
    ///
    /// This will wait for all active connections to be returned to the pool
    /// before closing them.
    pub async fn close(&self) {
        self.pool.close().await;
    }

    /// Check if the database connection is healthy
    ///
    /// Executes a simple query to verify the connection is working.
    pub async fn health_check(&self) -> StorageResult<()> {
        sqlx::query("SELECT 1").execute(&self.pool).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Unit test for DatabaseConfig builder pattern
    #[test]
    fn test_database_config_builder() {
        let config = DatabaseConfig::new("test.db")
            .max_connections(5)
            .min_connections(1)
            .create_if_missing(false)
            .auto_migrate(false);

        assert_eq!(config.database_path, "test.db");
        assert_eq!(config.max_connections, 5);
        assert_eq!(config.min_connections, 1);
        assert!(!config.create_if_missing);
        assert!(!config.auto_migrate);
    }

    /// Unit test for DatabaseConfig default values
    #[test]
    fn test_database_config_defaults() {
        let config = DatabaseConfig::default();

        assert_eq!(config.database_path, "turnkey.db");
        assert_eq!(config.max_connections, 10);
        assert_eq!(config.min_connections, 2);
        assert_eq!(config.max_lifetime, Duration::from_secs(1800));
        assert_eq!(config.acquire_timeout, Duration::from_secs(30));
        assert!(config.create_if_missing);
        assert!(config.auto_migrate);
    }

    /// Unit test for DatabaseConfig fluent API
    #[test]
    fn test_database_config_fluent_api() {
        let config = DatabaseConfig::new("custom.db")
            .max_connections(20)
            .min_connections(5)
            .create_if_missing(true)
            .auto_migrate(true);

        assert_eq!(config.database_path, "custom.db");
        assert_eq!(config.max_connections, 20);
        assert_eq!(config.min_connections, 5);
        assert!(config.create_if_missing);
        assert!(config.auto_migrate);
    }
}
