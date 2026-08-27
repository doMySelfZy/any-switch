use crate::domain::{ApplyStatus, ApplyTargetResult, SiteRow, SwitchRouteResult, TargetKind};
use crate::error::AppResult;
use crate::lock::try_lock_target;
use crate::paths::backups_dir;
use crate::repo;
use crate::state::AppState;
use chrono::Utc;
use std::fs;

pub fn switch_site_route(
    state: &AppState,
    site_id: &str,
    base_url: &str,
    apply: bool,
) -> AppResult<SwitchRouteResult> {
    let before = state.db.with_conn(|c| repo::site::get_site(c, site_id))?;
    let site = state
        .db
        .with_conn(|c| repo::site::switch_site_route(c, site_id, base_url))?;
    let results = if apply && before.base_url != site.base_url {
        sync_applied_urls(state, &site)?
    } else {
        vec![]
    };
    Ok(SwitchRouteResult {
        site: site.to_dto(),
        results,
    })
}

pub fn sync_applied_urls(state: &AppState, site: &SiteRow) -> AppResult<Vec<ApplyTargetResult>> {
    let settings = state.db.with_conn(repo::settings::get_settings)?;
    let bindings = state
        .db
        .with_conn(|c| repo::binding::list_bindings_for_site(c, &site.id))?;
    let applied_at = Utc::now().timestamp_millis();
    let mut results = Vec::new();

    for binding in bindings {
        if binding.orphan {
            continue;
        }
        let target = binding.target;
        let _lock = try_lock_target(target.as_str())?;
        let backup_root = backups_dir()?
            .join(target.as_str())
            .join(format!("{}", applied_at));
        fs::create_dir_all(&backup_root)?;

        let rewrite = match target {
            TargetKind::ClaudeCode => crate::adapters::claude_code::rewrite_base_url(
                site,
                &binding,
                settings.claude_home_override.as_deref(),
                &backup_root,
            ),
            TargetKind::Codex => crate::adapters::codex::rewrite_base_url(
                site,
                &binding,
                settings.codex_home_override.as_deref(),
                &backup_root,
            ),
            TargetKind::Pi => crate::adapters::pi::rewrite_base_url(
                &binding,
                site,
                settings.pi_agent_dir_override.as_deref(),
                &backup_root,
            ),
        };

        match rewrite {
            Ok(o) => {
                let mut next = binding.clone();
                next.expected_fields = o.expected_fields;
                next.applied_at = applied_at;
                state
                    .db
                    .with_conn(|c| repo::binding::upsert_binding(c, &next))?;
                results.push(ApplyTargetResult {
                    target,
                    ok: true,
                    status: ApplyStatus::Applied,
                    backup_paths: o.backup_paths,
                    message: o.message,
                    live_summary: Some(o.live_summary),
                    touched_keys: None,
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
