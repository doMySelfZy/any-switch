use crate::adapters::claude_code::{has_1m_suffix, strip_1m_suffix};
use crate::capabilities::{
    capability_on, CODEX_COMPACT, CODEX_IMAGEGEN, CODEX_SEARCH, CODEX_VISION,
};
use crate::domain::{
    ApplyStatus, ApplyTargetResult, BindingApiKeySnapshot, CapabilitySource, ClaudeApplyOptions,
    ClaudeAuthKeyStyle, ClaudeEffortLevel, CodexApplyOptions, CodexReasoningEffort,
    ModelFetchOutcome, PiApplyOptions, PrimeApplyOptions, SiteModelDto, SiteRow,
    SwitchSiteApiKeyResult, TargetBinding, TargetKind,
};
use crate::error::AppResult;
use crate::lock::{try_lock_site, try_lock_target};
use crate::paths::backups_dir;
use crate::redact;
use crate::repo;
use crate::state::AppState;
use chrono::Utc;
use std::fs;

pub async fn switch_site_api_key(
    state: &AppState,
    site_id: &str,
    api_key_id: &str,
    sync_targets: bool,
) -> AppResult<SwitchSiteApiKeyResult> {
    let (site, captured_key_id, api_key, settings, cached_models) = {
        let _site_lock = try_lock_site(site_id)?;
        state.db.with_conn(|c| {
            repo::site_api_key::get_for_site(c, site_id, api_key_id)?;
            let activated = repo::site_api_key::activate(c, site_id, api_key_id)?;
            let site = repo::site::get_site(c, site_id)?;
            let key = repo::site_api_key::decrypt(&state.crypto, &activated)?;
            let settings = repo::settings::get_settings(c)?;
            let models = repo::site::list_models_for_key(c, &activated.id)?;
            Ok((site, activated.id, key, settings, models))
        })?
    };

    let fetch = match crate::models_fetch::fetch_models(&site, &api_key, &settings).await {
        Ok(mut result) => {
            for model in &mut result.models {
                model.api_key_id = captured_key_id.clone();
                model.site_id = site_id.to_string();
            }
            let models = state.db.with_conn(|c| {
                let still_active = repo::site_api_key::get_active(c, site_id)?;
                if still_active.id != captured_key_id {
                    return Ok(repo::site::list_models_for_key(c, &still_active.id)?);
                }
                repo::site::replace_models(c, site_id, &captured_key_id, &result.models)?;
                repo::site::update_fetch_meta_for_key(
                    c,
                    &captured_key_id,
                    result.latency_ms as i64,
                    None,
                )?;
                repo::site::list_models_for_key(c, &captured_key_id)
            })?;
            (
                true,
                ModelFetchOutcome {
                    ok: true,
                    api_key_id: captured_key_id.clone(),
                    latency_ms: result.latency_ms,
                    endpoint: Some(result.endpoint),
                    fetched_at: Some(result.fetched_at),
                    error: None,
                },
                models,
            )
        }
        Err(e) => {
            let msg = redact::api_key(&e.to_string());
            let _ = state.db.with_conn(|c| {
                repo::site::update_fetch_meta_for_key(c, &captured_key_id, 0, Some(&msg))
            });
            (
                false,
                ModelFetchOutcome {
                    ok: false,
                    api_key_id: captured_key_id.clone(),
                    latency_ms: 0,
                    endpoint: None,
                    fetched_at: None,
                    error: Some(msg),
                },
                cached_models,
            )
        }
    };

    let (fetch_ok, fetch_outcome, models) = fetch;
    let results = if sync_targets && fetch_ok {
        let _site_lock = try_lock_site(site_id)?;
        let site = state.db.with_conn(|c| {
            let active = repo::site_api_key::get_active(c, site_id)?;
            if active.id != captured_key_id {
                return Err(crate::error::AppError::new(
                    "validation_failed",
                    "api key is not the site's current key",
                ));
            }
            repo::site::get_site(c, site_id)
        })?;
        sync_applied_keys(state, &site, &models)?
    } else {
        Vec::new()
    };

    let site = state.db.with_conn(|c| repo::site::get_site(c, site_id))?;
    Ok(SwitchSiteApiKeyResult {
        site: site.to_dto(),
        models,
        fetch: fetch_outcome,
        results,
    })
}

fn model_ids(models: &[SiteModelDto]) -> std::collections::HashSet<String> {
    models.iter().map(|m| m.model_id.clone()).collect()
}

fn supported(ids: &std::collections::HashSet<String>, model_id: &str) -> bool {
    let trimmed = model_id.trim();
    ids.contains(trimmed) || ids.contains(&strip_1m_suffix(trimmed))
}

fn skip_result(target: TargetKind, message: &str) -> ApplyTargetResult {
    ApplyTargetResult {
        target,
        ok: false,
        status: ApplyStatus::Stale,
        backup_paths: vec![],
        message: message.into(),
        live_summary: None,
        touched_keys: None,
    }
}

