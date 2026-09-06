use crate::domain::{
    allows_extended, is_extended_level, SiteProtocol, TargetBinding, TargetKind, ThinkingWrite,
    BASELINE_MISSING, BASELINE_MODEL_THINKING, BASELINE_THINKING_LEVEL, CLEARED_MODEL_THINKING_KEY,
    WROTE_THINKING_LEVEL,
};
use crate::error::{AppError, AppResult};
use jsonc_parser::cst::CstInputValue;
use serde_json::{Map, Value};
use std::collections::HashMap;

pub fn push_model_thinking(
    fields: &mut Vec<(String, CstInputValue)>,
    model_id: &str,
    thinking: &ThinkingWrite,
    protocol: SiteProtocol,
    target: TargetKind,
) {
    let Some(cfg) = thinking.model(model_id).filter(|cfg| cfg.reasoning) else {
        return;
    };
    fields.push(("reasoning".into(), CstInputValue::Bool(true)));
    if allows_extended(target, protocol, Some(cfg)) {
        let map: Vec<(String, CstInputValue)> = cfg
            .thinking_level_map
            .iter()
            .filter(|(level, _)| is_extended_level(level))
            .map(|(level, value)| (level.clone(), CstInputValue::String(value.clone())))
            .collect();
        if !map.is_empty() {
            fields.push(("thinkingLevelMap".into(), CstInputValue::Object(map)));
        }
    }
    if target == TargetKind::Pi
        && protocol == SiteProtocol::Anthropic
        && cfg.force_adaptive_thinking
    {
        fields.push((
            "compat".into(),
            CstInputValue::Object(vec![(
                "forceAdaptiveThinking".into(),
                CstInputValue::Bool(true),
            )]),
        ));
    }
}

pub fn fill_extended_maps(write: &mut ThinkingWrite) {
    let xhigh = write.extended.xhigh.clone();
    let max = write.extended.max.clone();
    for cfg in write.models.values_mut() {
        if !cfg.reasoning {
            continue;
        }
        if let Some(value) = xhigh.clone() {
            cfg.thinking_level_map
                .entry("xhigh".into())
                .or_insert(value);
        }
        if let Some(value) = max.clone() {
            cfg.thinking_level_map.entry("max".into()).or_insert(value);
        }
    }
}

pub fn apply_settings_thinking(
    settings: &mut Map<String, Value>,
    thinking: &ThinkingWrite,
    provider_id: &str,
    model_id: &str,
    target: TargetKind,
    binding_before: Option<&TargetBinding>,
) -> HashMap<String, String> {
    let mut expected = HashMap::new();
    let (baseline_level, baseline_model) =
        capture_thinking_baseline(settings, provider_id, model_id, target, binding_before);
    expected.insert(BASELINE_THINKING_LEVEL.into(), baseline_level);
    expected.insert(BASELINE_MODEL_THINKING.into(), baseline_model);
    let Some(level) = thinking.default_level.as_deref() else {
        return expected;
    };
    settings.insert(
        "defaultThinkingLevel".into(),
        Value::String(level.to_string()),
    );
    expected.insert(WROTE_THINKING_LEVEL.into(), level.to_string());
    if target == TargetKind::Pi {
        let key = format!("{provider_id}/{model_id}");
        clear_model_thinking_level(settings, &key);
        expected.insert(CLEARED_MODEL_THINKING_KEY.into(), key);
    }
    expected
}

