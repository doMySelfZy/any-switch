use super::{SiteProtocol, TargetKind};
use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const BASELINE_THINKING_LEVEL: &str = "baseline_default_thinking_level";
pub const BASELINE_MODEL_THINKING: &str = "baseline_model_thinking_level";
pub const WROTE_THINKING_LEVEL: &str = "wrote_default_thinking_level";
pub const CLEARED_MODEL_THINKING_KEY: &str = "cleared_model_thinking_key";
pub const BASELINE_MISSING: &str = "__xiaobai_missing__";

const STANDARD_LEVELS: &[&str] = &["off", "minimal", "low", "medium", "high"];
const EXTENDED_LEVELS: &[&str] = &["xhigh", "max"];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ThinkingExtendedMap {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub xhigh: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ModelThinkingConfig {
    #[serde(default)]
    pub reasoning: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub force_adaptive_thinking: bool,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub thinking_level_map: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ThinkingWrite {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_level: Option<String>,
    #[serde(default)]
    pub extended: ThinkingExtendedMap,
    #[serde(default)]
    pub models: BTreeMap<String, ModelThinkingConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SiteThinkingPreset {
    pub site_id: String,
    pub target: TargetKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_level: Option<String>,
    #[serde(default)]
    pub extended: ThinkingExtendedMap,
    #[serde(default)]
    pub models: BTreeMap<String, ModelThinkingConfig>,
}

impl SiteThinkingPreset {
    pub fn empty(site_id: impl Into<String>, target: TargetKind) -> Self {
        Self {
            site_id: site_id.into(),
            target,
            default_level: None,
            extended: ThinkingExtendedMap::default(),
            models: BTreeMap::new(),
        }
    }

    pub fn to_write(&self) -> ThinkingWrite {
        ThinkingWrite {
            default_level: self.default_level.clone(),
            extended: self.extended.clone(),
            models: self.models.clone(),
        }
    }
}

impl ThinkingWrite {
    pub fn model(&self, id: &str) -> Option<&ModelThinkingConfig> {
        self.models.get(id)
    }

    pub fn mapping_for(&self, model_id: &str, level: &str) -> Option<String> {
        if let Some(value) = self
            .models
            .get(model_id)
            .and_then(|cfg| cfg.thinking_level_map.get(level))
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
        {
            return Some(value.to_string());
        }
        match level {
            "xhigh" => non_empty(self.extended.xhigh.as_deref()),
            "max" => non_empty(self.extended.max.as_deref()),
            _ => None,
        }
    }
}

pub fn is_thinking_target(target: TargetKind) -> bool {
    matches!(target, TargetKind::Pi | TargetKind::Prime)
}

pub fn parse_thinking_level(raw: &str) -> Option<String> {
    let level = raw.trim().to_ascii_lowercase();
    if STANDARD_LEVELS.contains(&level.as_str()) || EXTENDED_LEVELS.contains(&level.as_str()) {
        Some(level)
    } else {
        None
    }
}

pub fn is_extended_level(level: &str) -> bool {
    EXTENDED_LEVELS.contains(&level)
}

pub fn allows_extended(
    target: TargetKind,
    protocol: SiteProtocol,
    model: Option<&ModelThinkingConfig>,
) -> bool {
    match (target, protocol) {
        (_, SiteProtocol::OpenaiCompatible) => true,
        (TargetKind::Pi, SiteProtocol::Anthropic) => {
            model.is_some_and(|cfg| cfg.force_adaptive_thinking)
        }
        (TargetKind::Prime, SiteProtocol::Anthropic) => false,
        _ => false,
    }
}

pub fn normalize_preset(mut preset: SiteThinkingPreset) -> SiteThinkingPreset {
    preset.default_level = preset
        .default_level
        .as_deref()
        .and_then(parse_thinking_level);
    preset.extended.xhigh = non_empty(preset.extended.xhigh.as_deref());
    preset.extended.max = non_empty(preset.extended.max.as_deref());
    let mut models = BTreeMap::new();
    for (id, mut cfg) in preset.models {
        let id = id.trim().to_string();
        if id.is_empty() {
            continue;
        }
        cfg.thinking_level_map = cfg
            .thinking_level_map
            .into_iter()
            .filter_map(|(level, value)| {
                let level = parse_thinking_level(&level)?;
                let value = non_empty(Some(value.as_str()))?;
                Some((level, value))
            })
            .collect();
        if !cfg.reasoning {
            cfg.force_adaptive_thinking = false;
        }
        models.insert(id, cfg);
    }
    preset.models = models;
    preset
}

pub fn validate_write(
    write: &ThinkingWrite,
    target: TargetKind,
    protocol: SiteProtocol,
    model_id: &str,
) -> AppResult<()> {
    let preset = SiteThinkingPreset {
        site_id: String::new(),
        target,
        default_level: write.default_level.clone(),
        extended: write.extended.clone(),
        models: write.models.clone(),
    };
    validate_preset(&preset, protocol, Some(model_id))
}

pub fn validate_preset(
    preset: &SiteThinkingPreset,
    protocol: SiteProtocol,
    apply_model_id: Option<&str>,
) -> AppResult<()> {
    if !is_thinking_target(preset.target) {
        return Err(AppError::new(
            "validation_failed",
            "thinking presets are only supported for Pi and Prime",
        ));
    }
    for cfg in preset.models.values() {
        if cfg.force_adaptive_thinking
            && (preset.target != TargetKind::Pi || protocol != SiteProtocol::Anthropic)
        {
            return Err(AppError::new(
                "validation_failed",
                "adaptive thinking is only available for Pi with the Anthropic protocol",
            ));
        }
        for level in cfg.thinking_level_map.keys() {
            if parse_thinking_level(level).is_none() {
                return Err(AppError::new(
                    "validation_failed",
                    format!("unknown thinking level in model map: {level}"),
                ));
            }
        }
    }
    let Some(level) = preset.default_level.as_deref() else {
        return Ok(());
    };
    let Some(level) = parse_thinking_level(level) else {
        return Err(AppError::new(
            "validation_failed",
            format!("unknown thinking level: {level}"),
        ));
    };
    let write = preset.to_write();
    if is_extended_level(&level) {
        validate_extended(preset, &write, protocol, apply_model_id, &level)?;
    } else if let Some(model_id) = apply_model_id {
        require_reasoning(&write, model_id)?;
    }
    Ok(())
}

fn validate_extended(
    preset: &SiteThinkingPreset,
    write: &ThinkingWrite,
    protocol: SiteProtocol,
    apply_model_id: Option<&str>,
    level: &str,
) -> AppResult<()> {
    if preset.target == TargetKind::Prime && protocol == SiteProtocol::Anthropic {
        return Err(AppError::new(
            "validation_failed",
            "Prime Anthropic models do not support extended thinking levels",
        ));
    }
    if let Some(model_id) = apply_model_id {
        require_reasoning(write, model_id)?;
        let cfg = write.model(model_id);
        if !allows_extended(preset.target, protocol, cfg) {
            return Err(AppError::new(
                "validation_failed",
                "enable adaptive thinking on the default model before using xhigh or max",
            ));
        }
        if write.mapping_for(model_id, level).is_none() {
            return Err(AppError::new(
                "validation_failed",
                format!("set the provider value for {level} before using it as the default"),
            ));
        }
        return Ok(());
    }
    let has_mapping = write.extended.xhigh.is_some() && level == "xhigh"
        || write.extended.max.is_some() && level == "max"
        || write
            .models
            .values()
            .any(|cfg| cfg.thinking_level_map.contains_key(level));
    if !has_mapping {
        return Err(AppError::new(
            "validation_failed",
            format!("set the provider value for {level} before using it as the default"),
        ));
    }
    if preset.target == TargetKind::Pi && protocol == SiteProtocol::Anthropic {
        let adaptive = write.models.values().any(|cfg| cfg.force_adaptive_thinking);
        if !adaptive {
            return Err(AppError::new(
                "validation_failed",
                "enable adaptive thinking before using xhigh or max on Pi Anthropic",
            ));
        }
    }
    Ok(())
}

fn require_reasoning(write: &ThinkingWrite, model_id: &str) -> AppResult<()> {
    if write.model(model_id).is_some_and(|cfg| cfg.reasoning) {
        Ok(())
    } else {
        Err(AppError::new(
            "validation_failed",
            "enable thinking on the default model before choosing a default thinking level",
        ))
    }
}

fn non_empty(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn preset(target: TargetKind) -> SiteThinkingPreset {
        let mut preset = SiteThinkingPreset::empty("s1", target);
        preset.models.insert(
            "model-a".into(),
            ModelThinkingConfig {
                reasoning: true,
                ..Default::default()
            },
        );
        preset
    }

    #[test]
    fn empty_preset_is_valid() {
        validate_preset(
            &SiteThinkingPreset::empty("s1", TargetKind::Pi),
            SiteProtocol::OpenaiCompatible,
            Some("model-a"),
        )
        .unwrap();
    }

    #[test]
    fn standard_level_requires_reasoning_on_apply() {
        let mut row = SiteThinkingPreset::empty("s1", TargetKind::Pi);
        row.default_level = Some("medium".into());
        let err =
            validate_preset(&row, SiteProtocol::OpenaiCompatible, Some("model-a")).unwrap_err();
        assert!(err.to_string().contains("default model"));
    }

    #[test]
    fn extended_openai_needs_mapping() {
        let mut row = preset(TargetKind::Pi);
        row.default_level = Some("max".into());
        let err =
            validate_preset(&row, SiteProtocol::OpenaiCompatible, Some("model-a")).unwrap_err();
        assert!(err.to_string().contains("provider value"));
        row.extended.max = Some("max".into());
        validate_preset(&row, SiteProtocol::OpenaiCompatible, Some("model-a")).unwrap();
    }

    #[test]
    fn prime_anthropic_rejects_extended() {
        let mut row = preset(TargetKind::Prime);
        row.default_level = Some("xhigh".into());
        row.extended.xhigh = Some("xhigh".into());
        let err = validate_preset(&row, SiteProtocol::Anthropic, Some("model-a")).unwrap_err();
        assert!(err.to_string().contains("Prime Anthropic"));
    }

    #[test]
    fn pi_anthropic_extended_needs_adaptive() {
        let mut row = preset(TargetKind::Pi);
        row.default_level = Some("xhigh".into());
        row.extended.xhigh = Some("xhigh".into());
        let err = validate_preset(&row, SiteProtocol::Anthropic, Some("model-a")).unwrap_err();
        assert!(err.to_string().contains("adaptive"));
        row.models
            .get_mut("model-a")
            .unwrap()
            .force_adaptive_thinking = true;
        validate_preset(&row, SiteProtocol::Anthropic, Some("model-a")).unwrap();
    }

    #[test]
    fn normalize_drops_blank_extended_values() {
        let mut row = SiteThinkingPreset::empty("s1", TargetKind::Pi);
        row.default_level = Some(" HIGH ".into());
        row.extended.xhigh = Some("  ".into());
        row.extended.max = Some("max".into());
        let row = normalize_preset(row);
        assert_eq!(row.default_level.as_deref(), Some("high"));
        assert!(row.extended.xhigh.is_none());
        assert_eq!(row.extended.max.as_deref(), Some("max"));
    }
}
