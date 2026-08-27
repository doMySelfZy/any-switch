use crate::error::{AppError, AppResult};
use crate::paths::{db_path, ensure_app_dirs};
use rusqlite::Connection;
use std::sync::Mutex;

pub struct Db {
    pub conn: Mutex<Connection>,
}

impl Db {
    pub fn open() -> AppResult<Self> {
        ensure_app_dirs()?;
        let path = db_path()?;
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;")?;
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
    conn.execute_batch(
        r#"
CREATE TABLE IF NOT EXISTS settings (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  json TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS sites (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  base_url TEXT NOT NULL,
  api_key_encrypted TEXT NOT NULL,
  key_prefix TEXT NOT NULL,
  protocol TEXT NOT NULL DEFAULT 'openai_compatible',
  claude_auth_key_style TEXT NOT NULL DEFAULT 'anthropic_auth_token',
  notes TEXT,
  enabled INTEGER NOT NULL DEFAULT 1,
  sort_order INTEGER NOT NULL DEFAULT 0,
  selected_model_id TEXT,
  last_model_fetch_at INTEGER,
  last_model_fetch_latency_ms INTEGER,
  last_model_fetch_error TEXT,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS site_models (
  id TEXT PRIMARY KEY,
  site_id TEXT NOT NULL REFERENCES sites(id) ON DELETE CASCADE,
  model_id TEXT NOT NULL,
  display_name TEXT NOT NULL,
  owned_by TEXT,
  raw_json TEXT,
  is_manual INTEGER NOT NULL DEFAULT 0,
  UNIQUE(site_id, model_id)
);

CREATE TABLE IF NOT EXISTS target_bindings (
  target TEXT PRIMARY KEY,
  site_id TEXT,
  site_name_snapshot TEXT NOT NULL,
  model_id TEXT NOT NULL,
  provider_id TEXT,
  key_fingerprint TEXT NOT NULL,
  managed_paths_json TEXT NOT NULL,
  managed_env_keys_json TEXT NOT NULL,
  expected_fields_json TEXT NOT NULL,
  orphan INTEGER NOT NULL DEFAULT 0,
  apply_record_id TEXT,
  applied_at INTEGER NOT NULL,
  FOREIGN KEY (site_id) REFERENCES sites(id) ON DELETE SET NULL
);

CREATE TABLE IF NOT EXISTS apply_records (
  id TEXT PRIMARY KEY,
  site_id TEXT,
  site_name_snapshot TEXT NOT NULL,
  target TEXT NOT NULL,
  model_id TEXT NOT NULL,
  provider_id TEXT,
  status TEXT NOT NULL,
  backup_dir TEXT,
  touched_keys_json TEXT NOT NULL,
  config_snapshot_hash TEXT,
  error TEXT,
  applied_at INTEGER NOT NULL,
  FOREIGN KEY (site_id) REFERENCES sites(id) ON DELETE SET NULL
);

CREATE TABLE IF NOT EXISTS site_model_exclusions (
  site_id TEXT NOT NULL REFERENCES sites(id) ON DELETE CASCADE,
  model_id TEXT NOT NULL,
  PRIMARY KEY (site_id, model_id)
);

CREATE TABLE IF NOT EXISTS webdav_config (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  base_url TEXT NOT NULL,
  username TEXT NOT NULL,
  password_encrypted TEXT NOT NULL,
  remote_path TEXT NOT NULL DEFAULT 'xiaobai-switch',
  accept_invalid_certs INTEGER NOT NULL DEFAULT 0,
  auto_sync_enabled INTEGER NOT NULL DEFAULT 0,
  sync_interval_minutes INTEGER NOT NULL DEFAULT 60,
  max_remote_backups INTEGER NOT NULL DEFAULT 10,
  updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS webdav_sync_state (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  last_attempt_at INTEGER,
  last_success_at INTEGER,
  status TEXT NOT NULL DEFAULT 'never',
  error TEXT
);

CREATE INDEX IF NOT EXISTS idx_apply_records_target_time ON apply_records(target, applied_at DESC);
CREATE INDEX IF NOT EXISTS idx_target_bindings_orphan ON target_bindings(orphan);
"#,
    )?;
    ensure_column(
        conn,
        "site_models",
        "is_manual",
        "ALTER TABLE site_models ADD COLUMN is_manual INTEGER NOT NULL DEFAULT 0",
    )?;
    ensure_column(
        conn,
        "sites",
        "base_urls_json",
        "ALTER TABLE sites ADD COLUMN base_urls_json TEXT",
    )?;
    ensure_column(
        conn,
        "sites",
        "capabilities_json",
        "ALTER TABLE sites ADD COLUMN capabilities_json TEXT",
    )?;
    backfill_base_urls(conn)
}

fn backfill_base_urls(conn: &Connection) -> AppResult<()> {
    let mut stmt = conn.prepare("SELECT id, base_url, base_urls_json FROM sites")?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<String>>(2)?,
        ))
    })?;
    let mut updates = Vec::new();
    for r in rows {
        let (id, base_url, json) = r?;
        let needs = match json.as_deref() {
            None | Some("") => true,
            Some(s) => serde_json::from_str::<Vec<String>>(s)
                .ok()
                .filter(|v| !v.is_empty())
                .is_none(),
        };
        if needs {
            let encoded = serde_json::to_string(&vec![base_url])?;
            updates.push((id, encoded));
        }
    }
    drop(stmt);
    for (id, json) in updates {
        conn.execute(
            "UPDATE sites SET base_urls_json = ?2 WHERE id = ?1",
            rusqlite::params![id, json],
        )?;
    }
    Ok(())
}

fn ensure_column(conn: &Connection, table: &str, column: &str, alter_sql: &str) -> AppResult<()> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let exists = stmt
        .query_map([], |row| row.get::<_, String>(1))?
        .filter_map(|r| r.ok())
        .any(|name| name == column);
    if !exists {
        conn.execute_batch(alter_sql)?;
    }
    Ok(())
}