pub fn restore_settings_thinking(settings: &mut Map<String, Value>, binding: &TargetBinding) {
    let Some(wrote) = binding.expected_fields.get(WROTE_THINKING_LEVEL) else {
        return;
    };
    let live = settings
        .get("defaultThinkingLevel")
        .and_then(Value::as_str)
        .unwrap_or(BASELINE_MISSING);
    if live == wrote {
        match binding
            .expected_fields
            .get(BASELINE_THINKING_LEVEL)
            .map(String::as_str)
        {
            Some(BASELINE_MISSING) | None => {
                settings.remove("defaultThinkingLevel");
            }
            Some(value) => {
                settings.insert(
                    "defaultThinkingLevel".into(),
                    Value::String(value.to_string()),
                );
            }
        }
    }
    let Some(key) = binding.expected_fields.get(CLEARED_MODEL_THINKING_KEY) else {
        return;
    };
    let live_override = settings
        .get("modelThinkingLevels")
        .and_then(Value::as_object)
        .and_then(|object| object.get(key));
    if live_override.is_some() {
        return;
    }
    match binding
        .expected_fields
        .get(BASELINE_MODEL_THINKING)
        .map(String::as_str)
    {
        Some(BASELINE_MISSING) | None => {}
        Some(value) => {
            let object = settings
                .entry("modelThinkingLevels".to_string())
                .or_insert_with(|| Value::Object(Map::new()));
            if let Some(object) = object.as_object_mut() {
                object.insert(key.clone(), Value::String(value.to_string()));
            }
        }
    }
}

pub fn verify_written_thinking(
    provider: &Value,
    settings: &Value,
    thinking: &ThinkingWrite,
    protocol: SiteProtocol,
    target: TargetKind,
) -> AppResult<()> {
    let models = provider
        .get("models")
        .and_then(Value::as_array)
        .ok_or_else(|| AppError::new("invalid_config", "managed models list missing"))?;
    for model in models {
        let id = model.get("id").and_then(Value::as_str).unwrap_or_default();
        let want = thinking.model(id).filter(|cfg| cfg.reasoning);
        let live_reasoning = model.get("reasoning").and_then(Value::as_bool) == Some(true);
        if want.is_some() != live_reasoning {
            return Err(AppError::new(
                "invalid_config",
                format!("thinking capability mismatch for {id}"),
            ));
        }
        let Some(cfg) = want else {
            continue;
        };
        if allows_extended(target, protocol, Some(cfg)) && !cfg.thinking_level_map.is_empty() {
            let live_map = model.get("thinkingLevelMap").and_then(Value::as_object);
            for (level, value) in &cfg.thinking_level_map {
                if !is_extended_level(level) {
                    continue;
                }
                if live_map
                    .and_then(|map| map.get(level))
                    .and_then(Value::as_str)
                    != Some(value)
                {
                    return Err(AppError::new(
                        "invalid_config",
                        format!("thinking level map mismatch for {id}.{level}"),
                    ));
                }
            }
        }
        if target == TargetKind::Pi
            && protocol == SiteProtocol::Anthropic
            && cfg.force_adaptive_thinking
        {
            let adaptive = model
                .get("compat")
                .and_then(|value| value.get("forceAdaptiveThinking"))
                .and_then(Value::as_bool)
                == Some(true);
            if !adaptive {
                return Err(AppError::new(
                    "invalid_config",
                    format!("adaptive thinking missing for {id}"),
                ));
            }
        }
    }
    if let Some(level) = thinking.default_level.as_deref() {
        if settings.get("defaultThinkingLevel").and_then(Value::as_str) != Some(level) {
            return Err(AppError::new(
                "invalid_config",
                "defaultThinkingLevel was not written",
            ));
        }
    }
    Ok(())
}

pub fn thinking_live_fields(
    provider: Option<&Value>,
    settings: &Value,
) -> Vec<(String, Option<String>)> {
    let reasoning_count = provider
        .and_then(|value| value.get("models"))
        .and_then(Value::as_array)
        .map(|models| {
            models
                .iter()
                .filter(|model| model.get("reasoning").and_then(Value::as_bool) == Some(true))
                .count()
        })
        .unwrap_or(0);
    vec![
        (
            "defaultThinkingLevel".into(),
            settings
                .get("defaultThinkingLevel")
                .and_then(Value::as_str)
                .map(str::to_string),
        ),
        (
            "reasoningModelCount".into(),
            Some(reasoning_count.to_string()),
        ),
    ]
}

