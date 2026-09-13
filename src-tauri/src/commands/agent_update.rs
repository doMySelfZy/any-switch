use crate::adapters::agent_update::{
    check_npm_package_version, get_npm_package_name, update_npm_package,
};
use crate::domain::AgentUpdateStatus;
use crate::error::{AppError, AppResult};
use crate::repo::agent_update as repo_agent_update;
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchUpdateResult {
    pub successes: Vec<String>,
    pub failures: Vec<(String, String)>,
}

/// Check updates for all installed agents
#[tauri::command]
pub async fn check_agent_updates(state: State<'_, AppState>) -> AppResult<Vec<AgentUpdateStatus>> {
    let agent_kinds = vec!["claude_code", "codex", "pi", "prime"];
    let mut statuses = Vec::new();

    for kind in agent_kinds {
        let name = match kind {
            "claude_code" => "Claude Code",
            "codex" => "Codex",
            "pi" => "Pi",
            "prime" => "Prime",
            _ => kind,
        };

        let package_name = match get_npm_package_name(kind) {
            Some(name) => name,
            None => continue,
        };

        let (current_version, latest_version) =
            match check_npm_package_version(package_name).await {
                Ok((current, latest)) => (current, latest),
                Err(e) => {
                    eprintln!("Failed to check {} updates: {}", kind, e);
                    continue;
                }
            };

        let has_update = match (&current_version, &latest_version) {
            (Some(current), Some(latest)) => {
                // Compare versions using semver
                match (
                    semver::Version::parse(current),
                    semver::Version::parse(latest),
                ) {
                    (Ok(current_ver), Ok(latest_ver)) => latest_ver > current_ver,
                    _ => false,
                }
            }
            _ => false,
        };

        let status = AgentUpdateStatus {
            kind: kind.to_string(),
            name: name.to_string(),
            current_version,
            latest_version,
            has_update,
            last_check_at: Some(chrono::Utc::now().timestamp()),
        };

        // Save to database
        let _ = state.db.with_conn(|conn| {
            repo_agent_update::upsert_agent_update_status(conn, &status)
        });

        statuses.push(status);
    }

    Ok(statuses)
}

/// Update a specific agent
#[tauri::command]
pub async fn update_agent(kind: String, state: State<'_, AppState>) -> AppResult<String> {
    let package_name = get_npm_package_name(&kind)
        .ok_or_else(|| AppError::new("unknown_agent", format!("Unknown agent kind: {}", kind)))?;

    let new_version = update_npm_package(package_name)
        .await
        .map_err(|e| AppError::new("update_failed", e.to_string()))?;

    // Update database status
    let _ = state.db.with_conn(|conn| {
        if let Ok(Some(mut status)) = repo_agent_update::get_agent_update_status(conn, &kind) {
            status.current_version = Some(new_version.clone());
            status.has_update = false;
            status.last_check_at = Some(chrono::Utc::now().timestamp());
            let _ = repo_agent_update::upsert_agent_update_status(conn, &status);
        }
        Ok(())
    });

    Ok(new_version)
}

/// Batch update multiple agents
#[tauri::command]
pub async fn batch_update_agents(
    kinds: Vec<String>,
    state: State<'_, AppState>,
) -> AppResult<BatchUpdateResult> {
    let mut successes = Vec::new();
    let mut failures = Vec::new();

    for kind in kinds {
        match update_agent(kind.clone(), state.clone()).await {
            Ok(_) => successes.push(kind),
            Err(e) => failures.push((kind, e.to_string())),
        }
    }

    Ok(BatchUpdateResult {
        successes,
        failures,
    })
}

/// Get all agent update statuses from database
#[tauri::command]
pub async fn get_all_agent_update_statuses(
    state: State<'_, AppState>,
) -> AppResult<Vec<AgentUpdateStatus>> {
    state.db.with_conn(|conn| {
        repo_agent_update::get_all_agent_update_statuses(conn)
    })
}

/// Get single agent update status
#[tauri::command]
pub async fn get_agent_update_status(
    kind: String,
    state: State<'_, AppState>,
) -> AppResult<Option<AgentUpdateStatus>> {
    state.db.with_conn(|conn| {
        repo_agent_update::get_agent_update_status(conn, &kind)
    })
}

/// Clear/delete agent update status
#[tauri::command]
pub async fn clear_agent_update_status(
    kind: String,
    state: State<'_, AppState>,
) -> AppResult<()> {
    state.db.with_conn(|conn| {
        repo_agent_update::delete_agent_update_status(conn, &kind)
    })
}
