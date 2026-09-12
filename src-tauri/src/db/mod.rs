use crate::crypto::Crypto;
use crate::error::{AppError, AppResult};
use crate::paths::{db_path, ensure_app_dirs};
use rusqlite::Connection;
use std::sync::Mutex;

pub mod migrate;

pub const SCHEMA_VERSION: i32 = 1;

pub struct Db {
    pub conn: Mutex<Connection>,
}

impl Db {
    pub fn open() -> AppResult<Self> {
        ensure_app_dirs()?;
        let path = db_path()?;
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;")?;
        crate::sync::install_db_hook(&conn);
        let db = Self {
            conn: Mutex::new(conn),
        };
        db.migrate()?;
        Ok(db)
    }

    pub fn migrate(&self) -> AppResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::new("internal", e.to_string()))?;
        apply_schema(&conn)
    }

    pub fn with_conn<T>(&self, f: impl FnOnce(&Connection) -> AppResult<T>) -> AppResult<T> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::new("internal", e.to_string()))?;
        f(&conn)
    }
}

pub fn apply_schema(conn: &Connection) -> AppResult<()> {
    apply_schema_with_crypto(conn, None, migrate::BackupMode::Auto)
}

pub fn apply_schema_with_crypto(
    conn: &Connection,
    crypto: Option<&Crypto>,
    backup: migrate::BackupMode,
) -> AppResult<()> {
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    migrate::apply_schema(conn, crypto, backup)
}

pub fn verify_encrypted_payloads(conn: &Connection, crypto: &Crypto) -> AppResult<()> {
    migrate::verify_encrypted_payloads(conn, crypto)
}

pub(crate) fn table_exists(conn: &Connection, name: &str) -> AppResult<bool> {
    let n: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
        rusqlite::params![name],
        |row| row.get(0),
    )?;
    Ok(n > 0)
}

pub(crate) fn column_exists(conn: &Connection, table: &str, column: &str) -> AppResult<bool> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let exists = stmt
        .query_map([], |row| row.get::<_, String>(1))?
        .filter_map(|r| r.ok())
        .any(|name| name == column);
    Ok(exists)
}

pub(crate) fn user_version(conn: &Connection) -> AppResult<i32> {
    Ok(conn.query_row("PRAGMA user_version", [], |row| row.get(0))?)
}

pub(crate) fn set_user_version(conn: &Connection, version: i32) -> AppResult<()> {
    conn.execute_batch(&format!("PRAGMA user_version = {version}"))?;
    Ok(())
}

pub(crate) fn ensure_column(
    conn: &Connection,
    table: &str,
    column: &str,
    alter_sql: &str,
) -> AppResult<()> {
    if !column_exists(conn, table, column)? {
        conn.execute_batch(alter_sql)?;
    }
    Ok(())
}
