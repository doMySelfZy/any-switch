use crate::capabilities::{
    capability_on, CODEX_COMPACT, CODEX_IMAGEGEN, CODEX_SEARCH, CODEX_VISION,
};
use crate::domain::{
    ApplyTargetResult, CapabilitySource, ClaudeAuthKeyStyle, ClaudeEffortLevel,
    CodexReasoningEffort, SiteRow, TargetKind, TargetLiveStatus,
};
use crate::error::{AppError, AppResult};
use crate::repo;
use crate::state::AppState;
use serde::Serialize;
use std::collections::HashMap;
use tauri::{AppHandle, Emitter, Manager};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaudeHydration {
    pub model_id: Option<String>,
    pub opus_model: Option<String>,
    pub sonnet_model: Option<String>,
    pub haiku_model: Option<String>,
    pub effort: Option<ClaudeEffortLevel>,
    pub auth: ClaudeAuthKeyStyle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodexHydration {
    pub model_id: Option<String>,
    pub write_all_models: bool,
    pub reasoning: Option<CodexReasoningEffort>,
    pub remote_compaction: bool,
    pub image_understanding: bool,
    pub image_generation: bool,
    pub web_search: bool,
    pub capability_source: CapabilitySource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PiHydration {
    pub model_id: Option<String>,
    pub write_all_models: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TrayApplyFailed {
    code: String,
    message: String,
}

fn live_str(summary: &HashMap<String, Option<String>>, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(Some(value)) = summary.get(*key) {
            if !value.is_empty() {
                return Some(value.clone());
            }
        }
    }
    None
}

fn applied_on_site(site_id: &str, status: Option<&TargetLiveStatus>) -> bool {
    status
        .and_then(|s| s.applied_site_id.as_deref())
        .is_some_and(|id| id == site_id)
}

fn infer_claude_auth(
    summary: Option<&HashMap<String, Option<String>>>,
    fallback: ClaudeAuthKeyStyle,
) -> ClaudeAuthKeyStyle {
    let Some(summary) = summary else {
        return fallback;
    };
    if live_str(summary, &["ANTHROPIC_AUTH_TOKEN"]).is_some() {
        return ClaudeAuthKeyStyle::AnthropicAuthToken;
    }
    if live_str(summary, &["ANTHROPIC_API_KEY"]).is_some() {
        return ClaudeAuthKeyStyle::AnthropicApiKey;
    }
    fallback
}

pub fn hydrate_claude(site: &SiteRow, status: Option<&TargetLiveStatus>) -> ClaudeHydration {
    let live = status.map(|s| &s.live_summary);
    let on_site = applied_on_site(&site.id, status);
    let live_model = live
        .and_then(|s| live_str(s, &["ANTHROPIC_MODEL", "model"]))
        .or_else(|| status.and_then(|s| s.applied_model_id.clone()));
    let fallback_auth = site.claude_auth_key_style.clone();
    let effort = live.and_then(|s| live_str(s, &["CLAUDE_CODE_EFFORT_LEVEL", "effortLevel"]));
    let effort = effort.as_deref().and_then(ClaudeEffortLevel::parse);

    if on_site {
        ClaudeHydration {
            model_id: live_model.or_else(|| site.selected_model_id.clone()),
            opus_model: live.and_then(|s| live_str(s, &["ANTHROPIC_DEFAULT_OPUS_MODEL"])),
            sonnet_model: live.and_then(|s| live_str(s, &["ANTHROPIC_DEFAULT_SONNET_MODEL"])),
            haiku_model: live.and_then(|s| live_str(s, &["ANTHROPIC_DEFAULT_HAIKU_MODEL"])),
            effort,
            auth: infer_claude_auth(live, fallback_auth),
        }
    } else {
        ClaudeHydration {
            model_id: site.selected_model_id.clone(),
            opus_model: None,
            sonnet_model: None,
            haiku_model: None,
            effort,
            auth: fallback_auth,
        }
    }
}

pub fn hydrate_codex(site: &SiteRow, status: Option<&TargetLiveStatus>) -> CodexHydration {
    let live = status.map(|s| &s.live_summary);
    let on_site = applied_on_site(&site.id, status);
    let live_model = live
        .and_then(|s| live_str(s, &["model"]))
        .or_else(|| status.and_then(|s| s.applied_model_id.clone()));

    let web_search = match live.and_then(|s| live_str(s, &["web_search"])) {
        Some(value) => !value.eq_ignore_ascii_case("disabled"),
        None => live
            .and_then(|s| live_str(s, &["model", "model_provider"]))
            .is_some(),
    };

    let capability_source = CapabilitySource::parse(
        live.and_then(|s| live_str(s, &["capability_source"]))
            .as_deref(),
    );

    let (remote_compaction, image_understanding, image_generation, web_search) =
        if capability_source == CapabilitySource::Site {
            (
                capability_on(&site.capabilities, CODEX_COMPACT),
                capability_on(&site.capabilities, CODEX_VISION),
                capability_on(&site.capabilities, CODEX_IMAGEGEN),
                capability_on(&site.capabilities, CODEX_SEARCH),
            )
        } else {
            (
                live.and_then(|s| live_str(s, &["remote_compaction"]))
                    .is_some_and(|v| v.eq_ignore_ascii_case("on"))
                    || live
                        .and_then(|s| live_str(s, &["provider_display_name"]))
                        .is_some_and(|v| v == "OpenAI"),
                live.and_then(|s| live_str(s, &["tools_view_image", "view_image"]))
                    .is_some_and(|v| {
                        v.eq_ignore_ascii_case("true") || v.eq_ignore_ascii_case("on")
                    }),
                live.and_then(|s| live_str(s, &["features_image_generation", "image_generation"]))
                    .is_some_and(|v| {
                        v.eq_ignore_ascii_case("true") || v.eq_ignore_ascii_case("on")
                    }),
                web_search,
            )
        };

    CodexHydration {
        model_id: if on_site {
            live_model.or_else(|| site.selected_model_id.clone())
        } else {
            site.selected_model_id.clone()
        },
        write_all_models: live
            .and_then(|s| live_str(s, &["model_catalog_json"]))
            .is_some(),
        reasoning: live
            .and_then(|s| live_str(s, &["model_reasoning_effort"]))
            .as_deref()
            .and_then(CodexReasoningEffort::parse),
        remote_compaction,
        image_understanding,
        image_generation,
        web_search,
        capability_source,
    }
}

pub fn hydrate_pi(site: &SiteRow, status: Option<&TargetLiveStatus>) -> PiHydration {
    let live = status.map(|value| &value.live_summary);
    let on_site = applied_on_site(&site.id, status);
    let live_model = live
        .and_then(|summary| live_str(summary, &["defaultModel"]))
        .or_else(|| status.and_then(|value| value.applied_model_id.clone()));
    PiHydration {
        model_id: if on_site {
            live_model.or_else(|| site.selected_model_id.clone())
        } else {
            site.selected_model_id.clone()
        },
        write_all_models: live
            .and_then(|summary| live_str(summary, &["writeAllModels"]))
            .is_some_and(|value| value.eq_ignore_ascii_case("true"))
            || live
                .and_then(|summary| live_str(summary, &["modelCount"]))
                .and_then(|count| count.parse::<usize>().ok())
                .is_some_and(|count| count > 1),
    }
}

pub fn pick_tray_targets(
    has_claude_binding: bool,
    has_codex_binding: bool,
    has_pi_binding: bool,
) -> Vec<TargetKind> {
    if !has_claude_binding && !has_codex_binding && !has_pi_binding {
        return vec![TargetKind::ClaudeCode, TargetKind::Codex];
    }
    let mut out = Vec::new();
    if has_claude_binding {
        out.push(TargetKind::ClaudeCode);
    }
    if has_codex_binding {
        out.push(TargetKind::Codex);
    }
    if has_pi_binding {
        out.push(TargetKind::Pi);
    }
    out
}

fn emit_failed(app: &AppHandle, err: &AppError) {
    let AppError::Coded { code, message, .. } = err;
    let _ = app.emit(
        "tray-apply-failed",
        TrayApplyFailed {
            code: (*code).into(),
            message: message.clone(),
        },
    );
}

pub fn apply_site_from_tray(app: &AppHandle, site_id: &str) {
    match apply_site_from_tray_inner(app, site_id) {
        Ok(results) => {
            let _ = app.emit("tray-applied", results);
            crate::tray::request_tray_menu_sync(app);
        }
        Err(err) => {
            tracing::warn!("Tray apply failed: {err}");
            emit_failed(app, &err);
        }
    }
}

fn apply_site_from_tray_inner(app: &AppHandle, site_id: &str) -> AppResult<Vec<ApplyTargetResult>> {
    let state = app.state::<AppState>();
    let site = state.db.with_conn(|c| repo::site::get_site(c, site_id))?;
    let bindings = state.db.with_conn(repo::binding::list_bindings)?;
    let tools = crate::commands::targets::detect_cli_tools_cached(false);
    let statuses = crate::commands::targets::list_target_status_with_tools(&state, &tools)?;

    let has_claude = bindings.iter().any(|b| b.target == TargetKind::ClaudeCode);
    let has_codex = bindings.iter().any(|b| b.target == TargetKind::Codex);
    let has_pi = bindings.iter().any(|b| b.target == TargetKind::Pi);
    let targets = pick_tray_targets(has_claude, has_codex, has_pi);

    let mut results = Vec::new();
    let mut attempted = false;
    for target in targets {
        let status = statuses.iter().find(|s| s.kind == target);
        match target {
            TargetKind::ClaudeCode => {
                let h = hydrate_claude(&site, status);
                let Some(model_id) = h.model_id.filter(|s| !s.trim().is_empty()) else {
                    continue;
                };
                attempted = true;
                let applied = crate::commands::apply::apply_site(
                    app.clone(),
                    app.state::<AppState>(),
                    site.id.clone(),
                    vec![TargetKind::ClaudeCode],
                    model_id,
                    Some(h.auth.as_str().into()),
                    h.opus_model,
                    h.sonnet_model,
                    h.haiku_model,
                    h.effort.map(|e| e.as_str().into()),
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                )?;
                results.extend(applied.results);
            }
            TargetKind::Codex => {
                let h = hydrate_codex(&site, status);
                let Some(model_id) = h.model_id.filter(|s| !s.trim().is_empty()) else {
                    continue;
                };
                attempted = true;
                let applied = crate::commands::apply::apply_site(
                    app.clone(),
                    app.state::<AppState>(),
                    site.id.clone(),
                    vec![TargetKind::Codex],
                    model_id,
                    None,
                    None,
                    None,
                    None,
                    None,
                    Some(h.write_all_models),
                    h.reasoning.map(|e| e.as_str().into()),
                    Some(h.remote_compaction),
                    Some(h.image_understanding),
                    Some(h.image_generation),
                    Some(h.web_search),
                    Some(h.capability_source.as_str().into()),
                    None,
                )?;
                results.extend(applied.results);
            }
            TargetKind::Pi => {
                let hydration = hydrate_pi(&site, status);
                let Some(model_id) = hydration.model_id.filter(|value| !value.trim().is_empty())
                else {
                    continue;
                };
                attempted = true;
                let applied = crate::commands::apply::apply_site(
                    app.clone(),
                    app.state::<AppState>(),
                    site.id.clone(),
                    vec![TargetKind::Pi],
                    model_id,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    Some(hydration.write_all_models),
                )?;
                results.extend(applied.results);
            }
        }
    }

    if !attempted {
        return Err(AppError::new(
            "validation_failed",
            "site has no selected model",
        ));
    }
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{ApplyStatus, SiteProtocol};

    fn site(id: &str, selected: Option<&str>, auth: ClaudeAuthKeyStyle) -> SiteRow {
        SiteRow {
            id: id.into(),
            name: id.into(),
            base_url: "https://api.example.com".into(),
            base_urls: vec!["https://api.example.com".into()],
            api_key_encrypted: String::new(),
            key_prefix: "sk-xx".into(),
            protocol: SiteProtocol::OpenaiCompatible,
            claude_auth_key_style: auth,
            notes: None,
            enabled: true,
            sort_order: 0,
            selected_model_id: selected.map(|s| s.into()),
            last_model_fetch_at: None,
            last_model_fetch_latency_ms: None,
            last_model_fetch_error: None,
            created_at: 1,
            updated_at: 1,
            capabilities: Default::default(),
        }
    }

    fn claude_status() -> TargetLiveStatus {
        let mut live = HashMap::new();
        live.insert("ANTHROPIC_MODEL".into(), Some("codex-auto-review".into()));
        live.insert(
            "ANTHROPIC_DEFAULT_OPUS_MODEL".into(),
            Some("opus-live".into()),
        );
        live.insert(
            "ANTHROPIC_DEFAULT_SONNET_MODEL".into(),
            Some("sonnet-live".into()),
        );
        live.insert(
            "ANTHROPIC_DEFAULT_HAIKU_MODEL".into(),
            Some("haiku-live".into()),
        );
        live.insert("ANTHROPIC_AUTH_TOKEN".into(), Some("sk-live".into()));
        live.insert("CLAUDE_CODE_EFFORT_LEVEL".into(), Some("high".into()));
        TargetLiveStatus {
            kind: TargetKind::ClaudeCode,
            installed: true,
            version: Some("2.1.0".into()),
            config_path: "/tmp/settings.json".into(),
            status: ApplyStatus::Applied,
            applied_site_id: Some("shuai".into()),
            applied_site_name: Some("shuai".into()),
            applied_model_id: Some("codex-auto-review".into()),
            provider_id: None,
            orphan: false,
            live_summary: live,
            last_applied_at: Some(99),
            stale_reason: None,
        }
    }

    fn codex_status() -> TargetLiveStatus {
        let mut live = HashMap::new();
        live.insert("model".into(), Some("codex-auto-review".into()));
        live.insert("model_reasoning_effort".into(), Some("xhigh".into()));
        live.insert("model_catalog_json".into(), Some("/tmp/models.json".into()));
        TargetLiveStatus {
            kind: TargetKind::Codex,
            installed: true,
            version: None,
            config_path: "/tmp/config.toml".into(),
            status: ApplyStatus::Applied,
            applied_site_id: Some("shuai".into()),
            applied_site_name: Some("shuai".into()),
            applied_model_id: Some("codex-auto-review".into()),
            provider_id: None,
            orphan: false,
            live_summary: live,
            last_applied_at: Some(99),
            stale_reason: None,
        }
    }

    #[test]
    fn hydrate_claude_uses_live_for_applied_site() {
        let defaults = hydrate_claude(
            &site(
                "shuai",
                Some("gpt-4.1"),
                ClaudeAuthKeyStyle::AnthropicApiKey,
            ),
            Some(&claude_status()),
        );
        assert_eq!(defaults.model_id.as_deref(), Some("codex-auto-review"));
        assert_eq!(defaults.opus_model.as_deref(), Some("opus-live"));
        assert_eq!(defaults.sonnet_model.as_deref(), Some("sonnet-live"));
        assert_eq!(defaults.haiku_model.as_deref(), Some("haiku-live"));
        assert_eq!(defaults.effort, Some(ClaudeEffortLevel::High));
        assert_eq!(defaults.auth, ClaudeAuthKeyStyle::AnthropicAuthToken);
    }

    #[test]
    fn hydrate_claude_does_not_copy_aliases_when_switching_site() {
        let defaults = hydrate_claude(
            &site(
                "gptnb",
                Some("gpt-4.1"),
                ClaudeAuthKeyStyle::AnthropicApiKey,
            ),
            Some(&claude_status()),
        );
        assert_eq!(defaults.model_id.as_deref(), Some("gpt-4.1"));
        assert_eq!(defaults.opus_model, None);
        assert_eq!(defaults.sonnet_model, None);
        assert_eq!(defaults.haiku_model, None);
        assert_eq!(defaults.auth, ClaudeAuthKeyStyle::AnthropicApiKey);
        assert_eq!(defaults.effort, Some(ClaudeEffortLevel::High));
    }

    #[test]
    fn hydrate_claude_empty_when_nothing_written() {
        let defaults = hydrate_claude(
            &site(
                "shuai",
                Some("gpt-4.1"),
                ClaudeAuthKeyStyle::AnthropicAuthToken,
            ),
            None,
        );
        assert_eq!(defaults.model_id.as_deref(), Some("gpt-4.1"));
        assert_eq!(defaults.effort, None);
        assert_eq!(defaults.opus_model, None);
    }

    #[test]
    fn hydrate_codex_uses_live_for_applied_site() {
        let defaults = hydrate_codex(
            &site(
                "shuai",
                Some("gpt-4.1"),
                ClaudeAuthKeyStyle::AnthropicAuthToken,
            ),
            Some(&codex_status()),
        );
        assert_eq!(defaults.model_id.as_deref(), Some("codex-auto-review"));
        assert!(defaults.write_all_models);
        assert_eq!(defaults.reasoning, Some(CodexReasoningEffort::Xhigh));
        assert!(!defaults.remote_compaction);
        assert!(!defaults.image_understanding);
        assert!(!defaults.image_generation);
        assert!(!defaults.web_search);
        assert_eq!(defaults.capability_source, CapabilitySource::Site);
    }

    #[test]
    fn hydrate_codex_uses_site_model_when_switching() {
        let defaults = hydrate_codex(
            &site(
                "gptnb",
                Some("gpt-4.1"),
                ClaudeAuthKeyStyle::AnthropicAuthToken,
            ),
            Some(&codex_status()),
        );
        assert_eq!(defaults.model_id.as_deref(), Some("gpt-4.1"));
        assert!(defaults.write_all_models);
        assert_eq!(defaults.reasoning, Some(CodexReasoningEffort::Xhigh));
    }

    #[test]
    fn hydrate_codex_does_not_force_catalog_when_empty() {
        let defaults = hydrate_codex(
            &site(
                "shuai",
                Some("gpt-4.1"),
                ClaudeAuthKeyStyle::AnthropicAuthToken,
            ),
            None,
        );
        assert!(!defaults.write_all_models);
        assert_eq!(defaults.reasoning, None);
        assert!(!defaults.remote_compaction);
        assert!(!defaults.image_understanding);
        assert!(!defaults.image_generation);
        assert!(!defaults.web_search);
        assert_eq!(defaults.capability_source, CapabilitySource::Site);
    }

    #[test]
    fn hydrate_codex_reads_platform_capabilities() {
        let mut live = HashMap::new();
        live.insert("model".into(), Some("gpt-5.4".into()));
        live.insert("capability_source".into(), Some("custom".into()));
        live.insert("remote_compaction".into(), Some("on".into()));
        live.insert("tools_view_image".into(), Some("true".into()));
        live.insert("features_image_generation".into(), Some("true".into()));
        live.insert("web_search".into(), Some("cached".into()));
        let mut status = codex_status();
        status.live_summary = live;
        let defaults = hydrate_codex(
            &site(
                "shuai",
                Some("gpt-4.1"),
                ClaudeAuthKeyStyle::AnthropicAuthToken,
            ),
            Some(&status),
        );
        assert!(defaults.remote_compaction);
        assert!(defaults.image_understanding);
        assert!(defaults.image_generation);
        assert!(defaults.web_search);
        assert_eq!(defaults.capability_source, CapabilitySource::Custom);
    }

    #[test]
    fn hydrate_codex_follow_site_reads_current_presets() {
        let mut live = HashMap::new();
        live.insert("model".into(), Some("gpt-5.4".into()));
        live.insert("capability_source".into(), Some("site".into()));
        live.insert("remote_compaction".into(), Some("off".into()));
        live.insert("web_search".into(), Some("disabled".into()));
        let mut status = codex_status();
        status.live_summary = live;
        let mut row = site(
            "shuai",
            Some("gpt-4.1"),
            ClaudeAuthKeyStyle::AnthropicAuthToken,
        );
        row.capabilities.insert(CODEX_COMPACT.into(), true);
        row.capabilities.insert(CODEX_VISION.into(), true);
        let defaults = hydrate_codex(&row, Some(&status));
        assert_eq!(defaults.capability_source, CapabilitySource::Site);
        assert!(defaults.remote_compaction);
        assert!(defaults.image_understanding);
        assert!(!defaults.image_generation);
        assert!(!defaults.web_search);
    }

    #[test]
    fn hydrate_pi_preserves_write_all_with_one_live_model() {
        let mut status = codex_status();
        status.kind = TargetKind::Pi;
        status.live_summary = HashMap::from([
            ("defaultModel".into(), Some("model-a".into())),
            ("modelCount".into(), Some("1".into())),
            ("writeAllModels".into(), Some("true".into())),
        ]);
        let defaults = hydrate_pi(
            &site(
                "shuai",
                Some("fallback"),
                ClaudeAuthKeyStyle::AnthropicAuthToken,
            ),
            Some(&status),
        );
        assert_eq!(defaults.model_id.as_deref(), Some("model-a"));
        assert!(defaults.write_all_models);
    }

    #[test]
    fn pick_targets_defaults_to_both_when_unbound() {
        assert_eq!(
            pick_tray_targets(false, false, false),
            vec![TargetKind::ClaudeCode, TargetKind::Codex]
        );
        assert_eq!(
            pick_tray_targets(true, false, false),
            vec![TargetKind::ClaudeCode]
        );
        assert_eq!(
            pick_tray_targets(false, true, false),
            vec![TargetKind::Codex]
        );
        assert_eq!(pick_tray_targets(false, false, true), vec![TargetKind::Pi]);
        assert_eq!(
            pick_tray_targets(true, true, true),
            vec![TargetKind::ClaudeCode, TargetKind::Codex, TargetKind::Pi]
        );
    }

    #[test]
    fn skip_target_without_model_when_not_on_site() {
        let defaults = hydrate_claude(
            &site("gptnb", None, ClaudeAuthKeyStyle::AnthropicAuthToken),
            Some(&claude_status()),
        );
        assert_eq!(defaults.model_id, None);
    }
}
