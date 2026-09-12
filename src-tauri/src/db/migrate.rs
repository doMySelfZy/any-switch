use super::{
    column_exists, ensure_column, set_user_version, table_exists, user_version, SCHEMA_VERSION,
};
use crate::crypto::Crypto;
use crate::error::{AppError, AppResult};
use crate::paths::{app_backups_dir, db_path, master_key_path};
use rusqlite::{params, Connection};
use std::fs;
use std::path::Path;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackupMode {
    Auto,
    Skip,
    Required,
}

const V1_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS settings (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  json TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS sites (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  base_url TEXT NOT NULL,
  protocol TEXT NOT NULL DEFAULT 'openai_compatible',
  claude_auth_key_style TEXT NOT NULL DEFAULT 'anthropic_auth_token',
  notes TEXT,
  enabled INTEGER NOT NULL DEFAULT 1,
  sort_order INTEGER NOT NULL DEFAULT 0,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  base_urls_json TEXT,
  capabilities_json TEXT
);

CREATE TABLE IF NOT EXISTS site_api_keys (
  id TEXT PRIMARY KEY,
  site_id TEXT NOT NULL REFERENCES sites(id) ON DELETE CASCADE,
  label TEXT NOT NULL,
  api_key_encrypted TEXT NOT NULL,
  key_prefix TEXT NOT NULL,
  is_active INTEGER NOT NULL DEFAULT 0,
  selected_model_id TEXT,
  last_model_fetch_at INTEGER,
  last_model_fetch_latency_ms INTEGER,
  last_model_fetch_error TEXT,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_site_api_keys_one_active
  ON site_api_keys(site_id) WHERE is_active = 1;
CREATE UNIQUE INDEX IF NOT EXISTS idx_site_api_keys_label
  ON site_api_keys(site_id, lower(label));

CREATE TABLE IF NOT EXISTS site_models (
  id TEXT PRIMARY KEY,
  site_id TEXT NOT NULL REFERENCES sites(id) ON DELETE CASCADE,
  api_key_id TEXT NOT NULL REFERENCES site_api_keys(id) ON DELETE CASCADE,
  model_id TEXT NOT NULL,
  display_name TEXT NOT NULL,
  owned_by TEXT,
  raw_json TEXT,
  is_manual INTEGER NOT NULL DEFAULT 0,
  UNIQUE(api_key_id, model_id)
);

CREATE TABLE IF NOT EXISTS site_model_exclusions (
  site_id TEXT NOT NULL REFERENCES sites(id) ON DELETE CASCADE,
  api_key_id TEXT NOT NULL REFERENCES site_api_keys(id) ON DELETE CASCADE,
  model_id TEXT NOT NULL,
  PRIMARY KEY (api_key_id, model_id)
);

CREATE TABLE IF NOT EXISTS site_thinking_presets (
  site_id TEXT NOT NULL REFERENCES sites(id) ON DELETE CASCADE,
  target TEXT NOT NULL,
  json TEXT NOT NULL,
  updated_at INTEGER NOT NULL,
  PRIMARY KEY (site_id, target)
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
  site_api_key_id TEXT,
  site_api_key_label_snapshot TEXT,
  site_api_key_prefix_snapshot TEXT,
  FOREIGN KEY (site_id) REFERENCES sites(id) ON DELETE SET NULL,
  FOREIGN KEY (site_api_key_id) REFERENCES site_api_keys(id) ON DELETE SET NULL
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
  site_api_key_id TEXT,
  site_api_key_label_snapshot TEXT,
  site_api_key_prefix_snapshot TEXT,
  FOREIGN KEY (site_id) REFERENCES sites(id) ON DELETE SET NULL,
  FOREIGN KEY (site_api_key_id) REFERENCES site_api_keys(id) ON DELETE SET NULL
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

CREATE TABLE IF NOT EXISTS sync_meta (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_apply_records_target_time ON apply_records(target, applied_at DESC);
CREATE INDEX IF NOT EXISTS idx_target_bindings_orphan ON target_bindings(orphan);
CREATE INDEX IF NOT EXISTS idx_site_api_keys_site ON site_api_keys(site_id);
CREATE INDEX IF NOT EXISTS idx_site_models_key ON site_models(api_key_id);
"#;

pub fn apply_schema(
    conn: &Connection,
    crypto: Option<&Crypto>,
    backup: BackupMode,
) -> AppResult<()> {
    let version = user_version(conn)?;
    if version >= SCHEMA_VERSION {
        conn.execute_batch(V1_SCHEMA)?;
        return Ok(());
    }

    if needs_legacy_migration(conn)? {
        let owned;
        let crypto = match crypto {
            Some(c) => c,
            None => {
                owned = Crypto::ensure_can_decrypt_db(true)?;
                &owned
            }
        };
        migrate_legacy(conn, crypto, backup)?;
        return Ok(());
    }

    conn.execute_batch(V1_SCHEMA)?;
    backfill_base_urls(conn)?;
    set_user_version(conn, SCHEMA_VERSION)?;
    Ok(())
}

fn needs_legacy_migration(conn: &Connection) -> AppResult<bool> {
    if !table_exists(conn, "sites")? {
        return Ok(false);
    }
    if table_exists(conn, "site_api_keys")? {
        return Ok(false);
    }
    column_exists(conn, "sites", "api_key_encrypted")
}

fn migrate_legacy(conn: &Connection, crypto: &Crypto, backup: BackupMode) -> AppResult<()> {
    verify_encrypted_payloads(conn, crypto)?;
    maybe_backup(conn, backup)?;

    conn.execute_batch("PRAGMA foreign_keys = OFF;")?;
    conn.execute_batch("BEGIN IMMEDIATE;")?;
    let result = run_legacy_migration(conn);
    match result {
        Ok(()) => conn.execute_batch("COMMIT;")?,
        Err(_) => {
            let _ = conn.execute_batch("ROLLBACK;");
        }
    }
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    result?;

    let mut stmt = conn.prepare("PRAGMA foreign_key_check")?;
    let violations: Vec<String> = stmt
        .query_map([], |row| {
            Ok(format!(
                "{}:{}",
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    if !violations.is_empty() {
        return Err(AppError::new(
            "internal",
            format!(
                "foreign key check failed after migration: {}",
                violations.join(",")
            ),
        ));
    }
    Ok(())
}

fn run_legacy_migration(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        r#"
CREATE TABLE site_api_keys (
  id TEXT PRIMARY KEY,
  site_id TEXT NOT NULL REFERENCES sites(id) ON DELETE CASCADE,
  label TEXT NOT NULL,
  api_key_encrypted TEXT NOT NULL,
  key_prefix TEXT NOT NULL,
  is_active INTEGER NOT NULL DEFAULT 0,
  selected_model_id TEXT,
  last_model_fetch_at INTEGER,
  last_model_fetch_latency_ms INTEGER,
  last_model_fetch_error TEXT,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);
"#,
    )?;

    let mut sites = conn.prepare(
        "SELECT id, api_key_encrypted, key_prefix, selected_model_id, last_model_fetch_at, last_model_fetch_latency_ms, last_model_fetch_error, created_at, updated_at FROM sites",
    )?;
    let rows = sites.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, Option<String>>(3)?,
            row.get::<_, Option<i64>>(4)?,
            row.get::<_, Option<i64>>(5)?,
            row.get::<_, Option<String>>(6)?,
            row.get::<_, i64>(7)?,
            row.get::<_, i64>(8)?,
        ))
    })?;
    let mut site_keys = Vec::new();
    for row in rows {
        site_keys.push(row?);
    }
    drop(sites);

    let mut key_by_site = std::collections::HashMap::new();
    for (site_id, enc, prefix, selected, fetch_at, latency, fetch_err, created, updated) in
        site_keys
    {
        let key_id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO site_api_keys (id, site_id, label, api_key_encrypted, key_prefix, is_active, selected_model_id, last_model_fetch_at, last_model_fetch_latency_ms, last_model_fetch_error, created_at, updated_at)
             VALUES (?1,?2,'K 1',?3,?4,1,?5,?6,?7,?8,?9,?10)",
            params![
                key_id,
                site_id,
                enc,
                prefix,
                selected,
                fetch_at,
                latency,
                fetch_err,
                created,
                updated
            ],
        )?;
        key_by_site.insert(site_id, key_id);
    }

    conn.execute_batch(
        r#"
CREATE TABLE site_models_new (
  id TEXT PRIMARY KEY,
  site_id TEXT NOT NULL,
  api_key_id TEXT NOT NULL,
  model_id TEXT NOT NULL,
  display_name TEXT NOT NULL,
  owned_by TEXT,
  raw_json TEXT,
  is_manual INTEGER NOT NULL DEFAULT 0,
  UNIQUE(api_key_id, model_id)
);
CREATE TABLE site_model_exclusions_new (
  site_id TEXT NOT NULL,
  api_key_id TEXT NOT NULL,
  model_id TEXT NOT NULL,
  PRIMARY KEY (api_key_id, model_id)
);
"#,
    )?;

    if table_exists(conn, "site_models")? {
        let mut stmt = conn.prepare(
            "SELECT id, site_id, model_id, display_name, owned_by, raw_json, is_manual FROM site_models",
        )?;
        let models = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, i64>(6).unwrap_or(0),
            ))
        })?;
        for model in models {
            let (id, site_id, model_id, display, owned, raw, manual) = model?;
            let Some(key_id) = key_by_site.get(&site_id) else {
                continue;
            };
            conn.execute(
                "INSERT OR IGNORE INTO site_models_new (id, site_id, api_key_id, model_id, display_name, owned_by, raw_json, is_manual) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
                params![id, site_id, key_id, model_id, display, owned, raw, manual],
            )?;
        }
        drop(stmt);
        conn.execute_batch("DROP TABLE site_models;")?;
    }
    conn.execute_batch(
        r#"
CREATE TABLE site_models (
  id TEXT PRIMARY KEY,
  site_id TEXT NOT NULL REFERENCES sites(id) ON DELETE CASCADE,
  api_key_id TEXT NOT NULL REFERENCES site_api_keys(id) ON DELETE CASCADE,
  model_id TEXT NOT NULL,
  display_name TEXT NOT NULL,
  owned_by TEXT,
  raw_json TEXT,
  is_manual INTEGER NOT NULL DEFAULT 0,
  UNIQUE(api_key_id, model_id)
);
INSERT INTO site_models SELECT * FROM site_models_new;
DROP TABLE site_models_new;
"#,
    )?;

    if table_exists(conn, "site_model_exclusions")? {
        let mut stmt = conn.prepare("SELECT site_id, model_id FROM site_model_exclusions")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        for row in rows {
            let (site_id, model_id) = row?;
            let Some(key_id) = key_by_site.get(&site_id) else {
                continue;
            };
            conn.execute(
                "INSERT OR IGNORE INTO site_model_exclusions_new (site_id, api_key_id, model_id) VALUES (?1,?2,?3)",
                params![site_id, key_id, model_id],
            )?;
        }
        drop(stmt);
        conn.execute_batch("DROP TABLE site_model_exclusions;")?;
    }
    conn.execute_batch(
        r#"
CREATE TABLE site_model_exclusions (
  site_id TEXT NOT NULL REFERENCES sites(id) ON DELETE CASCADE,
  api_key_id TEXT NOT NULL REFERENCES site_api_keys(id) ON DELETE CASCADE,
  model_id TEXT NOT NULL,
  PRIMARY KEY (api_key_id, model_id)
);
INSERT INTO site_model_exclusions SELECT * FROM site_model_exclusions_new;
DROP TABLE site_model_exclusions_new;
"#,
    )?;

    if table_exists(conn, "target_bindings")? {
        ensure_column(
            conn,
            "target_bindings",
            "site_api_key_id",
            "ALTER TABLE target_bindings ADD COLUMN site_api_key_id TEXT",
        )?;
        ensure_column(
            conn,
            "target_bindings",
            "site_api_key_label_snapshot",
            "ALTER TABLE target_bindings ADD COLUMN site_api_key_label_snapshot TEXT",
        )?;
        ensure_column(
            conn,
            "target_bindings",
            "site_api_key_prefix_snapshot",
            "ALTER TABLE target_bindings ADD COLUMN site_api_key_prefix_snapshot TEXT",
        )?;
        conn.execute(
            "UPDATE target_bindings SET site_api_key_id = (SELECT id FROM site_api_keys WHERE site_api_keys.site_id = target_bindings.site_id AND is_active = 1), site_api_key_label_snapshot = 'K 1', site_api_key_prefix_snapshot = (SELECT key_prefix FROM site_api_keys WHERE site_api_keys.site_id = target_bindings.site_id AND is_active = 1) WHERE site_id IS NOT NULL",
            [],
        )?;
    }

    if table_exists(conn, "apply_records")? {
        ensure_column(
            conn,
            "apply_records",
            "site_api_key_id",
            "ALTER TABLE apply_records ADD COLUMN site_api_key_id TEXT",
        )?;
        ensure_column(
            conn,
            "apply_records",
            "site_api_key_label_snapshot",
            "ALTER TABLE apply_records ADD COLUMN site_api_key_label_snapshot TEXT",
        )?;
        ensure_column(
            conn,
            "apply_records",
            "site_api_key_prefix_snapshot",
            "ALTER TABLE apply_records ADD COLUMN site_api_key_prefix_snapshot TEXT",
        )?;
        conn.execute(
            "UPDATE apply_records SET site_api_key_id = (SELECT id FROM site_api_keys WHERE site_api_keys.site_id = apply_records.site_id AND is_active = 1), site_api_key_label_snapshot = 'K 1', site_api_key_prefix_snapshot = (SELECT key_prefix FROM site_api_keys WHERE site_api_keys.site_id = apply_records.site_id AND is_active = 1) WHERE site_id IS NOT NULL",
            [],
        )?;
    }

    conn.execute_batch(
        r#"
CREATE TABLE sites_new (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  base_url TEXT NOT NULL,
  protocol TEXT NOT NULL DEFAULT 'openai_compatible',
  claude_auth_key_style TEXT NOT NULL DEFAULT 'anthropic_auth_token',
  notes TEXT,
  enabled INTEGER NOT NULL DEFAULT 1,
  sort_order INTEGER NOT NULL DEFAULT 0,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  base_urls_json TEXT,
  capabilities_json TEXT
);
INSERT INTO sites_new (id, name, base_url, protocol, claude_auth_key_style, notes, enabled, sort_order, created_at, updated_at, base_urls_json, capabilities_json)
SELECT id, name, base_url, protocol, claude_auth_key_style, notes, enabled, sort_order, created_at, updated_at, base_urls_json, capabilities_json FROM sites;
DROP TABLE sites;
ALTER TABLE sites_new RENAME TO sites;
"#,
    )?;

    conn.execute_batch(V1_SCHEMA)?;

    let sites: i64 = conn.query_row("SELECT COUNT(*) FROM sites", [], |r| r.get(0))?;
    let keys: i64 = conn.query_row("SELECT COUNT(*) FROM site_api_keys", [], |r| r.get(0))?;
    if sites != keys {
        return Err(AppError::new(
            "internal",
            format!("migration key count mismatch: sites={sites} keys={keys}"),
        ));
    }
    let active_sites: i64 = conn.query_row(
        "SELECT COUNT(*) FROM (SELECT site_id FROM site_api_keys WHERE is_active = 1 GROUP BY site_id)",
        [],
        |r| r.get(0),
    )?;
    if active_sites != sites {
        return Err(AppError::new(
            "internal",
            "migration did not produce exactly one active key per site",
        ));
    }

    set_user_version(conn, SCHEMA_VERSION)?;
    Ok(())
}

