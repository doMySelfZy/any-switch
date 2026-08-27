use crate::backup::{self, BackupMeta, BackupMetaFile};
use crate::capabilities::{
    capability_on, CODEX_COMPACT, CODEX_IMAGEGEN, CODEX_SEARCH, CODEX_VISION,
};
use crate::domain::{
    ApplyRecordDto, ApplyResult, ApplyStatus, ApplyTargetResult, BackupInfo, BackupPreview,
    CapabilitySource, ClaudeApplyOptions, ClaudeAuthKeyStyle, ClaudeEffortLevel, CodexApplyOptions,
    CodexReasoningEffort, PiApplyOptions, TargetKind, TouchedKeys,
};
use crate::error::{AppError, AppResult};
use crate::lock::try_lock_target;
use crate::paths::backups_dir;
use crate::repo;
use crate::state::AppState;
use chrono::Utc;
use std::fs;
use std::path::Path;
use tauri::State;
use uuid::Uuid;

fn non_empty(s: Option<String>) -> Option<String> {
    s.map(|v| v.trim().to_string()).filter(|v| !v.is_empty())
}

#[tauri::command]
pub fn apply_site(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    site_id: String,
    targets: Vec<TargetKind>,
    model_id: String,
    claude_auth_key_style: Option<String>,
    claude_opus_model_id: Option<String>,
    claude_sonnet_model_id: Option<String>,
    claude_haiku_model_id: Option<String>,
    claude_effort_level: Option<String>,
    codex_write_all_models: Option<bool>,
    codex_reasoning_effort: Option<String>,
    codex_remote_compaction: Option<bool>,
    codex_image_understanding: Option<bool>,
    codex_image_generation: Option<bool>,
    codex_web_search: Option<bool>,
    codex_capability_source: Option<String>,
    pi_write_all_models: Option<bool>,
) -> AppResult<ApplyResult> {
    if targets.is_empty() {
        return Err(AppError::new("validation_failed", "no targets selected"));
    }
    if model_id.trim().is_empty() {
        return Err(AppError::new("validation_failed", "model id required"));
    }

    let settings = state.db.with_conn(repo::settings::get_settings)?;
    let (site, api_key) = state.db.with_conn(|c| {
        let site = repo::site::get_site(c, &site_id)?;
        let key = state.crypto.decrypt(&site.api_key_encrypted)?;
        Ok((site, key))
    })?;

    let auth = claude_auth_key_style
        .as_deref()
        .map(ClaudeAuthKeyStyle::parse)
        .unwrap_or(site.claude_auth_key_style.clone());

    let claude_opts = ClaudeApplyOptions {
        opus_model_id: non_empty(claude_opus_model_id),
        sonnet_model_id: non_empty(claude_sonnet_model_id),
        haiku_model_id: non_empty(claude_haiku_model_id),
        effort_level: claude_effort_level
            .as_deref()
            .and_then(ClaudeEffortLevel::parse),
    };

    let capability_source = CapabilitySource::parse(codex_capability_source.as_deref());
    let (remote_compaction, image_understanding, image_generation, web_search) =
        match capability_source {
            CapabilitySource::Site => (
                capability_on(&site.capabilities, CODEX_COMPACT),
                capability_on(&site.capabilities, CODEX_VISION),
                capability_on(&site.capabilities, CODEX_IMAGEGEN),
                capability_on(&site.capabilities, CODEX_SEARCH),
            ),
            CapabilitySource::Custom => (
                codex_remote_compaction.unwrap_or(false),
                codex_image_understanding.unwrap_or(false),
                codex_image_generation.unwrap_or(false),
                codex_web_search.unwrap_or(false),
            ),
        };

    let write_all = codex_write_all_models.unwrap_or(false);
    let catalog_models = if write_all && targets.contains(&TargetKind::Codex) {
        state.db.with_conn(|c| {
            let models = repo::site::list_models(c, &site_id)?;
            Ok(models
                .into_iter()
                .map(|m| (m.model_id, m.display_name))
                .collect::<Vec<_>>())
        })?
    } else {
        vec![]
    };

    let codex_opts = CodexApplyOptions {
        write_all_models: write_all,
        reasoning_effort: codex_reasoning_effort
            .as_deref()
            .and_then(CodexReasoningEffort::parse),
        catalog_models,
        remote_compaction,
        image_understanding,
        image_generation,
        web_search,
        capability_source,
    };

    let pi_write_all = pi_write_all_models.unwrap_or(false);
    let pi_catalog_models = if pi_write_all && targets.contains(&TargetKind::Pi) {
        state.db.with_conn(|c| {
            let models = repo::site::list_models(c, &site_id)?;
            Ok(models
                .into_iter()
                .map(|model| (model.model_id, model.display_name))
                .collect::<Vec<_>>())
        })?
    } else {
        Vec::new()
    };
    let pi_opts = PiApplyOptions {
        write_all_models: pi_write_all,
        catalog_models: pi_catalog_models,
    };

    let applied_at = Utc::now().timestamp_millis();
    let mut results = Vec::new();

    for target in targets {
        let _lock = try_lock_target(target.as_str())?;
        let backup_root = backups_dir()?
            .join(target.as_str())
            .join(format!("{}", applied_at));
        fs::create_dir_all(&backup_root)?;

        let binding_before = state
            .db
            .with_conn(|c| repo::binding::get_binding(c, target))?;

        if target == TargetKind::Pi {
            match crate::adapters::pi::apply(
                &site,
                &api_key,
                &model_id,
                &pi_opts,
                binding_before.as_ref(),
                settings.pi_agent_dir_override.as_deref(),
                &backup_root,
            ) {
                Ok(outcome) => {
                    let record_id = outcome
                        .binding
                        .apply_record_id
                        .clone()
                        .unwrap_or_else(|| Uuid::new_v4().to_string());
                    let mut binding = outcome.binding.clone();
                    binding.apply_record_id = Some(record_id.clone());
                    state
                        .db
                        .with_conn(|c| repo::binding::upsert_binding(c, &binding))?;
                    state.db.with_conn(|c| {
                        repo::apply::insert_record(
                            c,
                            &record_id,
                            Some(&site.id),
                            &site.name,
                            target.as_str(),
                            &model_id,
                            Some(&outcome.provider_id),
                            "success",
                            Some(&backup_root.display().to_string()),
                            &outcome.touched,
                            None,
                            applied_at,
                        )
                    })?;
                    results.push(ApplyTargetResult {
                        target,
                        ok: true,
                        status: ApplyStatus::Applied,
                        backup_paths: outcome.backup_paths,
                        message: outcome.message,
                        live_summary: Some(outcome.live_summary),
                        touched_keys: Some(outcome.touched.paths),
                    });
                }
                Err(error) => {
                    let touched = TouchedKeys::default();
                    let _ = state.db.with_conn(|c| {
                        repo::apply::insert_record(
                            c,
                            &Uuid::new_v4().to_string(),
                            Some(&site.id),
                            &site.name,
                            target.as_str(),
                            &model_id,
                            None,
                            "failed",
                            Some(&backup_root.display().to_string()),
                            &touched,
                            Some(&error.to_string()),
                            applied_at,
                        )
                    });
                    results.push(ApplyTargetResult {
                        target,
                        ok: false,
                        status: ApplyStatus::Failed,
                        backup_paths: Vec::new(),
                        message: error.to_string(),
                        live_summary: None,
                        touched_keys: None,
                    });
                }
            }
            finalize_backup_dir(
                &backup_root,
                target,
                &site.name,
                &model_id,
                None,
                applied_at,
                settings.max_backup_copies,
            );
            continue;
        }

        if target == TargetKind::Codex {
            match crate::adapters::codex::apply(
                &site,
                &api_key,
                &model_id,
                &codex_opts,
                settings.codex_home_override.as_deref(),
                &backup_root,
            ) {
                Ok(o) => {
                    let inject_msg =
                        crate::env_inject::inject_codex_env(&settings, &o.env_key, &api_key)
                            .unwrap_or_else(|e| e.to_string());
                    let record_id = o
                        .binding
                        .apply_record_id
                        .clone()
                        .unwrap_or_else(|| Uuid::new_v4().to_string());
                    let mut binding = o.binding.clone();
                    binding.apply_record_id = Some(record_id.clone());
                    state
                        .db
                        .with_conn(|c| repo::binding::upsert_binding(c, &binding))?;
                    state.db.with_conn(|c| {
                        repo::apply::insert_record(
                            c,
                            &record_id,
                            Some(&site.id),
                            &site.name,
                            target.as_str(),
                            &model_id,
                            Some(&o.provider_id),
                            "success",
                            Some(&backup_root.display().to_string()),
                            &o.touched,
                            None,
                            applied_at,
                        )
                    })?;
                    results.push(ApplyTargetResult {
                        target,
                        ok: true,
                        status: ApplyStatus::Applied,
                        backup_paths: o.backup_paths,
                        message: format!("{} {}", o.message, inject_msg),
                        live_summary: Some(o.live_summary),
                        touched_keys: Some(o.touched.env_keys),
                    });
                }
                Err(e) => {
                    let touched = TouchedKeys::default();
                    let _ = state.db.with_conn(|c| {
                        repo::apply::insert_record(
                            c,
                            &Uuid::new_v4().to_string(),
                            Some(&site.id),
                            &site.name,
                            target.as_str(),
                            &model_id,
                            None,
                            "failed",
                            Some(&backup_root.display().to_string()),
                            &touched,
                            Some(&e.to_string()),
                            applied_at,
                        )
                    });
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
            finalize_backup_dir(
                &backup_root,
                target,
                &site.name,
                &model_id,
                None,
                applied_at,
                settings.max_backup_copies,
            );
            continue;
        }

        // Claude Code
        match crate::adapters::claude_code::apply(
            &site,
            &api_key,
            &model_id,
            auth.clone(),
            settings.force_exclusive_claude_auth_key,
            &claude_opts,
            binding_before.as_ref(),
            settings.claude_home_override.as_deref(),
            &backup_root,
        ) {
            Ok(o) => {
                let record_id = o
                    .binding
                    .apply_record_id
                    .clone()
                    .unwrap_or_else(|| Uuid::new_v4().to_string());
                let mut binding = o.binding.clone();
                binding.apply_record_id = Some(record_id.clone());
                state
                    .db
                    .with_conn(|c| repo::binding::upsert_binding(c, &binding))?;
                state.db.with_conn(|c| {
                    repo::apply::insert_record(
                        c,
                        &record_id,
                        Some(&site.id),
                        &site.name,
                        target.as_str(),
                        &model_id,
                        None,
                        "success",
                        Some(&backup_root.display().to_string()),
                        &o.touched,
                        None,
                        applied_at,
                    )
                })?;
                results.push(ApplyTargetResult {
                    target,
                    ok: true,
                    status: ApplyStatus::Applied,
                    backup_paths: o.backup_paths,
                    message: o.message,
                    live_summary: Some(o.live_summary),
                    touched_keys: Some(o.touched.claude_env_keys),
                });
            }
            Err(e) => {
                let touched = TouchedKeys::default();
                let _ = state.db.with_conn(|c| {
                    repo::apply::insert_record(
                        c,
                        &Uuid::new_v4().to_string(),
                        Some(&site.id),
                        &site.name,
                        target.as_str(),
                        &model_id,
                        None,
                        "failed",
                        Some(&backup_root.display().to_string()),
                        &touched,
                        Some(&e.to_string()),
                        applied_at,
                    )
                });
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
        finalize_backup_dir(
            &backup_root,
            target,
            &site.name,
            &model_id,
            None,
            applied_at,
            settings.max_backup_copies,
        );
    }

    let result = ApplyResult {
        site_id,
        model_id,
        results,
        applied_at,
    };
    crate::tray::request_tray_menu_sync(&app);
    Ok(result)
}

pub(crate) fn finalize_backup_dir(
    backup_root: &Path,
    target: TargetKind,
    site_name: &str,
    model_id: &str,
    apply_record_id: Option<&str>,
    created_at: i64,
    max_copies: u32,
) {
    let files = backup::payload_files(backup_root);
    if files.is_empty() {
        let _ = fs::remove_dir_all(backup_root);
    } else {
        let origins = crate::adapters::atomic::read_origins(backup_root);
        let prev = backup::read_meta(backup_root);
        let prev_map: std::collections::HashMap<String, Option<String>> = prev
            .map(|m| {
                m.files
                    .into_iter()
                    .map(|f| (f.name, f.original_path))
                    .collect()
            })
            .unwrap_or_default();
        let meta = BackupMeta {
            version: 1,
            target: target.as_str().into(),
            created_at,
            site_name: Some(site_name.into()),
            model_id: Some(model_id.into()),
            apply_record_id: apply_record_id.map(|s| s.to_string()),
            files: files
                .into_iter()
                .map(|name| BackupMetaFile {
                    original_path: origins
                        .get(&name)
                        .cloned()
                        .or_else(|| prev_map.get(&name).cloned().flatten()),
                    name,
                })
                .collect(),
        };
        let _ = backup::write_meta(backup_root, &meta);
    }
    let _ = backup::prune_target_backups(target, max_copies);
}

#[tauri::command]
pub fn revert_target(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    target: TargetKind,
) -> AppResult<()> {
    let _lock = try_lock_target(target.as_str())?;
    let settings = state.db.with_conn(repo::settings::get_settings)?;
    let binding = state
        .db
        .with_conn(|c| repo::binding::get_binding(c, target))?
        .ok_or_else(|| AppError::new("not_found", "no binding to revert"))?;

    match target {
        TargetKind::ClaudeCode => {
            crate::adapters::claude_code::surgical_revert(
                &binding,
                settings.claude_home_override.as_deref(),
            )?;
        }
        TargetKind::Codex => {
            crate::adapters::codex::surgical_revert(
                &binding,
                settings.codex_home_override.as_deref(),
            )?;
            if let Some(env_key) = binding.managed_env_keys.first() {
                let _ = crate::env_inject::remove_codex_env(&settings, env_key);
            }
        }
        TargetKind::Pi => {
            crate::adapters::pi::surgical_revert(
                &binding,
                settings.pi_agent_dir_override.as_deref(),
            )?;
        }
    }
    state
        .db
        .with_conn(|c| repo::binding::delete_binding(c, target))?;
    crate::tray::request_tray_menu_sync(&app);
    Ok(())
}

#[tauri::command]
pub fn restore_official_target(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    target: TargetKind,
) -> AppResult<()> {
    let _lock = try_lock_target(target.as_str())?;
    let settings = state.db.with_conn(repo::settings::get_settings)?;
    let binding = state
        .db
        .with_conn(|c| repo::binding::get_binding(c, target))?;

    let applied_at = Utc::now().timestamp_millis();
    let backup_root = backups_dir()?
        .join(target.as_str())
        .join(format!("{}", applied_at));
    fs::create_dir_all(&backup_root)?;

    let (site_name, model_id) = match &binding {
        Some(b) => (b.site_name_snapshot.as_str(), b.model_id.as_str()),
        None => ("official", "official"),
    };

    let result = match target {
        TargetKind::ClaudeCode => crate::adapters::claude_code::restore_official(
            settings.claude_home_override.as_deref(),
            &backup_root,
        )
        .map(|_| ()),
        TargetKind::Codex => crate::adapters::codex::restore_official(
            binding.as_ref(),
            settings.codex_home_override.as_deref(),
            &backup_root,
        )
        .map(|outcome| {
            for key in &outcome.env_keys {
                let _ = crate::env_inject::remove_codex_env(&settings, key);
            }
        }),
        TargetKind::Pi => crate::adapters::pi::restore_official(
            binding.as_ref(),
            settings.pi_agent_dir_override.as_deref(),
            &backup_root,
        )
        .map(|_| ()),
    };

    match result {
        Ok(()) => {
            if binding.is_some() {
                state
                    .db
                    .with_conn(|c| repo::binding::delete_binding(c, target))?;
            }
            finalize_backup_dir(
                &backup_root,
                target,
                site_name,
                model_id,
                None,
                applied_at,
                settings.max_backup_copies,
            );
            crate::tray::request_tray_menu_sync(&app);
            Ok(())
        }
        Err(e) => {
            finalize_backup_dir(
                &backup_root,
                target,
                site_name,
                model_id,
                None,
                applied_at,
                settings.max_backup_copies,
            );
            Err(e)
        }
    }
}

#[tauri::command]
pub fn list_apply_records(
    state: State<'_, AppState>,
    limit: Option<i64>,
) -> AppResult<Vec<ApplyRecordDto>> {
    state
        .db
        .with_conn(|c| repo::apply::list_records(c, limit.unwrap_or(50)))
}

#[tauri::command]
pub fn list_backups(
    state: State<'_, AppState>,
    target: Option<String>,
) -> AppResult<Vec<BackupInfo>> {
    let root = backups_dir()?;
    if !root.exists() {
        return Ok(vec![]);
    }
    let targets: Vec<TargetKind> = if let Some(t) = target.as_deref().and_then(TargetKind::parse) {
        vec![t]
    } else {
        vec![TargetKind::ClaudeCode, TargetKind::Codex, TargetKind::Pi]
    };
    backup::list_backups_in(&root, &targets, |dir| {
        state
            .db
            .with_conn(|c| repo::apply::find_record_by_backup_dir(c, dir))
            .ok()
            .flatten()
            .map(|r| (Some(r.id), Some(r.site_name_snapshot), Some(r.model_id)))
            .unwrap_or((None, None, None))
    })
}

#[tauri::command]
pub fn preview_backup(id: String) -> AppResult<BackupPreview> {
    backup::preview_backup_in(&backups_dir()?, &id)
}

#[tauri::command]
pub fn delete_backup(id: String) -> AppResult<()> {
    backup::delete_backup_in(&backups_dir()?, &id)
}

#[tauri::command]
pub fn restore_backup(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> AppResult<()> {
    let (target, _) = backup::parse_backup_id(&id)?;
    let _lock = try_lock_target(target.as_str())?;
    let settings = state.db.with_conn(repo::settings::get_settings)?;
    backup::restore_backup_in(&backups_dir()?, &id, &settings, None)?;
    crate::tray::request_tray_menu_sync(&app);
    Ok(())
}