fn capture_thinking_baseline(
    settings: &Map<String, Value>,
    provider_id: &str,
    model_id: &str,
    target: TargetKind,
    binding_before: Option<&TargetBinding>,
) -> (String, String) {
    if let Some(binding) = binding_before {
        if let Some(level) = binding.expected_fields.get(BASELINE_THINKING_LEVEL) {
            let model = binding
                .expected_fields
                .get(BASELINE_MODEL_THINKING)
                .cloned()
                .unwrap_or_else(|| BASELINE_MISSING.into());
            return (level.clone(), model);
        }
    }
    let level = settings
        .get("defaultThinkingLevel")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| BASELINE_MISSING.into());
    let model = if target == TargetKind::Pi {
        let key = format!("{provider_id}/{model_id}");
        settings
            .get("modelThinkingLevels")
            .and_then(Value::as_object)
            .and_then(|object| object.get(&key))
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| BASELINE_MISSING.into())
    } else {
        BASELINE_MISSING.into()
    };
    (level, model)
}

fn clear_model_thinking_level(settings: &mut Map<String, Value>, key: &str) {
    let Some(Value::Object(object)) = settings.get_mut("modelThinkingLevels") else {
        return;
    };
    object.remove(key);
    if object.is_empty() {
        settings.remove("modelThinkingLevels");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ModelThinkingConfig;
    use serde_json::json;

    fn write_with(model: ModelThinkingConfig) -> ThinkingWrite {
        let mut write = ThinkingWrite::default();
        write.models.insert("model-a".into(), model);
        write
    }

    #[test]
    fn verify_accepts_reasoning_and_default_level() {
        let mut write = write_with(ModelThinkingConfig {
            reasoning: true,
            ..Default::default()
        });
        write.default_level = Some("medium".into());
        let provider = json!({
            "models": [{ "id": "model-a", "reasoning": true }]
        });
        let settings = json!({ "defaultThinkingLevel": "medium" });
        verify_written_thinking(
            &provider,
            &settings,
            &write,
            SiteProtocol::OpenaiCompatible,
            TargetKind::Pi,
        )
        .unwrap();
    }

    #[test]
    fn restore_keeps_user_changed_thinking_level() {
        let mut settings = json!({
            "defaultThinkingLevel": "high"
        })
        .as_object()
        .cloned()
        .unwrap();
        let binding = TargetBinding {
            target: TargetKind::Pi,
            site_id: None,
            site_name_snapshot: "s".into(),
            model_id: "model-a".into(),
            provider_id: Some("xiaobai_x".into()),
            key_fingerprint: "f".into(),
            managed_paths: vec![],
            managed_env_keys: vec![],
            expected_fields: HashMap::from([
                (WROTE_THINKING_LEVEL.into(), "medium".into()),
                (BASELINE_THINKING_LEVEL.into(), "off".into()),
            ]),
            orphan: false,
            applied_at: 0,
            apply_record_id: None,
            api_key: Default::default(),
        };
        restore_settings_thinking(&mut settings, &binding);
        assert_eq!(settings["defaultThinkingLevel"], "high");
    }

    #[test]
    fn restore_puts_back_original_level() {
        let mut settings = json!({
            "defaultThinkingLevel": "medium"
        })
        .as_object()
        .cloned()
        .unwrap();
        let binding = TargetBinding {
            target: TargetKind::Pi,
            site_id: None,
            site_name_snapshot: "s".into(),
            model_id: "model-a".into(),
            provider_id: Some("xiaobai_x".into()),
            key_fingerprint: "f".into(),
            managed_paths: vec![],
            managed_env_keys: vec![],
            expected_fields: HashMap::from([
                (WROTE_THINKING_LEVEL.into(), "medium".into()),
                (BASELINE_THINKING_LEVEL.into(), BASELINE_MISSING.into()),
            ]),
            orphan: false,
            applied_at: 0,
            apply_record_id: None,
            api_key: Default::default(),
        };
        restore_settings_thinking(&mut settings, &binding);
        assert!(settings.get("defaultThinkingLevel").is_none());
    }
}
