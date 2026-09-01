use crate::domain::SiteQuota;
use crate::error::AppResult;
use crate::repo;
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub async fn probe_site_quota(state: State<'_, AppState>, site_id: String) -> AppResult<SiteQuota> {
    let (site, api_key, settings) = state.db.with_conn(|c| {
        let site = repo::site::get_site(c, &site_id)?;
        let key = repo::site_api_key::get_active(c, &site_id)?;
        let secret = repo::site_api_key::decrypt(&state.crypto, &key)?;
        let settings = repo::settings::get_settings(c)?;
        Ok((site, secret, settings))
    })?;
    if api_key.trim().is_empty() {
        return Ok(crate::quota_probe::empty_key_result());
    }
    crate::quota_probe::probe_quota(&site, &api_key, &settings).await
}