pub fn verify_encrypted_payloads(conn: &Connection, crypto: &Crypto) -> AppResult<()> {
    let mut payloads = Vec::new();
    if table_exists(conn, "sites")? && column_exists(conn, "sites", "api_key_encrypted")? {
        let mut stmt = conn.prepare("SELECT api_key_encrypted FROM sites")?;
        for value in stmt.query_map([], |row| row.get::<_, String>(0))? {
            payloads.push(value?);
        }
    }
    if table_exists(conn, "site_api_keys")? {
        let mut stmt = conn.prepare("SELECT api_key_encrypted FROM site_api_keys")?;
        for value in stmt.query_map([], |row| row.get::<_, String>(0))? {
            payloads.push(value?);
        }
    }
    if table_exists(conn, "webdav_config")? {
        let mut stmt = conn.prepare("SELECT password_encrypted FROM webdav_config")?;
        for value in stmt.query_map([], |row| row.get::<_, String>(0))? {
            payloads.push(value?);
        }
    }
    for value in payloads {
        if value.trim().is_empty() {
            continue;
        }
        crypto.decrypt(&value).map_err(|_| {
            AppError::new(
                "master_key_missing",
                "stored ciphertext cannot be decrypted with the current master.key",
            )
        })?;
    }
    Ok(())
}

