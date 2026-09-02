use crate::domain::{FetchModelsResult, ModelProbeResult, ProbeSiteApiKeyResult, SiteModelDto};
use crate::error::{AppError, AppResult};
use crate::redact;
use crate::repo;
use crate::state::AppState;
use tauri::State;

fn resolve_active_key(
    conn: &rusqlite::Connection,
    crypto: &crate::crypto::Crypto,
    site_id: &str,
    api_key_id: Option<&str>,
) -> AppResult<(crate::domain::SiteRow, crate::domain::SiteApiKeyRow, String)> {
    let site = repo::site::get_site(conn, site_id)?;
    let key = match api_key_id {
        Some(id) => repo::site_api_key::require_active(conn, site_id, id)?,
        None => repo::site_api_key::get_active(conn, site_id)?,
    };
    let secret = repo::site_api_key::decrypt(crypto, &key)?;
    Ok((site, key, secret))
}

#[tauri::command]
pub async fn fetch_site_models(
    state: State<'_, AppState>,
    site_id: String,
    api_key_id: Option<String>,
) -> AppResult<FetchModelsResult> {
    let (site, captured_key_id, api_key, settings) = state.db.with_conn(|c| {
        let (site, key, secret) =
            resolve_active_key(c, &state.crypto, &site_id, api_key_id.as_deref())?;
        let settings = repo::settings::get_settings(c)?;
        Ok((site, key.id, secret, settings))
    })?;

    let result = match crate::models_fetch::fetch_models(&site, &api_key, &settings).await {
        Ok(mut r) => {
            for model in &mut r.models {
                model.api_key_id = captured_key_id.clone();
                model.site_id = site_id.clone();
            }
            state.db.with_conn(|c| {
                repo::site::replace_models(c, &site_id, &captured_key_id, &r.models)?;
                repo::site::update_fetch_meta_for_key(
                    c,
                    &captured_key_id,
                    r.latency_ms as i64,
                    None,
                )?;
                let models = repo::site::list_models_for_key(c, &captured_key_id)?;
                Ok(FetchModelsResult {
                    models,
                    latency_ms: r.latency_ms,
                    endpoint: r.endpoint,
                    fetched_at: r.fetched_at,
                    api_key_id: captured_key_id.clone(),
                })
            })?
        }
        Err(e) => {
            let msg = redact::api_key(&e.to_string());
            let _ = state.db.with_conn(|c| {
                repo::site::update_fetch_meta_for_key(c, &captured_key_id, 0, Some(&msg))
            });
            return Err(e);
        }
    };
    Ok(result)
}

#[tauri::command]
pub fn list_site_models(
    state: State<'_, AppState>,
    site_id: String,
    api_key_id: Option<String>,
) -> AppResult<Vec<SiteModelDto>> {
    state.db.with_conn(|c| match api_key_id.as_deref() {
        Some(id) => {
            repo::site_api_key::require_active(c, &site_id, id)?;
            repo::site::list_models_for_key(c, id)
        }
        None => repo::site::list_models(c, &site_id),
    })
}

#[tauri::command]
pub fn clear_site_models(
    state: State<'_, AppState>,
    site_id: String,
) -> AppResult<crate::domain::SiteDto> {
    state.db.with_conn(|c| {
        repo::site::clear_models(c, &site_id)?;
        Ok(repo::site::get_site(c, &site_id)?.to_dto())
    })
}

#[tauri::command]
pub fn delete_site_model(
    state: State<'_, AppState>,
    site_id: String,
    model_id: String,
) -> AppResult<crate::domain::SiteDto> {
    state.db.with_conn(|c| {
        repo::site::delete_model(c, &site_id, &model_id)?;
        Ok(repo::site::get_site(c, &site_id)?.to_dto())
    })
}

#[tauri::command]
pub async fn probe_site_api_key(
    state: State<'_, AppState>,
    site_id: String,
    api_key: String,
) -> AppResult<ProbeSiteApiKeyResult> {
    let api_key = api_key.trim().to_string();
    if api_key.is_empty() {
        return Err(AppError::new("validation_failed", "API key is required"));
    }
    let (site, settings) = state.db.with_conn(|c| {
        let site = repo::site::get_site(c, &site_id)?;
        let settings = repo::settings::get_settings(c)?;
        Ok((site, settings))
    })?;
    let result = crate::models_fetch::fetch_models(&site, &api_key, &settings).await?;
    Ok(ProbeSiteApiKeyResult {
        model_count: result.models.len(),
        latency_ms: result.latency_ms,
        endpoint: result.endpoint,
    })
}

#[tauri::command]
pub async fn probe_site_model(
    state: State<'_, AppState>,
    site_id: String,
    model_id: String,
) -> AppResult<ModelProbeResult> {
    let model_id = model_id.trim().to_string();
    if model_id.is_empty() {
        return Err(AppError::new("validation_failed", "model id required"));
    }
    let (site, api_key, settings) = state.db.with_conn(|c| {
        let (site, _, secret) = resolve_active_key(c, &state.crypto, &site_id, None)?;
        let settings = repo::settings::get_settings(c)?;
        Ok((site, secret, settings))
    })?;
    if api_key.trim().is_empty() {
        return Err(AppError::new("validation_failed", "api key required"));
    }
    crate::model_probe::probe_model(&site, &api_key, &model_id, &settings).await
}
