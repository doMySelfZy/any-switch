use crate::adapters::atomic::{atomic_write, backup_file, FileLock};
use crate::domain::{McpKind, McpServer};
use crate::error::{AppError, AppResult};
use crate::paths::{
    claude_mcp_json_path, resolve_codex_home, resolve_pi_agent_dir, resolve_prime_agent_dir,
    set_secret_permissions,
};
use serde_json::{Map, Value};
use std::fs;
use std::path::Path;

/// XiaoBaiSwitch 只管理这个前缀下的 MCP 条目：同前缀但不在当前清单里的条目会在应用时
/// 一并清理，这样重命名、禁用、删除或取消目标都不会在客户端里留下孤儿配置。
/// 与 Pi/Prime 的 `xiaobai_` provider 命名空间同一套约定。
const MANAGED_PREFIX: &str = "xiaobai_";

/// Result of applying MCP servers to one target.
#[derive(Debug, Clone)]
pub struct McpTargetResult {
    pub ok: bool,
    pub backup_paths: Vec<String>,
    pub message: String,
}

fn kind_name(kind: McpKind) -> &'static str {
    match kind {
        McpKind::Stdio => "stdio",
        McpKind::Sse => "sse",
        McpKind::Http => "http",
    }
}

fn managed_key(name: &str) -> String {
    format!("{MANAGED_PREFIX}{name}")
}

fn keep_keys(servers: &[McpServer]) -> Vec<String> {
    servers
        .iter()
        .filter(|server| server.enabled)
        .map(|server| managed_key(&server.name))
        .collect()
}

/// 客户端条目 = 用户填的 config 原样透传，再补上缺失的 `type` 与 env/headers。
/// 用户自己在 config 里写 `type` / `transport` 时以用户为准。
fn server_entry(server: &McpServer) -> Value {
    let mut entry = match server.config.as_object() {
        Some(map) => Value::Object(map.clone()),
        None => Value::Object(Map::new()),
    };
    let object = entry
        .as_object_mut()
        .expect("server_entry always builds an object");
    object
        .entry("type")
        .or_insert_with(|| Value::String(kind_name(server.kind).to_string()));
    if has_entries(&server.env) {
        object.insert("env".to_string(), server.env.clone());
    }
    if has_entries(&server.headers) {
        object.insert("headers".to_string(), server.headers.clone());
    }
    entry
}

fn has_entries(value: &Value) -> bool {
    value.as_object().is_some_and(|map| !map.is_empty())
}

/// 合并到 JSON 配置的 `mcpServers`，保留所有非托管条目与文件里的其它未知字段。
pub fn merge_servers_into_json(root: &mut Value, servers: &[McpServer]) -> AppResult<()> {
    let object = root.as_object_mut().ok_or_else(|| {
        AppError::new("invalid_config", "MCP config root must be a JSON object")
    })?;
    let entry = object
        .entry("mcpServers")
        .or_insert_with(|| Value::Object(Map::new()));
    let map = entry.as_object_mut().ok_or_else(|| {
        AppError::new("invalid_config", "existing mcpServers must be a JSON object")
    })?;

    let keep = keep_keys(servers);
    map.retain(|key, _| !key.starts_with(MANAGED_PREFIX) || keep.contains(key));
    for server in servers.iter().filter(|server| server.enabled) {
        map.insert(managed_key(&server.name), server_entry(server));
    }
    Ok(())
}

fn read_json_config(
    path: &Path,
    backup_root: &Path,
    label: &str,
) -> AppResult<(Value, Vec<String>)> {
    if !path.exists() {
        return Ok((Value::Object(Map::new()), Vec::new()));
    }
    let backup = backup_file(path, backup_root)?;
    let text = fs::read_to_string(path)?;
    let root = serde_json::from_str(&text)
        .map_err(|error| AppError::new("invalid_config", format!("invalid {label}: {error}")))?;
    Ok((root, vec![backup.display().to_string()]))
}

fn write_json(path: &Path, root: &Value, secret: bool) -> AppResult<()> {
    let text = serde_json::to_string_pretty(root)? + "\n";
    atomic_write(path, text.as_bytes(), false)?;
    if secret {
        set_secret_permissions(path);
    }
    Ok(())
}