pub fn sync_applied_keys(
    state: &AppState,
    site: &SiteRow,
    models: &[SiteModelDto],
) -> AppResult<Vec<ApplyTargetResult>> {
    let settings = state.db.with_conn(repo::settings::get_settings)?;
    let bindings = state
        .db
        .with_conn(|c| repo::binding::list_bindings_for_site(c, &site.id))?;
    let api_key = state.crypto.decrypt(&site.api_key_encrypted)?;
    let snapshot = site.api_key_snapshot();
    let ids = model_ids(models);
    let applied_at = Utc::now().timestamp_millis();
    let mut results = Vec::new();

    for binding in bindings {
        if binding.orphan {
            continue;
        }
        let target = binding.target;
        if let Some(skipped) = skip_if_incompatible(target, &binding, &ids) {
            results.push(skipped);
            continue;
        }
        let _lock = try_lock_target(target.as_str())?;
        let backup_root = backups_dir()?
            .join(target.as_str())
            .join(format!("{}", applied_at));
        fs::create_dir_all(&backup_root)?;

        let rewrite = match target {
            TargetKind::ClaudeCode => {
                apply_claude(site, &api_key, &binding, &settings, &backup_root)
            }
            TargetKind::Codex => {
                apply_codex(site, &api_key, &binding, models, &settings, &backup_root)
            }
            TargetKind::Pi => apply_pi(site, &api_key, &binding, models, &settings, &backup_root),
            TargetKind::Prime => {
                apply_prime(site, &api_key, &binding, models, &settings, &backup_root)
            }
        };

        match rewrite {
            Ok((mut next, backup_paths, message, live_summary, touched)) => {
                next.api_key = snapshot.clone();
                next.applied_at = applied_at;
                state
                    .db
                    .with_conn(|c| repo::binding::upsert_binding(c, &next))?;
                results.push(ApplyTargetResult {
                    target,
                    ok: true,
                    status: ApplyStatus::Applied,
                    backup_paths,
                    message,
                    live_summary: Some(live_summary),
                    touched_keys: Some(touched),
                });
            }
            Err(e) => {
                results.push(ApplyTargetResult {
                    target,
                    ok: false,
                    status: ApplyStatus::Failed,
                    backup_paths: vec![],
                    message: e.to_string(),
                    live_summary: None,
                    touched_keys: None,
                });
            }
        }

        crate::commands::apply::finalize_backup_dir(
            &backup_root,
            target,
            &site.name,
            &binding.model_id,
            None,
            applied_at,
            settings.max_backup_copies,
        );
    }

    Ok(results)
}

fn skip_if_incompatible(
    target: TargetKind,
    binding: &TargetBinding,
    ids: &std::collections::HashSet<String>,
) -> Option<ApplyTargetResult> {
    if !supported(ids, &binding.model_id) {
        return Some(skip_result(
            target,
            "current model is not available on the new API key",
        ));
    }
    if target == TargetKind::ClaudeCode {
        for key in [
            "ANTHROPIC_DEFAULT_OPUS_MODEL",
            "ANTHROPIC_DEFAULT_SONNET_MODEL",
            "ANTHROPIC_DEFAULT_HAIKU_MODEL",
        ] {
            if let Some(model) = binding.expected_fields.get(key) {
                if !supported(ids, model) {
                    return Some(skip_result(
                        target,
                        "Claude model alias is not available on the new API key",
                    ));
                }
            }
        }
    }
    None
}

fn apply_claude(
    site: &SiteRow,
    api_key: &str,
    binding: &TargetBinding,
    settings: &crate::domain::AppSettings,
    backup_root: &std::path::PathBuf,
) -> AppResult<(
    TargetBinding,
    Vec<String>,
    String,
    std::collections::HashMap<String, Option<String>>,
    Vec<String>,
)> {
    let auth = match binding
        .expected_fields
        .get("auth_env_key")
        .map(|s| s.as_str())
    {
        Some("ANTHROPIC_API_KEY") => ClaudeAuthKeyStyle::AnthropicApiKey,
        Some("ANTHROPIC_AUTH_TOKEN") => ClaudeAuthKeyStyle::AnthropicAuthToken,
        _ => site.claude_auth_key_style.clone(),
    };
    let options = ClaudeApplyOptions {
        opus_model_id: binding
            .expected_fields
            .get("ANTHROPIC_DEFAULT_OPUS_MODEL")
            .cloned(),
        sonnet_model_id: binding
            .expected_fields
            .get("ANTHROPIC_DEFAULT_SONNET_MODEL")
            .cloned(),
        haiku_model_id: binding
            .expected_fields
            .get("ANTHROPIC_DEFAULT_HAIKU_MODEL")
            .cloned(),
        effort_level: binding
            .expected_fields
            .get("CLAUDE_CODE_EFFORT_LEVEL")
            .and_then(|s| ClaudeEffortLevel::parse(s)),
        use_1m_context: has_1m_suffix(&binding.model_id),
    };
    let outcome = crate::adapters::claude_code::apply(
        site,
        api_key,
        &binding.model_id,
        auth,
        settings.force_exclusive_claude_auth_key,
        &options,
        Some(binding),
        settings.claude_home_override.as_deref(),
        backup_root,
    )?;
    Ok((
        outcome.binding,
        outcome.backup_paths,
        outcome.message,
        outcome.live_summary,
        outcome.touched.paths,
    ))
}

