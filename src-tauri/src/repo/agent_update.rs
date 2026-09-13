use crate::domain::AgentUpdateStatus;
use crate::error::AppResult;
use rusqlite::{Connection, OptionalExtension};

pub fn upsert_agent_update_status(conn: &Connection, status: &AgentUpdateStatus) -> AppResult<()> {
    conn.execute(
        "INSERT INTO agent_update_status (kind, name, current_version, latest_version, has_update, last_check_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(kind) DO UPDATE SET
           name = excluded.name,
           current_version = excluded.current_version,
           latest_version = excluded.latest_version,
           has_update = excluded.has_update,
           last_check_at = excluded.last_check_at",
        (
            &status.kind,
            &status.name,
            &status.current_version,
            &status.latest_version,
            status.has_update as i32,
            status.last_check_at,
        ),
    )?;
    Ok(())
}

pub fn get_agent_update_status(
    conn: &Connection,
    kind: &str,
) -> AppResult<Option<AgentUpdateStatus>> {
    let mut stmt = conn.prepare(
        "SELECT kind, name, current_version, latest_version, has_update, last_check_at
         FROM agent_update_status WHERE kind = ?1",
    )?;

    let status = stmt
        .query_row([kind], |row| {
            Ok(AgentUpdateStatus {
                kind: row.get(0)?,
                name: row.get(1)?,
                current_version: row.get(2)?,
                latest_version: row.get(3)?,
                has_update: row.get::<_, i32>(4)? != 0,
                last_check_at: row.get(5)?,
            })
        })
        .optional()?;

    Ok(status)
}

pub fn get_all_agent_update_statuses(conn: &Connection) -> AppResult<Vec<AgentUpdateStatus>> {
    let mut stmt = conn.prepare(
        "SELECT kind, name, current_version, latest_version, has_update, last_check_at
         FROM agent_update_status ORDER BY kind",
    )?;

    let statuses = stmt
        .query_map([], |row| {
            Ok(AgentUpdateStatus {
                kind: row.get(0)?,
                name: row.get(1)?,
                current_version: row.get(2)?,
                latest_version: row.get(3)?,
                has_update: row.get::<_, i32>(4)? != 0,
                last_check_at: row.get(5)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(statuses)
}

pub fn delete_agent_update_status(conn: &Connection, kind: &str) -> AppResult<()> {
    conn.execute("DELETE FROM agent_update_status WHERE kind = ?1", [kind])?;
    Ok(())
}
