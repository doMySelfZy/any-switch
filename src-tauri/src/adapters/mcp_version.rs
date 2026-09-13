//! MCP 版本检查和更新功能。
//!
//! 通过解析 MCP 配置中的命令（npx/uvx 等），从包管理器获取当前安装版本和最新可用版本。

use crate::domain::{AppSettings, McpBatchUpdateResult, McpKind, McpServer, McpServerSummary, McpUpdateInfo};
use crate::error::{AppError, AppResult};
use crate::repo;
use crate::state::AppState;
use chrono::Utc;
use serde_json::Value;
use std::process::Command;

/// 从 MCP 配置中提取包名和版本信息
fn extract_package_info(config: &Value, kind: McpKind) -> Option<(String, String)> {
    match kind {
        McpKind::Stdio => {
            // npx -y @modelcontextprotocol/server-filesystem
            // uvx mcp-server-git
            let command = config.get("command")?.as_str()?;
            let args = config.get("args")?.as_array()?;

            if command == "npx" && args.len() >= 2 {
                // npx -y package@version 或 npx -y package
                let package_arg = args.iter()
                    .find(|v| v.as_str().map(|s| !s.starts_with('-')).unwrap_or(false))?
                    .as_str()?;

                return Some(("npm".to_string(), package_arg.to_string()));
            } else if command == "uvx" && !args.is_empty() {
                // uvx package==version 或 uvx package
                let package_arg = args[0].as_str()?;
                return Some(("pypi".to_string(), package_arg.to_string()));
            }
            None
        }
        _ => None, // SSE 和 HTTP 暂不支持版本检查
    }
}

/// 检查 npm 包的版本信息
async fn check_npm_version(package: &str) -> AppResult<(Option<String>, Option<String>)> {
    // 解析包名和版本（如果有）
    let (pkg_name, specified_version) = if let Some(pos) = package.find('@') {
        if pos == 0 {
            // @scope/package 或 @scope/package@version
            if let Some(second_at) = package[1..].find('@') {
                (&package[..second_at + 1], Some(&package[second_at + 2..]))
            } else {
                (package, None)
            }
        } else {
            // package@version
            (&package[..pos], Some(&package[pos + 1..]))
        }
    } else {
        (package, None)
    };

    // 获取最新版本
    let output = Command::new("npm")
        .args(["view", pkg_name, "version"])
        .output()
        .map_err(|e| AppError::new("npm_error", format!("执行 npm 失败: {}", e)))?;

    if !output.status.success() {
        return Err(AppError::new(
            "npm_error",
            format!("npm view 失败: {}", String::from_utf8_lossy(&output.stderr)),
        ));
    }

    let latest = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let latest = if latest.is_empty() { None } else { Some(latest) };

    Ok((specified_version.map(|s| s.to_string()), latest))
}

/// 检查 PyPI 包的版本信息
async fn check_pypi_version(package: &str) -> AppResult<(Option<String>, Option<String>)> {
    // 解析包名和版本
    let (pkg_name, specified_version) = if let Some(pos) = package.find("==") {
        (&package[..pos], Some(&package[pos + 2..]))
    } else {
        (package, None)
    };

    // 使用 pip index versions 获取最新版本
    let output = Command::new("pip")
        .args(["index", "versions", pkg_name])
        .output()
        .map_err(|e| AppError::new("pip_error", format!("执行 pip 失败: {}", e)))?;

    if !output.status.success() {
        return Err(AppError::new(
            "pip_error",
            format!("pip index versions 失败: {}", String::from_utf8_lossy(&output.stderr)),
        ));
    }

    // 解析输出获取最新版本
    let output_str = String::from_utf8_lossy(&output.stdout);
    let latest = output_str
        .lines()
        .find(|line| line.contains("Available versions:"))
        .and_then(|line| {
            line.split_whitespace()
                .nth(2) // "Available versions: x.y.z, ..."
                .map(|v| v.trim_end_matches(',').to_string())
        });

    Ok((specified_version.map(|s| s.to_string()), latest))
}

/// 检查单个 MCP 的更新状态
async fn check_single_update(
    server: &McpServer,
    _settings: &AppSettings,
) -> AppResult<McpUpdateInfo> {
    let now = Utc::now().timestamp_millis();

    let (package_manager, package) = match extract_package_info(&server.config, server.kind) {
        Some(info) => info,
        None => {
            return Ok(McpUpdateInfo {
                id: server.id.clone(),
                name: server.name.clone(),
                current_version: None,
                latest_version: None,
                update_available: false,
                checked_at: now,
            });
        }
    };

    let (current, latest) = match package_manager.as_str() {
        "npm" => check_npm_version(&package).await?,
        "pypi" => check_pypi_version(&package).await?,
        _ => (None, None),
    };

    let update_available = match (&current, &latest) {
        (Some(curr), Some(lat)) => curr != lat,
        (None, Some(_)) => true, // 没指定版本但有最新版，算有更新
        _ => false,
    };

    Ok(McpUpdateInfo {
        id: server.id.clone(),
        name: server.name.clone(),
        current_version: current,
        latest_version: latest,
        update_available,
        checked_at: now,
    })
}

