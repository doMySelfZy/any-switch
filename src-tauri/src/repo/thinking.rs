use crate::domain::{normalize_preset, SiteThinkingPreset, TargetKind, ThinkingWrite};
use crate::error::{AppError, AppResult};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};

pub fn get_preset(
    conn: &Connection,
    site_id: &str,
    target: TargetKind,
) -> AppResult<SiteThinkingPreset> {
    let json: Option<String> = conn
        .query_row(
            "SELECT json FROM site_thinking_presets WHERE site_id = ?1 AND target = ?2",
            params![site_id, target.as_str()],
            |row| row.get(0),
        )
        .optional()?;
    match json {
        Some(json) => parse_stored(site_id, target, &json),
        None => Ok(SiteThinkingPreset::empty(site_id, target)),
    }
}

pub fn get_write(conn: &Connection, site_id: &str, target: TargetKind) -> AppResult<ThinkingWrite> {
    Ok(get_preset(conn, site_id, target)?.to_write())
}

pub fn save_preset(conn: &Connection, preset: &SiteThinkingPreset) -> AppResult<()> {
    let json = serde_json::to_string(preset)?;
    let now = Utc::now().timestamp_millis();
    conn.execute(
        "INSERT INTO site_thinking_presets (site_id, target, json, updated_at)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(site_id, target) DO UPDATE SET json = excluded.json, updated_at = excluded.updated_at",
        params![preset.site_id, preset.target.as_str(), json, now],
    )?;
    Ok(())
}

fn parse_stored(site_id: &str, target: TargetKind, json: &str) -> AppResult<SiteThinkingPreset> {
    let mut preset: SiteThinkingPreset = serde_json::from_str(json).map_err(|error| {
        AppError::new(
            "invalid_config",
            format!("invalid thinking preset: {error}"),
        )
    })?;
    preset.site_id = site_id.to_string();
    preset.target = target;
    Ok(normalize_preset(preset))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ModelThinkingConfig;
    use rusqlite::Connection;

    fn setup() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::apply_schema(&conn).unwrap();
        conn.execute(
            "INSERT INTO sites (id, name, base_url, protocol, claude_auth_key_style, notes, enabled, sort_order, created_at, updated_at)
             VALUES ('s1', 'T', 'https://api.example.com', 'openai_compatible', 'anthropic_auth_token', NULL, 1, 0, 1, 1)",
            [],
        )
        .unwrap();
        conn
    }

    #[test]
    fn missing_preset_returns_empty() {
        let conn = setup();
        let preset = get_preset(&conn, "s1", TargetKind::Pi).unwrap();
        assert!(preset.default_level.is_none());
        assert!(preset.models.is_empty());
    }

    #[test]
    fn round_trips_and_survives_site_delete() {
        let conn = setup();
        let mut preset = SiteThinkingPreset::empty("s1", TargetKind::Pi);
        preset.default_level = Some("medium".into());
        preset.models.insert(
            "model-a".into(),
            ModelThinkingConfig {
                reasoning: true,
                ..Default::default()
            },
        );
        save_preset(&conn, &preset).unwrap();
        let loaded = get_preset(&conn, "s1", TargetKind::Pi).unwrap();
        assert_eq!(loaded.default_level.as_deref(), Some("medium"));
        assert!(loaded.models["model-a"].reasoning);

        conn.execute("DELETE FROM sites WHERE id = 's1'", [])
            .unwrap();
        let n: i64 = conn
            .query_row("SELECT COUNT(*) FROM site_thinking_presets", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(n, 0);
    }

    #[test]
    fn pi_and_prime_presets_are_independent() {
        let conn = setup();
        let mut pi = SiteThinkingPreset::empty("s1", TargetKind::Pi);
        pi.default_level = Some("low".into());
        let mut prime = SiteThinkingPreset::empty("s1", TargetKind::Prime);
        prime.default_level = Some("high".into());
        save_preset(&conn, &pi).unwrap();
        save_preset(&conn, &prime).unwrap();
        assert_eq!(
            get_preset(&conn, "s1", TargetKind::Pi)
                .unwrap()
                .default_level
                .as_deref(),
            Some("low")
        );
        assert_eq!(
            get_preset(&conn, "s1", TargetKind::Prime)
                .unwrap()
                .default_level
                .as_deref(),
            Some("high")
        );
    }
}