fn maybe_backup(conn: &Connection, mode: BackupMode) -> AppResult<()> {
    if mode == BackupMode::Skip {
        return Ok(());
    }
    let path = conn.path().map(Path::new);
    let is_file = path.is_some_and(|p| p.exists() && p.file_name().is_some());
    if !is_file {
        if mode == BackupMode::Required {
            return Err(AppError::new(
                "backup_failed",
                "cannot create pre-migration backup for an in-memory database",
            ));
        }
        return Ok(());
    }
    backup_pre_migration(conn)
}

fn backup_pre_migration(conn: &Connection) -> AppResult<()> {
    let dest_dir = app_backups_dir()?.join("pre_migration_site_api_keys");
    fs::create_dir_all(&dest_dir)?;
    let dest_db = dest_dir.join("xiaobai-switch.db");
    if dest_db.exists() {
        fs::remove_file(&dest_db)?;
    }
    let escaped = dest_db.to_string_lossy().replace('\'', "''");
    conn.execute_batch(&format!("VACUUM INTO '{escaped}'"))
        .map_err(|e| {
            AppError::new(
                "backup_failed",
                format!("pre-migration database backup failed: {e}"),
            )
        })?;
    let key = master_key_path()?;
    if key.exists() {
        fs::copy(&key, dest_dir.join("master.key")).map_err(|e| {
            AppError::new(
                "backup_failed",
                format!("pre-migration master.key backup failed: {e}"),
            )
        })?;
    } else {
        return Err(AppError::new(
            "master_key_missing",
            "Cannot decrypt stored API keys: master.key is missing",
        ));
    }
    let _ = db_path();
    Ok(())
}