pub fn apply_to_claude(
    servers: &[McpServer],
    claude_home_override: Option<&str>,
    backup_root: &Path,
) -> AppResult<McpTargetResult> {
    let path = claude_mcp_json_path(claude_home_override)?;
    // Claude Code 自己也在写这个文件（会话、信任状态、缓存），读-改-写期间必须持锁。
    let _lock = FileLock::acquire(&path)?;
    let (mut root, backup_paths) = read_json_config(&path, backup_root, "~/.claude.json")?;
    merge_servers_into_json(&mut root, servers)?;
    write_json(&path, &root, true)?;
    Ok(McpTargetResult {
        ok: true,
        backup_paths,
        message: format!("Applied {} MCP servers to Claude Code", servers.len()),
    })
}

pub fn apply_to_codex(
    servers: &[McpServer],
    codex_home_override: Option<&str>,
    backup_root: &Path,
) -> AppResult<McpTargetResult> {
    let path = resolve_codex_home(codex_home_override)?.join("config.toml");
    let _lock = FileLock::acquire(&path)?;
    let mut backup_paths = Vec::new();
    let mut doc = if path.exists() {
        let backup = backup_file(&path, backup_root)?;
        backup_paths.push(backup.display().to_string());
        let text = fs::read_to_string(&path)?;
        text.parse::<toml_edit::DocumentMut>().map_err(|error| {
            AppError::new(
                "invalid_config",
                format!("invalid Codex config.toml: {error}"),
            )
        })?
    } else {
        toml_edit::DocumentMut::new()
    };

    // `mcp_servers` 存在但不是表（例如 `mcp_servers = "oops"` 或 `[[mcp_servers]]`）时，
    // toml_edit 的索引赋值会 panic，这里必须先判定形状并给出可读错误。
    if let Some(item) = doc.get("mcp_servers") {
        if !item.is_table() && !item.is_inline_table() {
            return Err(AppError::new(
                "invalid_config",
                "Codex config.toml has a non-table 'mcp_servers' key",
            ));
        }
    }
    // inline table 形态（`mcp_servers = { … }`）也要参与托管条目清理，
    // 否则它会被整段跳过、留下失效的 xiaobai_* 条目。统一收敛成标准表后只需处理一种形态。
    if doc
        .get("mcp_servers")
        .is_some_and(|item| item.is_inline_table())
    {
        let inline = doc["mcp_servers"]
            .as_inline_table()
            .cloned()
            .unwrap_or_default();
        let mut table = toml_edit::Table::new();
        for (key, value) in inline.iter() {
            table.insert(key, toml_edit::Item::Value(value.clone()));
        }
        doc["mcp_servers"] = toml_edit::Item::Table(table);
    }

    let keep = keep_keys(servers);
    if let Some(section) = doc.get_mut("mcp_servers").and_then(|item| item.as_table_mut()) {
        section.retain(|key, _| !key.starts_with(MANAGED_PREFIX) || keep.iter().any(|k| k == key));
    }

    for server in servers.iter().filter(|server| server.enabled) {
        let key = managed_key(&server.name);
        let mut table = toml_edit::InlineTable::new();
        // Codex 从字段形状判断传输方式：有 command 走 stdio，有 url 走 streamable HTTP。
        if let Some(command) = server.config.get("command").and_then(|v| v.as_str()) {
            table.insert("command", command.into());
        }
        if let Some(args) = server.config.get("args").and_then(|v| v.as_array()) {
            let args: Vec<String> = args
                .iter()
                .filter_map(|value| value.as_str().map(str::to_string))
                .collect();
            if !args.is_empty() {
                table.insert("args", toml_edit::Value::Array(toml_edit::Array::from_iter(args)));
            }
        }
        if let Some(url) = server.config.get("url").and_then(|v| v.as_str()) {
            table.insert("url", url.into());
        }
        if let Some(bearer) = server
            .config
            .get("bearer_token_env_var")
            .and_then(|v| v.as_str())
        {
            table.insert("bearer_token_env_var", bearer.into());
        }
        if let Some(cwd) = server.config.get("cwd").and_then(|v| v.as_str()) {
            table.insert("cwd", cwd.into());
        }
        for (field, source) in [
            ("http_headers", server.config.get("http_headers")),
            ("env_http_headers", server.config.get("env_http_headers")),
        ] {
            if let Some(map) = source.and_then(|value| value.as_object()) {
                if !map.is_empty() {
                    let mut headers = toml_edit::InlineTable::new();
                    for (name, value) in map {
                        if let Some(text) = value.as_str() {
                            headers.insert(name, text.into());
                        }
                    }
                    table.insert(field, toml_edit::Value::InlineTable(headers));
                }
            }
        }
        // 统一表单里的 headers 映射到 Codex 的 http_headers；用户自己写 http_headers 时以它为准。
        if !server.config.get("http_headers").is_some_and(|v| v.is_object())
            && has_entries(&server.headers)
        {
            let mut headers = toml_edit::InlineTable::new();
            if let Some(map) = server.headers.as_object() {
                for (name, value) in map {
                    if let Some(text) = value.as_str() {
                        headers.insert(name, text.into());
                    }
                }
            }
            table.insert("http_headers", toml_edit::Value::InlineTable(headers));
        }
        if has_entries(&server.env) {
            let mut env = toml_edit::InlineTable::new();
            if let Some(map) = server.env.as_object() {
                for (name, value) in map {
                    if let Some(text) = value.as_str() {
                        env.insert(name, text.into());
                    }
                }
            }
            table.insert("env", toml_edit::Value::InlineTable(env));
        }
        if !doc.contains_key("mcp_servers") {
            doc["mcp_servers"] = toml_edit::table();
        }
        doc["mcp_servers"][&key] = toml_edit::value(toml_edit::Value::InlineTable(table));
    }

    atomic_write(&path, doc.to_string().as_bytes(), false)?;

    Ok(McpTargetResult {
        ok: true,
        backup_paths,
        message: format!("Applied {} MCP servers to Codex", servers.len()),
    })
}

