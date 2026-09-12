use crate::error::AppResult;
use rusqlite::{params, Connection, OptionalExtension};

/// 同步引擎的本地小账本（键值对）：
/// - last_synced_database_sha256 / last_synced_master_key_sha256：上次与远端达成一致时的数据指纹；
/// - last_published_revision：本机最近发布的远端版本号。
pub fn get_meta(conn: &Connection, key: &str) -> AppResult<Option<String>> {
    conn.query_row(
        "SELECT value FROM sync_meta WHERE key = ?1",
        [key],
        |row| row.get(0),
    )
    .optional()
    .map_err(Into::into)
}

pub fn set_meta(conn: &Connection, key: &str, value: &str) -> AppResult<()> {
    conn.execute(
        "INSERT INTO sync_meta (key, value, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(key) DO UPDATE SET
           value = excluded.value,
           updated_at = excluded.updated_at",
        params![key, value, chrono::Utc::now().timestamp_millis()],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meta_round_trip_and_upsert() {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::apply_schema(&conn).unwrap();
        assert_eq!(get_meta(&conn, "missing").unwrap(), None);
        set_meta(&conn, "a", "1").unwrap();
        set_meta(&conn, "a", "2").unwrap();
        assert_eq!(get_meta(&conn, "a").unwrap().as_deref(), Some("2"));
    }
}
