//! MCP 版本检查与更新的 Tauri 命令。

use crate::adapters::mcp_update;
use crate::error::AppResult;
use crate::repo::{mcp, mcp_version};
use crate::state::AppState;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Serialize, Deserialize)]
pub struct McpUpdateStatus {
    pub id: String,
    pub name: String,
    pub current_version: Option<String>,
    pub latest_version: Option<String>,
    pub has_update: bool,
    pub last_check_at: Option<i64>,
}

/// 检查所有已安装 MCP 的更新状态
#[tauri::command]
pub async fn check_mcp_updates(state: State<'_, AppState>) -> AppResult<Vec<McpUpdateStatus>> {
    let servers = state
        .db
        .with_conn(|conn| mcp::list_full(conn, &state.crypto))?;

    let mut results = Vec::new();
    let now = Utc::now().timestamp();

    for server in servers {
        // 从 config 中提取包名
        let package_name = extract_package_name(&server.config);

        let (current_version, latest_version, has_update) = if let Some(pkg_name) = package_name {
            // 检查版本
            match mcp_update::check_npm_package_version(&pkg_name).await {
                Ok(result) => {
                    // 更新数据库
                    state.db.with_conn(|conn| {
                        mcp_version::update_version_info(
                            conn,
                            &server.id,
                            result.current_version.clone(),
                            result.latest_version.clone(),
                            now,
                        )
                    })?;

                    (
                        result.current_version,
                        result.latest_version,
                        result.has_update,
                    )
                }
                Err(e) => {
                    tracing::warn!("检查 MCP {} 版本失败: {}", server.name, e);
                    (server.current_version, server.latest_version, false)
                }
            }
        } else {
            // 无法提取包名，使用数据库中的信息
            (server.current_version.clone(), server.latest_version.clone(), false)
        };

        results.push(McpUpdateStatus {
            id: server.id,
            name: server.name,
            current_version,
            latest_version,
            has_update,
            last_check_at: Some(now),
        });
    }

    Ok(results)
}

/// 更新单个 MCP 到最新版本
#[tauri::command]
pub async fn update_mcp_server(state: State<'_, AppState>, id: String) -> AppResult<String> {
    // 获取服务器信息
    let server = state
        .db
        .with_conn(|conn| mcp::get(conn, &id, &state.crypto))?;

    // 提取包名
    let package_name = extract_package_name(&server.config)
        .ok_or_else(|| crate::error::AppError::new("mcp_update", "无法确定包名"))?;

    // 执行更新
    let updated_version = mcp_update::update_npm_package(&package_name).await?;

    // 更新数据库
    let now = Utc::now().timestamp();
    state.db.with_conn(|conn| {
        mcp_version::update_version_info(
            conn,
            &id,
            Some(updated_version.clone()),
            Some(updated_version.clone()),
            now,
        )
    })?;

    Ok(updated_version)
}

/// 批量更新所有有更新的 MCP
#[tauri::command]
pub async fn batch_update_mcp_servers(
    state: State<'_, AppState>,
    ids: Vec<String>,
) -> AppResult<Vec<(String, Result<String, String>)>> {
    let mut results = Vec::new();

    for id in ids {
        let result = match update_mcp_server(state.clone(), id.clone()).await {
            Ok(version) => Ok(version),
            Err(e) => Err(e.to_string()),
        };
        results.push((id, result));
    }

    Ok(results)
}

/// 从 MCP config 中提取 npm 包名
fn extract_package_name(config: &serde_json::Value) -> Option<String> {
    // 尝试从 command 字段提取
    if let Some(command) = config.get("command").and_then(|v| v.as_str()) {
        // npx @modelcontextprotocol/server-filesystem
        if command.starts_with("npx ") {
            let package = command.strip_prefix("npx ")?.trim();
            // 移除可能的参数
            let package = package.split_whitespace().next()?;
            return Some(package.to_string());
        }

        // 直接是包名的情况
        if command.contains("@modelcontextprotocol/") || command.contains("mcp-") {
            return Some(command.to_string());
        }
    }

    // 尝试从 url 字段提取 (对于 npx 远程包)
    if let Some(url) = config.get("url").and_then(|v| v.as_str()) {
        if url.contains("npm:") {
            let package = url.strip_prefix("npm:")?;
            return Some(package.to_string());
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_extract_package_name() {
        let config1 = json!({
            "command": "npx @modelcontextprotocol/server-filesystem"
        });
        assert_eq!(
            extract_package_name(&config1),
            Some("@modelcontextprotocol/server-filesystem".to_string())
        );

        let config2 = json!({
            "command": "npx @modelcontextprotocol/server-filesystem /path/to/dir"
        });
        assert_eq!(
            extract_package_name(&config2),
            Some("@modelcontextprotocol/server-filesystem".to_string())
        );
    }
}
