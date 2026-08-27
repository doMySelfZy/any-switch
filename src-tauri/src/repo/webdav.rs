use crate::domain::WebDavSyncStatus;
use crate::error::AppResult;
use rusqlite::{params, Connection, OptionalExtension};

#[derive(Debug, Clone)]
pub struct StoredWebDavConfig {
    pub base_url: String,
    pub username: String,
    pub password_encrypted: String,
    pub remote_path: String,
    pub accept_invalid_certs: bool,
    pub auto_sync_enabled: bool,
    pub sync_interval_minutes: u32,
    pub max_remote_backups: u32,
}

pub fn get_config(conn: &Connection) -> AppResult<Option<StoredWebDavConfig>> {
    conn.query_row(
        "SELECT base_url, username, password_encrypted, remote_path,
                accept_invalid_certs, auto_sync_enabled, sync_interval_minutes,
                max_remote_backups
         FROM webdav_config WHERE id = 1",
        [],
        |row| {
            Ok(StoredWebDavConfig {
                base_url: row.get(0)?,
                username: row.get(1)?,
                password_encrypted: row.get(2)?,
                remote_path: row.get(3)?,
                accept_invalid_certs: row.get::<_, i64>(4)? != 0,
                auto_sync_enabled: row.get::<_, i64>(5)? != 0,
                sync_interval_minutes: row.get(6)?,
                max_remote_backups: row.get(7)?,
            })
        },
    )
    .optional()
    .map_err(Into::into)
}

pub fn save_config(conn: &Connection, config: &StoredWebDavConfig) -> AppResult<()> {
    conn.execute(
        "INSERT INTO webdav_config (
           id, base_url, username, password_encrypted, remote_path,
           accept_invalid_certs, auto_sync_enabled, sync_interval_minutes,
           max_remote_backups, updated_at
         ) VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
         ON CONFLICT(id) DO UPDATE SET
           base_url = excluded.base_url,
           username = excluded.username,
           password_encrypted = excluded.password_encrypted,
           remote_path = excluded.remote_path,
           accept_invalid_certs = excluded.accept_invalid_certs,
           auto_sync_enabled = excluded.auto_sync_enabled,
           sync_interval_minutes = excluded.sync_interval_minutes,
           max_remote_backups = excluded.max_remote_backups,
           updated_at = excluded.updated_at",
        params![
            config.base_url,
            config.username,
            config.password_encrypted,
            config.remote_path,
            config.accept_invalid_certs as i64,
            config.auto_sync_enabled as i64,
            config.sync_interval_minutes,
            config.max_remote_backups,
            chrono::Utc::now().timestamp_millis(),
        ],
    )?;
    Ok(())
}

pub fn get_sync_status(conn: &Connection) -> AppResult<WebDavSyncStatus> {
    Ok(conn
        .query_row(
            "SELECT last_attempt_at, last_success_at, status, error
             FROM webdav_sync_state WHERE id = 1",
            [],
            |row| {
                Ok(WebDavSyncStatus {
                    last_attempt_at: row.get(0)?,
                    last_success_at: row.get(1)?,
                    status: row.get(2)?,
                    error: row.get(3)?,
                })
            },
        )
        .optional()?
        .unwrap_or_default())
}

pub fn record_sync_status(
    conn: &Connection,
    status: &str,
    error: Option<&str>,
    success: bool,
) -> AppResult<WebDavSyncStatus> {
    let now = chrono::Utc::now().timestamp_millis();
    conn.execute(
        "INSERT INTO webdav_sync_state (
           id, last_attempt_at, last_success_at, status, error
         ) VALUES (1, ?1, ?2, ?3, ?4)
         ON CONFLICT(id) DO UPDATE SET
           last_attempt_at = excluded.last_attempt_at,
           last_success_at = CASE
             WHEN ?5 = 1 THEN excluded.last_success_at
             ELSE webdav_sync_state.last_success_at
           END,
           status = excluded.status,
           error = excluded.error",
        params![
            now,
            if success { Some(now) } else { None },
            status,
            error,
            success as i64,
        ],
    )?;
    get_sync_status(conn)
}

pub fn clear_sync_status(conn: &Connection) -> AppResult<()> {
    conn.execute("DELETE FROM webdav_sync_state WHERE id = 1", [])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_and_status_round_trip() {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::apply_schema(&conn).unwrap();
        let config = StoredWebDavConfig {
            base_url: "https://dav.example.com/".into(),
            username: "alice".into(),
            password_encrypted: "ciphertext".into(),
            remote_path: "xiaobai-switch".into(),
            accept_invalid_certs: false,
            auto_sync_enabled: true,
            sync_interval_minutes: 60,
            max_remote_backups: 10,
        };
        save_config(&conn, &config).unwrap();
        let restored = get_config(&conn).unwrap().unwrap();
        assert_eq!(restored.password_encrypted, "ciphertext");
        assert!(restored.auto_sync_enabled);

        record_sync_status(&conn, "failed", Some("network"), false).unwrap();
        record_sync_status(&conn, "success", None, true).unwrap();
        let status = get_sync_status(&conn).unwrap();
        assert_eq!(status.status, "success");
        assert!(status.last_success_at.is_some());
        assert!(status.error.is_none());
    }
}