fn apply_codex(
    site: &SiteRow,
    api_key: &str,
    binding: &TargetBinding,
    models: &[SiteModelDto],
    settings: &crate::domain::AppSettings,
    backup_root: &std::path::PathBuf,
) -> AppResult<(
    TargetBinding,
    Vec<String>,
    String,
    std::collections::HashMap<String, Option<String>>,
    Vec<String>,
)> {
    let write_all = binding.expected_fields.contains_key("model_catalog_json");
    let catalog_models = if write_all {
        models
            .iter()
            .map(|m| (m.model_id.clone(), m.display_name.clone()))
            .collect()
    } else {
        Vec::new()
    };
    let options = CodexApplyOptions {
        write_all_models: write_all,
        reasoning_effort: binding
            .expected_fields
            .get("model_reasoning_effort")
            .and_then(|s| CodexReasoningEffort::parse(s)),
        catalog_models,
        remote_compaction: binding
            .expected_fields
            .get("remote_compaction")
            .map(|s| s == "enabled")
            .unwrap_or_else(|| capability_on(&site.capabilities, CODEX_COMPACT)),
        image_understanding: binding
            .expected_fields
            .get("image_understanding")
            .map(|s| s == "enabled")
            .unwrap_or_else(|| capability_on(&site.capabilities, CODEX_VISION)),
        image_generation: binding
            .expected_fields
            .get("image_generation")
            .map(|s| s == "enabled")
            .unwrap_or_else(|| capability_on(&site.capabilities, CODEX_IMAGEGEN)),
        web_search: binding
            .expected_fields
            .get("web_search")
            .map(|s| s == "enabled")
            .unwrap_or_else(|| capability_on(&site.capabilities, CODEX_SEARCH)),
        capability_source: if binding.expected_fields.contains_key("web_search") {
            CapabilitySource::Custom
        } else {
            CapabilitySource::Site
        },
    };
    let outcome = crate::adapters::codex::apply(
        site,
        api_key,
        &binding.model_id,
        &options,
        settings.codex_home_override.as_deref(),
        backup_root,
    )?;
    let inject_msg = crate::env_inject::inject_codex_env(settings, &outcome.env_key, api_key)
        .unwrap_or_else(|e| e.to_string());
    Ok((
        outcome.binding,
        outcome.backup_paths,
        format!("{} {}", outcome.message, inject_msg),
        outcome.live_summary,
        outcome.touched.paths,
    ))
}

fn apply_pi(
    site: &SiteRow,
    api_key: &str,
    binding: &TargetBinding,
    models: &[SiteModelDto],
    settings: &crate::domain::AppSettings,
    backup_root: &std::path::Path,
) -> AppResult<(
    TargetBinding,
    Vec<String>,
    String,
    std::collections::HashMap<String, Option<String>>,
    Vec<String>,
)> {
    let write_all = binding
        .expected_fields
        .get("write_all_models")
        .map(|s| s == "true" || s == "1")
        .unwrap_or(false);
    let catalog_models = if write_all {
        models
            .iter()
            .map(|m| (m.model_id.clone(), m.display_name.clone()))
            .collect()
    } else {
        Vec::new()
    };
    let options = PiApplyOptions {
        write_all_models: write_all,
        catalog_models,
    };
    let outcome = crate::adapters::pi::apply(
        site,
        api_key,
        &binding.model_id,
        &options,
        Some(binding),
        settings.pi_agent_dir_override.as_deref(),
        backup_root,
    )?;
    Ok((
        outcome.binding,
        outcome.backup_paths,
        outcome.message,
        outcome.live_summary,
        outcome.touched.paths,
    ))
}

fn apply_prime(
    site: &SiteRow,
    api_key: &str,
    binding: &TargetBinding,
    models: &[SiteModelDto],
    settings: &crate::domain::AppSettings,
    backup_root: &std::path::Path,
) -> AppResult<(
    TargetBinding,
    Vec<String>,
    String,
    std::collections::HashMap<String, Option<String>>,
    Vec<String>,
)> {
    let write_all = binding
        .expected_fields
        .get("write_all_models")
        .map(|s| s == "true" || s == "1")
        .unwrap_or(false);
    let catalog_models = if write_all {
        models
            .iter()
            .map(|m| (m.model_id.clone(), m.display_name.clone()))
            .collect()
    } else {
        Vec::new()
    };
    let options = PrimeApplyOptions {
        write_all_models: write_all,
        catalog_models,
    };
    let outcome = crate::adapters::prime::apply(
        site,
        api_key,
        &binding.model_id,
        &options,
        Some(binding),
        settings.prime_agent_dir_override.as_deref(),
        backup_root,
    )?;
    Ok((
        outcome.binding,
        outcome.backup_paths,
        outcome.message,
        outcome.live_summary,
        outcome.touched.paths,
    ))
}

pub fn stamp_binding(binding: &mut TargetBinding, snapshot: &BindingApiKeySnapshot) {
    binding.api_key = snapshot.clone();
}