pub fn apply_to_pi(
    servers: &[McpServer],
    pi_agent_dir_override: Option<&str>,
    backup_root: &Path,
) -> AppResult<McpTargetResult> {
    let path = resolve_pi_agent_dir(pi_agent_dir_override)?.join("mcp.json");
    let _lock = FileLock::acquire(&path)?;
    let (mut root, backup_paths) = read_json_config(&path, backup_root, "Pi mcp.json")?;
    merge_servers_into_json(&mut root, servers)?;
    write_json(&path, &root, true)?;
    Ok(McpTargetResult {
        ok: true,
        backup_paths,
        message: format!("Applied {} MCP servers to Pi", servers.len()),
    })
}

pub fn apply_to_prime(
    servers: &[McpServer],
    prime_agent_dir_override: Option<&str>,
    backup_root: &Path,
) -> AppResult<McpTargetResult> {
    let path = resolve_prime_agent_dir(prime_agent_dir_override)?.join("settings.json");
    // 与 Prime 适配器共用 `settings.json.lock`，避免和它自己的写入互相覆盖。
    let _lock = FileLock::acquire(&path)?;
    let (mut root, backup_paths) = read_json_config(&path, backup_root, "Prime settings.json")?;
    merge_servers_into_json(&mut root, servers)?;
    write_json(&path, &root, true)?;
    Ok(McpTargetResult {
        ok: true,
        backup_paths,
        message: format!("Applied {} MCP servers to Prime", servers.len()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn server(name: &str, enabled: bool) -> McpServer {
        McpServer {
            id: format!("id-{name}"),
            name: name.to_string(),
            kind: McpKind::Stdio,
            enabled,
            targets: vec![],
            config: json!({"command": "mcp-demo", "args": ["--port", "3000"]}),
            env: json!({"API_KEY": "test"}),
            headers: json!({}),
            created_at: 0,
            updated_at: 0,
        }
    }

    fn temp_backup_root() -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let backup = dir.path().join("backup");
        fs::create_dir_all(&backup).unwrap();
        (dir, backup)
    }

    #[test]
    fn claude_merges_managed_servers_into_claude_json() {
        let (dir, backup) = temp_backup_root();
        let existing = json!({
            "numStartups": 12,
            "mcpServers": {"user-server": {"command": "user-cmd"}}
        });
        fs::write(
            dir.path().join(".claude.json"),
            serde_json::to_string_pretty(&existing).unwrap(),
        )
        .unwrap();

        let result = apply_to_claude(
            &[server("demo", true)],
            Some(dir.path().to_str().unwrap()),
            &backup,
        )
        .unwrap();

        assert!(result.ok);
        assert_eq!(result.backup_paths.len(), 1);
        let text = fs::read_to_string(dir.path().join(".claude.json")).unwrap();
        let root: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(root["numStartups"], 12);
        assert_eq!(root["mcpServers"]["user-server"]["command"], "user-cmd");
        assert_eq!(root["mcpServers"]["xiaobai_demo"]["command"], "mcp-demo");
        assert_eq!(root["mcpServers"]["xiaobai_demo"]["type"], "stdio");
        assert_eq!(root["mcpServers"]["xiaobai_demo"]["env"]["API_KEY"], "test");
    }

    #[test]
    fn claude_does_not_touch_settings_json() {
        let (dir, backup) = temp_backup_root();
        apply_to_claude(&[server("demo", true)], Some(dir.path().to_str().unwrap()), &backup)
            .unwrap();
        assert!(dir.path().join(".claude.json").exists());
        assert!(!dir.path().join("settings.json").exists());
    }

    #[test]
    fn managed_namespace_sweeps_stale_entries_only() {
        let (dir, backup) = temp_backup_root();
        let existing = json!({
            "mcpServers": {
                "xiaobai_removed": {"command": "old"},
                "user-server": {"command": "user-cmd"}
            }
        });
        fs::write(
            dir.path().join(".claude.json"),
            serde_json::to_string(&existing).unwrap(),
        )
        .unwrap();

        apply_to_claude(
            &[server("demo", true), server("disabled", false)],
            Some(dir.path().to_str().unwrap()),
            &backup,
        )
        .unwrap();

        let text = fs::read_to_string(dir.path().join(".claude.json")).unwrap();
        let root: Value = serde_json::from_str(&text).unwrap();
        assert!(root["mcpServers"]["xiaobai_removed"].is_null());
        assert!(root["mcpServers"]["xiaobai_disabled"].is_null());
        assert_eq!(root["mcpServers"]["user-server"]["command"], "user-cmd");
        assert_eq!(root["mcpServers"]["xiaobai_demo"]["command"], "mcp-demo");
    }

    #[test]
    fn malformed_existing_config_is_reported_not_overwritten() {
        let (dir, backup) = temp_backup_root();
        let path = dir.path().join(".claude.json");
        fs::write(&path, r#"{"mcpServers":"not-an-object"}"#).unwrap();

        let error = apply_to_claude(
            &[server("demo", true)],
            Some(dir.path().to_str().unwrap()),
            &backup,
        )
        .unwrap_err();

        assert!(error.to_string().contains("mcpServers"));
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.contains("not-an-object"), "file must stay untouched");
    }

    #[test]
    fn pi_and_prime_use_their_own_files() {
        let (dir, backup) = temp_backup_root();
        let pi_dir = dir.path().join("pi");
        let prime_dir = dir.path().join("prime");

        apply_to_pi(&[server("demo", true)], Some(pi_dir.to_str().unwrap()), &backup).unwrap();
        apply_to_prime(&[server("demo", true)], Some(prime_dir.to_str().unwrap()), &backup).unwrap();

        let pi: Value =
            serde_json::from_str(&fs::read_to_string(pi_dir.join("mcp.json")).unwrap()).unwrap();
        let prime: Value = serde_json::from_str(
            &fs::read_to_string(prime_dir.join("settings.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(pi["mcpServers"]["xiaobai_demo"]["command"], "mcp-demo");
        assert_eq!(prime["mcpServers"]["xiaobai_demo"]["command"], "mcp-demo");
    }

    #[test]
    fn codex_merges_toml_and_keeps_unrelated_tables_and_entries() {
        let (dir, backup) = temp_backup_root();
        let path = dir.path().join("config.toml");
        fs::write(
            &path,
            r#"model = "gpt-5"

[other]
key = "value"

[mcp_servers.xiaobai_removed]
command = "old"

[mcp_servers.user]
command = "user-cmd"
"#,
        )
        .unwrap();

        let result = apply_to_codex(
            &[server("demo", true), server("disabled", false)],
            Some(dir.path().to_str().unwrap()),
            &backup,
        )
        .unwrap();

        assert!(result.ok);
        assert_eq!(result.backup_paths.len(), 1);
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.contains("model = \"gpt-5\""));
        assert!(text.contains("[other]"));
        assert!(text.contains("[mcp_servers.user]"), "user entries preserved");
        assert!(text.contains("xiaobai_demo"));
        assert!(!text.contains("xiaobai_removed"));
        assert!(!text.contains("xiaobai_disabled"));
    }

    #[test]
    fn codex_maps_shared_headers_to_http_headers() {
        let (dir, backup) = temp_backup_root();
        let mut http = server("remote", true);
        http.kind = McpKind::Http;
        http.config = json!({"url": "https://example.com/mcp"});
        http.headers = json!({"Authorization": "Bearer token"});

        apply_to_codex(&[http], Some(dir.path().to_str().unwrap()), &backup).unwrap();

        let text = fs::read_to_string(dir.path().join("config.toml")).unwrap();
        assert!(
            text.contains("http_headers") && text.contains("Authorization"),
            "shared headers must reach Codex's http_headers field: {text}"
        );
    }

    #[test]
    fn codex_prefers_explicit_http_headers_over_shared_headers() {
        let (dir, backup) = temp_backup_root();
        let mut http = server("remote", true);
        http.kind = McpKind::Http;
        http.config = json!({
            "url": "https://example.com/mcp",
            "http_headers": {"X-Explicit": "yes"},
        });
        http.headers = json!({"X-Shared": "no"});

        apply_to_codex(&[http], Some(dir.path().to_str().unwrap()), &backup).unwrap();

        let text = fs::read_to_string(dir.path().join("config.toml")).unwrap();
        assert!(text.contains("X-Explicit"));
        assert!(!text.contains("X-Shared"), "explicit config wins: {text}");
    }

    #[test]
    fn codex_sweeps_stale_entries_inside_an_inline_table() {
        // 回归：inline table 形态的 mcp_servers 也必须参与清理，
        // 不能被 as_table_mut() 的 None 整段跳过。
        let (dir, backup) = temp_backup_root();
        let path = dir.path().join("config.toml");
        fs::write(
            &path,
            r#"mcp_servers = { xiaobai_removed = { command = "old" }, user = { command = "user-cmd" } }
"#,
        )
        .unwrap();

        let result = apply_to_codex(
            &[server("demo", true)],
            Some(dir.path().to_str().unwrap()),
            &backup,
        )
        .unwrap();

        assert!(result.ok);
        let text = fs::read_to_string(&path).unwrap();
        assert!(!text.contains("xiaobai_removed"), "stale entry must be swept: {text}");
        assert!(text.contains("user-cmd"), "user entries preserved: {text}");
        assert!(text.contains("xiaobai_demo"));
    }

    #[test]
    fn codex_creates_file_when_missing() {
        let (dir, backup) = temp_backup_root();
        let result =
            apply_to_codex(&[server("demo", true)], Some(dir.path().to_str().unwrap()), &backup)
                .unwrap();
        assert!(result.backup_paths.is_empty());
        let text = fs::read_to_string(dir.path().join("config.toml")).unwrap();
        assert!(text.contains("xiaobai_demo"));
    }

    #[test]
    fn codex_returns_error_instead_of_panicking_on_non_table_mcp_servers() {
        // 回归：`mcp_servers` 不是表时 toml_edit 的索引赋值会 panic；命令 panic
        // 会直接终止进程，所以这里必须是可读错误，且原文件保持原样。
        for content in ["mcp_servers = \"oops\"\n", "[[mcp_servers]]\ncommand = \"x\"\n"] {
            let (dir, backup) = temp_backup_root();
            let path = dir.path().join("config.toml");
            fs::write(&path, content).unwrap();

            let error = apply_to_codex(
                &[server("demo", true)],
                Some(dir.path().to_str().unwrap()),
                &backup,
            )
            .unwrap_err();

            assert!(error.to_string().contains("mcp_servers"));
            assert_eq!(fs::read_to_string(&path).unwrap(), content);
        }
    }

    #[test]
    fn codex_passes_through_cwd_and_http_headers() {
        let (dir, backup) = temp_backup_root();
        let mut http = server("remote", true);
        http.kind = McpKind::Http;
        http.config = json!({
            "url": "https://example.com/mcp",
            "cwd": "/tmp/work",
            "http_headers": {"X-Api-Key": "abc"},
            "bearer_token_env_var": "MY_TOKEN",
        });

        apply_to_codex(&[http], Some(dir.path().to_str().unwrap()), &backup).unwrap();

        let text = fs::read_to_string(dir.path().join("config.toml")).unwrap();
        assert!(text.contains("cwd"));
        assert!(text.contains("http_headers"));
        assert!(text.contains("X-Api-Key"));
        assert!(text.contains("MY_TOKEN"));
    }

    #[test]
    fn writes_are_serialized_by_the_config_lock() {
        // 持锁期间再写同一个目标必须报 lock_busy，而不是并发读-改-写丢更新。
        let (dir, backup) = temp_backup_root();
        let codex_dir = dir.path().join("codex");
        fs::create_dir_all(&codex_dir).unwrap();
        let _held = FileLock::acquire(&codex_dir.join("config.toml")).unwrap();

        let error = apply_to_codex(
            &[server("demo", true)],
            Some(codex_dir.to_str().unwrap()),
            &backup,
        )
        .unwrap_err();
        assert!(error.to_string().contains("using"));
    }

    #[test]
    fn lock_is_released_after_a_successful_apply() {
        let (dir, backup) = temp_backup_root();
        let pi_dir = dir.path().join("pi");
        apply_to_pi(&[server("demo", true)], Some(pi_dir.to_str().unwrap()), &backup).unwrap();
        assert!(!pi_dir.join("mcp.json.lock").exists());
    }
}
