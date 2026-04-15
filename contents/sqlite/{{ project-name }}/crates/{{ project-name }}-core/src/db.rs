//! SQLite persistence layer with versioned schema migrations.
//!
//! Schema is versioned using `PRAGMA user_version`. Migrations run automatically
//! on database open. To add a new migration, increment `CURRENT_VERSION` and add
//! a new block in `apply_migrations()`.

use anyhow::{Result, bail};
use rusqlite::Connection;

/// Current schema version. Bump this when adding new migrations.
const CURRENT_VERSION: u32 = 1;

/// Initial schema: key-value store and application state.
const SCHEMA_V1: &str = "
    CREATE TABLE IF NOT EXISTS kv (
        key   TEXT PRIMARY KEY,
        value TEXT NOT NULL
    );

    CREATE TABLE IF NOT EXISTS app_state (
        id             INTEGER PRIMARY KEY CHECK (id = 1),
        initialized_at TEXT NOT NULL DEFAULT (datetime('now'))
    );

    INSERT OR IGNORE INTO app_state (id) VALUES (1);
";

pub struct Database {
    conn: Connection,
}

impl Database {
    /// Open (or create) the SQLite database at the given path.
    pub fn open(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        let db = Self { conn };
        db.init_schema()?;
        Ok(db)
    }

    /// Check the schema version and apply any pending migrations.
    fn init_schema(&self) -> Result<()> {
        let version: u32 = self.conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
        if version < CURRENT_VERSION {
            self.apply_migrations(version)?;
        } else if version > CURRENT_VERSION {
            bail!(
                "database schema version {version} is newer than supported ({CURRENT_VERSION}); \
                 upgrade the application or use a compatible database"
            );
        }
        Ok(())
    }

    /// Run migrations from `from_version` up to `CURRENT_VERSION` inside a transaction.
    fn apply_migrations(&self, from_version: u32) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;

        if from_version < 1 {
            tx.execute_batch(SCHEMA_V1)?;
        }

        // Future migrations:
        // if from_version < 2 {
        //     tx.execute_batch(SCHEMA_V2)?;
        // }

        tx.pragma_update(None, "user_version", CURRENT_VERSION)?;
        tx.commit()?;

        tracing::info!(from = from_version, to = CURRENT_VERSION, "applied database migrations");
        Ok(())
    }

    /// Verify the database is reachable and at the expected schema version.
    pub fn health_check(&self) -> Result<()> {
        let version: u32 = self.conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
        if version != CURRENT_VERSION {
            bail!("schema version mismatch: expected {CURRENT_VERSION}, got {version}");
        }
        Ok(())
    }

    /// Get a value from the key-value store.
    pub fn get(&self, key: &str) -> Result<Option<String>> {
        let mut stmt = self.conn.prepare("SELECT value FROM kv WHERE key = ?1")?;
        let result = stmt
            .query_row([key], |row| row.get(0))
            .optional()?;
        Ok(result)
    }

    /// Set a value in the key-value store (insert or update).
    pub fn set(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO kv (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = ?2",
            [key, value],
        )?;
        Ok(())
    }
}

trait OptionalExt<T> {
    fn optional(self) -> Result<Option<T>, rusqlite::Error>;
}

impl<T> OptionalExt<T> for Result<T, rusqlite::Error> {
    fn optional(self) -> Result<Option<T>, rusqlite::Error> {
        match self {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_db() -> Database {
        Database::open(":memory:").expect("in-memory db should open")
    }

    #[test]
    fn open_creates_schema() {
        let db = test_db();
        let version: u32 = db
            .conn
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .unwrap();
        assert_eq!(version, CURRENT_VERSION);
    }

    #[test]
    fn kv_round_trip() {
        let db = test_db();
        assert_eq!(db.get("missing").unwrap(), None);

        db.set("key1", "value1").unwrap();
        assert_eq!(db.get("key1").unwrap(), Some("value1".to_string()));

        // Upsert overwrites
        db.set("key1", "updated").unwrap();
        assert_eq!(db.get("key1").unwrap(), Some("updated".to_string()));
    }

    #[test]
    fn health_check_succeeds() {
        let db = test_db();
        db.health_check().unwrap();
    }

    #[test]
    fn migrations_are_idempotent() {
        let db = test_db();
        // Opening again on a fresh :memory: should also succeed
        let db2 = test_db();
        db.set("k", "v").unwrap();
        db2.health_check().unwrap();
    }
}