/// 检查所有 MCP 的更新状态
pub async fn check_updates(
    state: &AppState,
    servers: &[McpServerSummary],
    settings: &AppSettings,
) -> AppResult<Vec<McpUpdateInfo>> {
    let mut results = Vec::new();
    let now = Utc::now().timestamp_millis();

    for server_summary in servers {
        // 获取完整的 MCP 配置（包含 config 字段）
        let server = match state.db.with_conn(|conn| repo::mcp::get(conn, &server_summary.id, &state.crypto)) {
            Ok(s) => s,
            Err(_) => continue, // 获取失败则跳过
        };

        match check_single_update(&server, settings).await {
            Ok(info) => {
                // 更新数据库中的版本信息
                let _ = state.db.with_conn(|conn| {
                    repo::mcp_version::update_version_info(
                        conn,
                        &server.id,
                        info.current_version.clone(),
                        info.latest_version.clone(),
                        now,
                    )
                });
                results.push(info);
            }
            Err(e) => {
                // 检查失败也记录下来，但不中断整体流程
                results.push(McpUpdateInfo {
                    id: server_summary.id.clone(),
                    name: server_summary.name.clone(),
                    current_version: None,
                    latest_version: None,
                    update_available: false,
                    checked_at: now,
                });
                eprintln!("检查 {} 更新失败: {}", server.name, e);
            }
        }
    }

    Ok(results)
}

/// 更新单个 MCP
pub async fn update_single(
    state: &AppState,
    server: &McpServer,
    _settings: &AppSettings,
) -> AppResult<McpUpdateInfo> {
    let (package_manager, package) = extract_package_info(&server.config, server.kind)
        .ok_or_else(|| AppError::new("unsupported", "该 MCP 不支持自动更新"))?;

    // 解析包名（去掉版本号）
    let pkg_name = match package_manager.as_str() {
        "npm" => {
            if let Some(pos) = package.find('@') {
                if pos == 0 {
                    // @scope/package@version
                    package.rfind('@').map(|p| &package[..p]).unwrap_or(&package)
                } else {
                    &package[..pos]
                }
            } else {
                &package
            }
        }
        "pypi" => package.split("==").next().unwrap_or(&package),
        _ => return Err(AppError::new("unsupported", "不支持的包管理器")),
    };

    // 执行更新
    let success = match package_manager.as_str() {
        "npm" => {
            let output = Command::new("npm")
                .args(["install", "-g", pkg_name])
                .output()
                .map_err(|e| AppError::new("npm_error", format!("执行 npm 失败: {}", e)))?;
            output.status.success()
        }
        "pypi" => {
            let output = Command::new("pip")
                .args(["install", "--upgrade", pkg_name])
                .output()
                .map_err(|e| AppError::new("pip_error", format!("执行 pip 失败: {}", e)))?;
            output.status.success()
        }
        _ => false,
    };

    if !success {
        return Err(AppError::new("update_failed", "更新失败"));
    }

    // 重新检查版本
    let (current, latest) = match package_manager.as_str() {
        "npm" => check_npm_version(pkg_name).await?,
        "pypi" => check_pypi_version(pkg_name).await?,
        _ => (None, None),
    };

    let now = Utc::now().timestamp_millis();

    // 更新数据库
    state.db.with_conn(|conn| {
        repo::mcp_version::update_version_info(
            conn,
            &server.id,
            current.clone(),
            latest.clone(),
            now,
        )
    })?;

    Ok(McpUpdateInfo {
        id: server.id.clone(),
        name: server.name.clone(),
        current_version: current.clone(),
        latest_version: latest,
        update_available: false, // 刚更新完，应该没有更新了
        checked_at: now,
    })
}

/// 批量更新多个 MCP
pub async fn batch_update(
    state: &AppState,
    servers: &[McpServer],
    settings: &AppSettings,
) -> AppResult<McpBatchUpdateResult> {
    let mut updated = Vec::new();
    let mut failed = Vec::new();

    for server in servers {
        match update_single(state, server, settings).await {
            Ok(_) => updated.push(server.id.clone()),
            Err(e) => failed.push((server.id.clone(), e.to_string())),
        }
    }

    Ok(McpBatchUpdateResult {
        updated,
        failed,
        total: servers.len(),
    })
}
