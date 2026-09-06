use crate::domain::{
    is_thinking_target, normalize_preset, validate_preset, SiteThinkingPreset, TargetKind,
};
use crate::error::{AppError, AppResult};
use crate::repo;
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub fn get_site_thinking_preset(
    state: State<'_, AppState>,
    site_id: String,
    target: TargetKind,
) -> AppResult<SiteThinkingPreset> {
    if !is_thinking_target(target) {
        return Err(AppError::new(
            "validation_failed",
            "thinking presets are only supported for Pi and Prime",
        ));
    }
    state.db.with_conn(|conn| {
        repo::site::get_site(conn, &site_id)?;
        repo::thinking::get_preset(conn, &site_id, target)
    })
}

#[tauri::command]
pub fn save_site_thinking_preset(
    state: State<'_, AppState>,
    preset: SiteThinkingPreset,
) -> AppResult<SiteThinkingPreset> {
    if !is_thinking_target(preset.target) {
        return Err(AppError::new(
            "validation_failed",
            "thinking presets are only supported for Pi and Prime",
        ));
    }
    let preset = normalize_preset(preset);
    state.db.with_conn(|conn| {
        let site = repo::site::get_site(conn, &preset.site_id)?;
        validate_preset(&preset, site.protocol, None)?;
        repo::thinking::save_preset(conn, &preset)?;
        Ok(preset)
    })
}
