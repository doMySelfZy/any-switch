use crate::domain::{
    CreateSiteInput, DeepLinkSiteImportInput, DeepLinkSiteImportResult, SiteDto, SwitchRouteResult,
    UpdateSiteInput,
};
use crate::error::AppResult;
use crate::repo;
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub fn list_sites(state: State<'_, AppState>) -> AppResult<Vec<SiteDto>> {
    let rows = state.db.with_conn(repo::site::list_sites)?;
    Ok(rows.into_iter().map(|r| r.to_dto()).collect())
}

#[tauri::command]
pub fn get_site(state: State<'_, AppState>, id: String) -> AppResult<SiteDto> {
    Ok(state
        .db
        .with_conn(|c| repo::site::get_site(c, &id))?
        .to_dto())
}

#[tauri::command]
pub fn get_site_api_key(state: State<'_, AppState>, id: String) -> AppResult<String> {
    state
        .db
        .with_conn(|c| repo::site::get_site_api_key(c, &state.crypto, &id))
}

#[tauri::command]
pub fn create_site(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    input: CreateSiteInput,
) -> AppResult<SiteDto> {
    let row = state
        .db
        .with_conn(|c| repo::site::create_site(c, &state.crypto, input))?;
    crate::tray::request_tray_menu_sync(&app);
    Ok(row.to_dto())
}

#[tauri::command]
pub fn import_site_from_deep_link(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    input: DeepLinkSiteImportInput,
) -> AppResult<DeepLinkSiteImportResult> {
    let result = crate::deep_link::import_site_from_deep_link(&state, input)?;
    crate::tray::request_tray_menu_sync(&app);
    Ok(result)
}

#[tauri::command]
pub fn update_site(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    id: String,
    input: UpdateSiteInput,
) -> AppResult<SiteDto> {
    let before = state.db.with_conn(|c| repo::site::get_site(c, &id))?;
    let row = state
        .db
        .with_conn(|c| repo::site::update_site(c, &state.crypto, &id, input))?;
    if before.base_url != row.base_url {
        let _ = crate::route_switch::sync_applied_urls(&state, &row);
    }
    crate::tray::request_tray_menu_sync(&app);
    Ok(row.to_dto())
}

#[tauri::command]
pub fn switch_site_route(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    site_id: String,
    base_url: String,
    apply: Option<bool>,
) -> AppResult<SwitchRouteResult> {
    let result =
        crate::route_switch::switch_site_route(&state, &site_id, &base_url, apply.unwrap_or(true))?;
    crate::tray::request_tray_menu_sync(&app);
    Ok(result)
}

#[tauri::command]
pub fn delete_site(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    id: String,
    cleanup_targets: Option<bool>,
) -> AppResult<()> {
    let cleanup = cleanup_targets.unwrap_or(false);
    let settings = state.db.with_conn(repo::settings::get_settings)?;

    if cleanup {
        let bindings = state
            .db
            .with_conn(|c| repo::binding::list_bindings_for_site(c, &id))?;
        for b in bindings {
            match b.target {
                crate::domain::TargetKind::ClaudeCode => {
                    crate::adapters::claude_code::surgical_revert(
                        &b,
                        settings.claude_home_override.as_deref(),
                    )?;
                }
                crate::domain::TargetKind::Codex => {
                    crate::adapters::codex::surgical_revert(
                        &b,
                        settings.codex_home_override.as_deref(),
                    )?;
                    if let Some(env_key) = b.managed_env_keys.first() {
                        crate::env_inject::remove_codex_env(&settings, env_key)?;
                    }
                }
                crate::domain::TargetKind::Pi => {
                    crate::adapters::pi::surgical_revert(
                        &b,
                        settings.pi_agent_dir_override.as_deref(),
                    )?;
                }
            }
            state
                .db
                .with_conn(|c| repo::binding::delete_binding(c, b.target))?;
        }
    } else {
        state
            .db
            .with_conn(|c| repo::binding::orphan_bindings_for_site(c, &id))?;
    }

    state.db.with_conn(|c| repo::site::delete_site(c, &id))?;
    crate::tray::request_tray_menu_sync(&app);
    Ok(())
}

#[tauri::command]
pub fn reorder_sites(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    ids: Vec<String>,
) -> AppResult<()> {
    state.db.with_conn(|c| {
        for (i, id) in ids.iter().enumerate() {
            c.execute(
                "UPDATE sites SET sort_order = ?2 WHERE id = ?1",
                rusqlite::params![id, i as i64],
            )?;
        }
        Ok(())
    })?;
    crate::tray::request_tray_menu_sync(&app);
    Ok(())
}

#[tauri::command]
pub fn set_selected_model(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    site_id: String,
    model_id: String,
) -> AppResult<()> {
    state
        .db
        .with_conn(|c| repo::site::set_selected_model(c, &site_id, &model_id))?;
    crate::tray::request_tray_menu_sync(&app);
    Ok(())
}
