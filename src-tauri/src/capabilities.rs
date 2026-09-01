use serde_json::Value;
use std::collections::HashMap;

pub const CODEX_COMPACT: &str = "codex-compact";
pub const CODEX_VISION: &str = "codex-vision";
pub const CODEX_IMAGEGEN: &str = "codex-imagegen";
pub const CODEX_SEARCH: &str = "codex-search";

pub const CODEX_CAPABILITY_KEYS: [&str; 4] =
    [CODEX_COMPACT, CODEX_VISION, CODEX_IMAGEGEN, CODEX_SEARCH];

pub type SiteCapabilities = HashMap<String, bool>;

pub fn parse_capability_flag(raw: &str) -> bool {
    matches!(
        raw.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "on" | "yes"
    )
}

pub fn is_codex_capability_key(key: &str) -> bool {
    CODEX_CAPABILITY_KEYS.contains(&key)
}

pub fn capability_on(caps: &SiteCapabilities, key: &str) -> bool {
    caps.get(key).copied().unwrap_or(false)
}

pub fn merge_codex_capabilities(
    existing: &SiteCapabilities,
    incoming: &SiteCapabilities,
) -> SiteCapabilities {
    let mut next = SiteCapabilities::new();
    for (key, value) in existing {
        if !is_codex_capability_key(key) {
            next.insert(key.clone(), *value);
        }
    }
    for (key, value) in incoming {
        if !is_codex_capability_key(key) {
            next.insert(key.clone(), *value);
        }
    }
    for key in CODEX_CAPABILITY_KEYS {
        next.insert(key.to_string(), capability_on(incoming, key));
    }
    next
}

pub fn capabilities_equal(a: &SiteCapabilities, b: &SiteCapabilities) -> bool {
    let mut keys: Vec<&String> = a.keys().chain(b.keys()).collect();
    keys.sort();
    keys.dedup();
    keys.into_iter()
        .all(|key| capability_on(a, key) == capability_on(b, key))
}

pub fn parse_capabilities_json(raw: Option<&str>) -> SiteCapabilities {
    let Some(text) = raw.map(str::trim).filter(|s| !s.is_empty()) else {
        return SiteCapabilities::new();
    };
    let Ok(Value::Object(map)) = serde_json::from_str::<Value>(text) else {
        return SiteCapabilities::new();
    };
    let mut out = SiteCapabilities::new();
    for (key, value) in map {
        let flag = match value {
            Value::Bool(b) => b,
            Value::String(s) => parse_capability_flag(&s),
            Value::Number(n) => n.as_i64() == Some(1),
            _ => continue,
        };
        out.insert(key, flag);
    }
    out
}

pub fn capabilities_json(caps: &SiteCapabilities) -> Result<String, serde_json::Error> {
    serde_json::to_string(caps)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_replaces_known_codex_keys() {
        let mut existing = SiteCapabilities::new();
        existing.insert(CODEX_SEARCH.into(), true);
        existing.insert("claude-foo".into(), true);
        let mut incoming = SiteCapabilities::new();
        incoming.insert(CODEX_COMPACT.into(), true);
        let merged = merge_codex_capabilities(&existing, &incoming);
        assert!(capability_on(&merged, CODEX_COMPACT));
        assert!(!capability_on(&merged, CODEX_SEARCH));
        assert!(capability_on(&merged, "claude-foo"));
    }

    #[test]
    fn json_round_trip_keeps_unknown_keys() {
        let raw = r#"{"codex-compact":true,"claude-foo":true}"#;
        let parsed = parse_capabilities_json(Some(raw));
        assert!(capability_on(&parsed, CODEX_COMPACT));
        assert!(capability_on(&parsed, "claude-foo"));
        assert!(!capability_on(&parsed, CODEX_VISION));
    }
}