fn backfill_base_urls(conn: &Connection) -> AppResult<()> {
    if !table_exists(conn, "sites")? {
        return Ok(());
    }
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

#[cfg(test)]
pub fn install_legacy_schema(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        r#"
PRAGMA foreign_keys = ON;
CREATE TABLE settings (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  json TEXT NOT NULL
);
CREATE TABLE sites (
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
  updated_at INTEGER NOT NULL,
  base_urls_json TEXT,
  capabilities_json TEXT
);
CREATE TABLE site_models (
  id TEXT PRIMARY KEY,
  site_id TEXT NOT NULL REFERENCES sites(id) ON DELETE CASCADE,
  model_id TEXT NOT NULL,
  display_name TEXT NOT NULL,
  owned_by TEXT,
  raw_json TEXT,
  is_manual INTEGER NOT NULL DEFAULT 0,
  UNIQUE(site_id, model_id)
);
CREATE TABLE site_model_exclusions (
  site_id TEXT NOT NULL REFERENCES sites(id) ON DELETE CASCADE,
  model_id TEXT NOT NULL,
  PRIMARY KEY (site_id, model_id)
);
CREATE TABLE target_bindings (
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
CREATE TABLE apply_records (
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
"#,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::Crypto;

    fn crypto() -> Crypto {
        Crypto::from_key([7u8; 32])
    }

    #[test]
    fn empty_db_installs_v1_without_legacy_key_columns() {
        let conn = Connection::open_in_memory().unwrap();
        apply_schema(&conn, None, BackupMode::Skip).unwrap();
        assert_eq!(user_version(&conn).unwrap(), SCHEMA_VERSION);
        assert!(table_exists(&conn, "site_api_keys").unwrap());
        assert!(table_exists(&conn, "site_thinking_presets").unwrap());
        assert!(!column_exists(&conn, "sites", "api_key_encrypted").unwrap());
    }

    #[test]
    fn repeated_apply_schema_is_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        apply_schema(&conn, None, BackupMode::Skip).unwrap();
        apply_schema(&conn, None, BackupMode::Skip).unwrap();
        assert_eq!(user_version(&conn).unwrap(), SCHEMA_VERSION);
    }

    #[test]
    fn migrates_legacy_site_key_and_models() {
        let conn = Connection::open_in_memory().unwrap();
        install_legacy_schema(&conn).unwrap();
        let crypto = crypto();
        let enc = crypto.encrypt("sk-legacy-secret").unwrap();
        conn.execute(
            "INSERT INTO sites (id, name, base_url, api_key_encrypted, key_prefix, protocol, claude_auth_key_style, notes, enabled, sort_order, selected_model_id, last_model_fetch_at, last_model_fetch_latency_ms, last_model_fetch_error, created_at, updated_at, base_urls_json)
             VALUES ('s1', 'Relay', 'https://api.example.com', ?1, 'sk-l…cret', 'openai_compatible', 'anthropic_auth_token', NULL, 1, 0, 'gpt-4.1', 9, 12, NULL, 1, 1, '[\"https://api.example.com\"]')",
            params![enc],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO site_models (id, site_id, model_id, display_name, owned_by, raw_json, is_manual) VALUES ('m1', 's1', 'gpt-4.1', 'gpt-4.1', 'openai', NULL, 0)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO site_model_exclusions (site_id, model_id) VALUES ('s1', 'hidden')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO target_bindings (target, site_id, site_name_snapshot, model_id, provider_id, key_fingerprint, managed_paths_json, managed_env_keys_json, expected_fields_json, orphan, apply_record_id, applied_at)
             VALUES ('claude_code', 's1', 'Relay', 'gpt-4.1', NULL, 'fp', '[]', '[]', '{}', 0, NULL, 1)",
            [],
        )
        .unwrap();

        apply_schema(&conn, Some(&crypto), BackupMode::Skip).unwrap();
        assert_eq!(user_version(&conn).unwrap(), SCHEMA_VERSION);
        assert!(!column_exists(&conn, "sites", "api_key_encrypted").unwrap());

        let (label, active, selected, stored): (String, i64, Option<String>, String) = conn
            .query_row(
                "SELECT label, is_active, selected_model_id, api_key_encrypted FROM site_api_keys WHERE site_id='s1'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap();
        assert_eq!(label, "K 1");
        assert_eq!(active, 1);
        assert_eq!(selected.as_deref(), Some("gpt-4.1"));
        assert_eq!(crypto.decrypt(&stored).unwrap(), "sk-legacy-secret");

        let model_key: String = conn
            .query_row(
                "SELECT api_key_id FROM site_models WHERE model_id='gpt-4.1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        let exclusion_key: String = conn
            .query_row(
                "SELECT api_key_id FROM site_model_exclusions WHERE model_id='hidden'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        let binding_key: Option<String> = conn
            .query_row(
                "SELECT site_api_key_id FROM target_bindings WHERE target='claude_code'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(Some(model_key.clone()), binding_key);
        assert_eq!(model_key, exclusion_key);
    }

    #[test]
    fn migration_rejects_wrong_master_key() {
        let conn = Connection::open_in_memory().unwrap();
        install_legacy_schema(&conn).unwrap();
        let enc = crypto().encrypt("sk-legacy-secret").unwrap();
        conn.execute(
            "INSERT INTO sites (id, name, base_url, api_key_encrypted, key_prefix, protocol, claude_auth_key_style, notes, enabled, sort_order, selected_model_id, last_model_fetch_at, last_model_fetch_latency_ms, last_model_fetch_error, created_at, updated_at)
             VALUES ('s1', 'Relay', 'https://api.example.com', ?1, 'sk-xx', 'openai_compatible', 'anthropic_auth_token', NULL, 1, 0, NULL, NULL, NULL, NULL, 1, 1)",
            params![enc],
        )
        .unwrap();
        let wrong = Crypto::from_key([8u8; 32]);
        let err = apply_schema(&conn, Some(&wrong), BackupMode::Skip).unwrap_err();
        assert!(err.to_string().contains("cannot be decrypted"));
        assert_eq!(user_version(&conn).unwrap(), 0);
        assert!(!table_exists(&conn, "site_api_keys").unwrap());
    }
}
