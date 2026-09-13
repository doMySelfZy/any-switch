//! MCP 版本信息的数据库操作。

use crate::error::AppResult;
use rusqlite::{params, Connection};

/// 更新 MCP 的版本信息和最后检查时间
pub fn update_version_info(
    conn: &Connection,
    id: &str,
    current_version: Option<String>,
    latest_version: Option<String>,
    checked_at: i64,
) -> AppResult<()> {
    conn.execute(
        "UPDATE mcp_servers
         SET current_version = ?, latest_version = ?, last_update_check_at = ?
         WHERE id = ?",
        params![current_version, latest_version, checked_at, id],
    )?;
    Ok(())
}

/// 获取某个 MCP 的版本信息
pub fn get_version_info(
    conn: &Connection,
    id: &str,
) -> AppResult<(Option<String>, Option<String>, Option<i64>)> {
    let result = conn.query_row(
        "SELECT current_version, latest_version, last_update_check_at
         FROM mcp_servers
         WHERE id = ?",
        params![id],
        |row| {
            Ok((
                row.get::<_, Option<String>>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<i64>>(2)?,
            ))
        },
    )?;
    Ok(result)
}

/// 清除版本信息（用于删除或重置）
pub fn clear_version_info(conn: &Connection, id: &str) -> AppResult<()> {
    conn.execute(
        "UPDATE mcp_servers
         SET current_version = NULL, latest_version = NULL, last_update_check_at = NULL
         WHERE id = ?",
        params![id],
    )?;
    Ok(())
}
