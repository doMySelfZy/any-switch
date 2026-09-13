use crate::{
    adapters::mcp as mcp_adapters,
    domain::{AppSettings, McpServer, McpServerInput, McpServerSummary, TargetKind},
    error::AppResult,
    paths::backups_dir,
    repo,
    state::AppState,
};
use serde::{Deserialize, Serialize};
use tauri::State;

#[tauri::command]
pub fn list_mcp_servers(state: State<'_, AppState>) -> AppResult<Vec<McpServerSummary>> {
    state.db.with_conn(|conn| repo::mcp::list(conn, &state.crypto))
}

#[tauri::command]
pub fn get_mcp_server(state: State<'_, AppState>, id: String) -> AppResult<McpServer> {
    state.db
        .with_conn(|conn| repo::mcp::get(conn, &id, &state.crypto))
}

#[tauri::command]
pub fn save_mcp_server(
    state: State<'_, AppState>,
    input: McpServerInput,
) -> AppResult<McpSaveResult> {
    let server = state
        .db
        .with_conn(|conn| repo::mcp::save(conn, &state.crypto, input))?;
    // 之前应用过的目标要重新同步一遍：改名或取消目标后，旧客户端里的托管条目才能清掉。
    // 同步失败不能反过来判定保存失败（数据已落库），但必须把目标级结果回给界面。
    let sweep = apply_to_targets(&state, &[])?;
    Ok(McpSaveResult { server, sweep })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpSaveResult {
    pub server: McpServer,
    pub sweep: McpApplyResult,
}

#[tauri::command]
pub fn delete_mcp_server(state: State<'_, AppState>, id: String) -> AppResult<McpApplyResult> {
    state.db.with_conn(|conn| repo::mcp::delete(conn, &id))?;
    apply_to_targets(&state, &[])
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpApplyTargetResult {
    pub target: TargetKind,
    pub ok: bool,
    pub backup_paths: Vec<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpApplyResult {
    pub results: Vec<McpApplyTargetResult>,
    pub applied_at: i64,
}

#[tauri::command]
pub fn apply_mcp_servers(
    state: State<'_, AppState>,
    targets: Vec<TargetKind>,
) -> AppResult<McpApplyResult> {
    apply_to_targets(&state, &targets)
}

/// 需要写盘的目标 = 本次请求 ∪ 上次应用过的目标。
///
/// 之所以要并上「上次应用过的目标」：用户把某个 MCP 从目标里移除、改名或禁用后，
/// 那个客户端里已经写入的托管条目必须被清掉，否则会留下孤儿配置。
/// 删除 MCP 时请求列表为空，此时完全依赖这个并集来清理。
fn merged_targets(requested: &[TargetKind], previously_applied: &[TargetKind]) -> Vec<TargetKind> {
    let mut union = requested.to_vec();
    for target in previously_applied {
        if !union.contains(target) {
            union.push(*target);
        }
    }
    union
}

/// 应用后要记下的目标集合 = 「清理后仍然持有托管条目的目标」∪「本次写失败的目标」。
///
/// - 仍然持有条目：该目标上还有启用的 MCP 指向它（`desired`），且本次写成功。
///   一个目标被清理干净后就不该继续留在记录里，否则以后每次保存都会重写它那份
///   客户端配置（`~/.claude.json` 这类文件是客户端自己在频繁改写的，不该被无谓触碰）。
/// - 写失败的目标：保留下来，下次仍然参与清理重试。
fn targets_to_record(
    results: &[McpApplyTargetResult],
    desired: &[TargetKind],
) -> Vec<TargetKind> {
    let mut recorded: Vec<TargetKind> = Vec::new();
    for result in results {
        let keep = !result.ok || desired.contains(&result.target);
        if keep && !recorded.contains(&result.target) {
            recorded.push(result.target);
        }
    }
    recorded
}

/// 当前有启用的 MCP 指向的目标集合：只有这些目标在清理后还需要保留托管条目。
fn targets_with_enabled_servers(servers: &[McpServer]) -> Vec<TargetKind> {
    let mut desired = Vec::new();
    for server in servers.iter().filter(|server| server.enabled) {
        for target in &server.targets {
            if !desired.contains(target) {
                desired.push(*target);
            }
        }
    }
    desired
}

/// 把数据库里的 MCP 现状写到目标客户端。
///
/// 实际写入的目标 = 本次请求的目标 ∪ 上次应用过的目标：这样用户把某个 MCP 从目标里移除后，
/// 那个客户端里的托管条目会被清掉，而不是留成孤儿。空目标列表用于「删除后清理」。
fn apply_to_targets(state: &AppState, requested: &[TargetKind]) -> AppResult<McpApplyResult> {
    let settings = state.db.with_conn(repo::settings::get_settings)?;
    let servers = state.db.with_conn(|conn| repo::mcp::list_full(conn, &state.crypto))?;
    let previously_applied = state.db.with_conn(repo::mcp::applied_targets)?;

    let union = merged_targets(requested, &previously_applied);

    let backup_root = backups_dir()?.join("mcp");
    std::fs::create_dir_all(&backup_root)?;

    let mut results = Vec::new();
    for target in &union {
        let target_servers: Vec<McpServer> = servers
            .iter()
            .filter(|server| server.targets.contains(target))
            .cloned()
            .collect();

        let outcome = match target {
            TargetKind::ClaudeCode => mcp_adapters::apply_to_claude(
                &target_servers,
                settings.claude_home_override.as_deref(),
                &backup_root,
            ),
            TargetKind::Codex => mcp_adapters::apply_to_codex(
                &target_servers,
                settings.codex_home_override.as_deref(),
                &backup_root,
            ),
            TargetKind::Pi => mcp_adapters::apply_to_pi(
                &target_servers,
                settings.pi_agent_dir_override.as_deref(),
                &backup_root,
            ),
            TargetKind::Prime => mcp_adapters::apply_to_prime(
                &target_servers,
                settings.prime_agent_dir_override.as_deref(),
                &backup_root,
            ),
        };

        match outcome {
            Ok(result) => results.push(McpApplyTargetResult {
                target: *target,
                ok: result.ok,
                backup_paths: result.backup_paths,
                message: result.message,
            }),
            Err(error) => results.push(McpApplyTargetResult {
                target: *target,
                ok: false,
                backup_paths: Vec::new(),
                message: error.to_string(),
            }),
        }
    }

    let desired = targets_with_enabled_servers(&servers);
    let merged = targets_to_record(&results, &desired);
    state
        .db
        .with_conn(|conn| repo::mcp::record_applied_targets(conn, &merged))?;

    Ok(McpApplyResult {
        results,
        applied_at: chrono::Utc::now().timestamp_millis(),
    })
}

/// 供设置面板展示目标配置文件的实际落点。
#[tauri::command]
pub fn mcp_target_paths(state: State<'_, AppState>) -> AppResult<Vec<(TargetKind, String)>> {
    let settings: AppSettings = state.db.with_conn(repo::settings::get_settings)?;
    Ok(vec![
        (
            TargetKind::ClaudeCode,
            crate::paths::claude_mcp_json_path(settings.claude_home_override.as_deref())?
                .display()
                .to_string(),
        ),
        (
            TargetKind::Codex,
            crate::paths::resolve_codex_home(settings.codex_home_override.as_deref())?
                .join("config.toml")
                .display()
                .to_string(),
        ),
        (
            TargetKind::Pi,
            crate::paths::resolve_pi_agent_dir(settings.pi_agent_dir_override.as_deref())?
                .join("mcp.json")
                .display()
                .to_string(),
        ),
        (
            TargetKind::Prime,
            crate::paths::resolve_prime_agent_dir(settings.prime_agent_dir_override.as_deref())?
                .join("settings.json")
                .display()
                .to_string(),
        ),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn result(target: TargetKind, ok: bool) -> McpApplyTargetResult {
        McpApplyTargetResult {
            target,
            ok,
            backup_paths: Vec::new(),
            message: String::new(),
        }
    }

    #[test]
    fn cleanup_covers_targets_that_were_applied_before() {
        // 用户把 MCP 从 Claude 改绑到 Codex 后，请求里只有 Codex，
        // 但 Claude 里已经写过的托管条目必须一起清掉。
        let union = merged_targets(&[TargetKind::Codex], &[TargetKind::ClaudeCode]);
        assert_eq!(union.len(), 2);
        assert!(union.contains(&TargetKind::Codex));
        assert!(union.contains(&TargetKind::ClaudeCode));
    }

    #[test]
    fn delete_with_no_requested_targets_still_cleans_every_previous_target() {
        let union = merged_targets(&[], &[TargetKind::Pi, TargetKind::Prime]);
        assert_eq!(union, vec![TargetKind::Pi, TargetKind::Prime]);
    }

    #[test]
    fn merged_targets_does_not_duplicate_requested_targets() {
        let union = merged_targets(
            &[TargetKind::ClaudeCode, TargetKind::Prime],
            &[TargetKind::Prime, TargetKind::Codex],
        );
        assert_eq!(union.len(), 3);
        assert!(union.contains(&TargetKind::ClaudeCode));
        assert!(union.contains(&TargetKind::Prime));
        assert!(union.contains(&TargetKind::Codex));
    }

    #[test]
    fn failed_targets_stay_eligible_for_the_next_cleanup() {
        // Claude 写成功且仍有启用的服务指向它，Codex 写失败：两个都保留在记录里，
        // 失败的那个下次仍会被重试，而不是被静默遗忘。
        let desired = vec![TargetKind::ClaudeCode, TargetKind::Codex];
        let recorded = targets_to_record(
            &[
                result(TargetKind::ClaudeCode, true),
                result(TargetKind::Codex, false),
            ],
            &desired,
        );
        assert_eq!(recorded.len(), 2);
        assert!(recorded.contains(&TargetKind::Codex));
    }

    #[test]
    fn successful_apply_records_the_new_target_set() {
        let recorded = targets_to_record(
            &[result(TargetKind::Codex, true)],
            &[TargetKind::Codex],
        );
        assert_eq!(recorded, vec![TargetKind::Codex]);
    }

    #[test]
    fn delete_cleanup_reports_every_swept_target() {
        // 删除后没有启用的服务指向任何目标，清理成功后记录应为空；
        // 但结果里仍要逐目标出现，界面才能说明哪些客户端被清理过。
        let previous = [TargetKind::Pi, TargetKind::Prime];
        let union = merged_targets(&[], &previous);
        let results: Vec<McpApplyTargetResult> = union
            .iter()
            .map(|target| result(*target, true))
            .collect();
        let recorded = targets_to_record(&results, &[]);
        assert!(recorded.is_empty(), "cleaned targets must not stay on the record");
        assert_eq!(results.len(), 2, "each swept target is reported");
    }

    #[test]
    fn a_target_that_stops_being_requested_is_dropped_from_the_record() {
        // Claude 曾经应用过，本次只写 Codex：Claude 仅用于清理，
        // 清理成功后不应继续留在记录里，否则每次保存都会重写它那份配置。
        let union = merged_targets(&[TargetKind::Codex], &[TargetKind::ClaudeCode]);
        let results = vec![
            result(TargetKind::ClaudeCode, true),
            result(TargetKind::Codex, true),
        ];
        let recorded = targets_to_record(&results, &[TargetKind::Codex]);
        assert_eq!(recorded, vec![TargetKind::Codex]);
    }

    #[test]
    fn enabled_servers_decide_which_targets_keep_managed_entries() {
        // 只有启用且有目标指向的服务才让目标留在记录里；
        // 禁用服务或空目标列表都不应阻止清理。
        let enabled = McpServer {
            id: "1".into(),
            name: "a".into(),
            kind: crate::domain::McpKind::Stdio,
            enabled: true,
            targets: vec![TargetKind::Pi],
            config: serde_json::json!({}),
            env: serde_json::json!({}),
            headers: serde_json::json!({}),
            created_at: 0,
            updated_at: 0,
        };
        let mut disabled = enabled.clone();
        disabled.enabled = false;
        disabled.targets = vec![TargetKind::Prime];
        let mut no_targets = enabled.clone();
        no_targets.targets = vec![];

        assert_eq!(
            targets_with_enabled_servers(&[enabled, disabled, no_targets]),
            vec![TargetKind::Pi]
        );
        assert!(targets_with_enabled_servers(&[]).is_empty());
    }
}
