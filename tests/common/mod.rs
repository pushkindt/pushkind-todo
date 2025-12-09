//! Helpers for integration tests.

use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use pushkind_common::db::{DbPool, establish_connection_pool};
use tempfile::NamedTempFile;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!(); // assumes migrations/ exists

/// Temporary database used in integration tests.
pub struct TestDb {
    _tempfile: NamedTempFile,
    pool: DbPool,
}

impl TestDb {
    pub fn new() -> Self {
        let tempfile = NamedTempFile::new().expect("Failed to create temp file");
        let pool = establish_connection_pool(tempfile.path().to_str().unwrap())
            .expect("Failed to establish SQLite connection.");
        let mut conn = pool
            .get()
            .expect("Failed to get SQLite connection from pool.");
        conn.run_pending_migrations(MIGRATIONS)
            .expect("Migrations failed");
        TestDb {
            _tempfile: tempfile,
            pool,
        }
    }

    pub fn pool(&self) -> DbPool {
        self.pool.clone()
    }
}
