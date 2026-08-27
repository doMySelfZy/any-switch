use crate::capabilities::{capabilities_json, parse_capabilities_json};
use crate::crypto::{key_prefix, Crypto};
use crate::domain::{
    ClaudeAuthKeyStyle, CreateSiteInput, SiteModelDto, SiteProtocol, SiteRow, UpdateSiteInput,
};
use crate::error::{AppError, AppResult};
use crate::url_normalize::{move_url_to_front, normalize_base_urls, parse_base_urls_json};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use uuid::Uuid;

fn map_site(row: &rusqlite::Row<'_>) -> rusqlite::Result<SiteRow> {
    let base_url: String = row.get(2)?;
    let base_urls_json: Option<String> = row.get(16).ok().flatten();
    let capabilities_json: Option<String> = row.get(17).ok().flatten();
    let base_urls = parse_base_urls_json(base_urls_json.as_deref(), &base_url);
    let capabilities = parse_capabilities_json(capabilities_json.as_deref());
    let active = base_urls
        .first()
        .cloned()
        .unwrap_or_else(|| base_url.clone());
    Ok(SiteRow {
        id: row.get(0)?,
        name: row.get(1)?,
        base_url: active,
        base_urls,
        api_key_encrypted: row.get(3)?,
        key_prefix: row.get(4)?,
        protocol: SiteProtocol::parse(&row.get::<_, String>(5)?),
        claude_auth_key_style: ClaudeAuthKeyStyle::parse(&row.get::<_, String>(6)?),
        notes: row.get(7)?,
        enabled: row.get::<_, i64>(8)? != 0,
        sort_order: row.get(9)?,
        selected_model_id: row.get(10)?,
        last_model_fetch_at: row.get(11)?,
        last_model_fetch_latency_ms: row.get(12)?,
        last_model_fetch_error: row.get(13)?,
        created_at: row.get(14)?,
        updated_at: row.get(15)?,
        capabilities,
    })
}

fn urls_json(urls: &[String]) -> AppResult<String> {
    Ok(serde_json::to_string(urls)?)
}

const SITE_COLS: &str = "id, name, base_url, api_key_encrypted, key_prefix, protocol, claude_auth_key_style, notes, enabled, sort_order, selected_model_id, last_model_fetch_at, last_model_fetch_latency_ms, last_model_fetch_error, created_at, updated_at, base_urls_json, capabilities_json";

pub fn list_sites(conn: &Connection) -> AppResult<Vec<SiteRow>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {SITE_COLS} FROM sites ORDER BY sort_order ASC, created_at ASC"
    ))?;
    let rows = stmt.query_map([], map_site)?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

pub fn get_site(conn: &Connection, id: &str) -> AppResult<SiteRow> {
    let mut stmt = conn.prepare(&format!("SELECT {SITE_COLS} FROM sites WHERE id = ?1"))?;
    stmt.query_row(params![id], map_site)
        .optional()?
        .ok_or_else(|| AppError::new("not_found", "site not found"))
}

pub fn get_site_api_key(conn: &Connection, crypto: &Crypto, id: &str) -> AppResult<String> {
    let site = get_site(conn, id)?;
    if site.api_key_encrypted.is_empty() {
        return Ok(String::new());
    }
    crypto.decrypt(&site.api_key_encrypted)
}

pub fn create_site(
    conn: &Connection,
    crypto: &Crypto,
    input: CreateSiteInput,
) -> AppResult<SiteRow> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().timestamp_millis();
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM sites", [], |r| r.get(0))?;
    let enc = crypto.encrypt(&input.api_key)?;
    let prefix = key_prefix(&input.api_key);
    let protocol = input
        .protocol
        .as_deref()
        .map(SiteProtocol::parse)
        .unwrap_or(SiteProtocol::OpenaiCompatible);
    let auth = input
        .claude_auth_key_style
        .as_deref()
        .map(ClaudeAuthKeyStyle::parse)
        .unwrap_or(ClaudeAuthKeyStyle::AnthropicAuthToken);

    let urls = if let Some(list) = input.base_urls.filter(|v| !v.is_empty()) {
        normalize_base_urls(&list)?
    } else {
        normalize_base_urls(&[input.base_url])?
    };
    let base_url = urls[0].clone();
    let urls_json = urls_json(&urls)?;
    let capabilities = input.capabilities.unwrap_or_default();
    let caps_json = capabilities_json(&capabilities)?;

    conn.execute(
        "INSERT INTO sites (id, name, base_url, api_key_encrypted, key_prefix, protocol, claude_auth_key_style, notes, enabled, sort_order, selected_model_id, last_model_fetch_at, last_model_fetch_latency_ms, last_model_fetch_error, created_at, updated_at, base_urls_json, capabilities_json)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,1,?9,NULL,NULL,NULL,NULL,?10,?10,?11,?12)",
        params![
            id,
            input.name,
            base_url,
            enc,
            prefix,
            protocol.as_str(),
            auth.as_str(),
            input.notes,
            count,
            now,
            urls_json,
            caps_json
        ],
    )?;
    get_site(conn, &id)
}

pub fn update_site(
    conn: &Connection,
    crypto: &Crypto,
    id: &str,
    input: UpdateSiteInput,
) -> AppResult<SiteRow> {
    let mut site = get_site(conn, id)?;
    if let Some(name) = input.name {
        site.name = name;
    }
    if let Some(list) = input.base_urls {
        let urls = normalize_base_urls(&list)?;
        site.base_urls = urls;
        site.base_url = site.base_urls[0].clone();
    } else if let Some(base_url) = input.base_url {
        let selected = normalize_base_urls(&[base_url])?[0].clone();
        if site.base_urls.iter().any(|u| u == &selected) {
            site.base_urls = move_url_to_front(&site.base_urls, &selected)?;
        } else if site.base_urls.is_empty() {
            site.base_urls = vec![selected.clone()];
        } else {
            site.base_urls[0] = selected.clone();
        }
        site.base_url = selected;
    }
    if let Some(api_key) = input.api_key {
        if !api_key.is_empty() {
            site.api_key_encrypted = crypto.encrypt(&api_key)?;
            site.key_prefix = key_prefix(&api_key);
        }
    }
    if let Some(p) = input.protocol {
        site.protocol = SiteProtocol::parse(&p);
    }
    if let Some(a) = input.claude_auth_key_style {
        site.claude_auth_key_style = ClaudeAuthKeyStyle::parse(&a);
    }
    if input.notes.is_some() {
        site.notes = input.notes;
    }
    if let Some(e) = input.enabled {
        site.enabled = e;
    }
    if input.selected_model_id.is_some() {
        site.selected_model_id = input.selected_model_id;
    }
    if let Some(o) = input.sort_order {
        site.sort_order = o;
    }
    if let Some(capabilities) = input.capabilities {
        site.capabilities = capabilities;
    }
    site.updated_at = Utc::now().timestamp_millis();

    persist_site(conn, &site)?;
    Ok(site)
}

fn persist_site(conn: &Connection, site: &SiteRow) -> AppResult<()> {
    conn.execute(
        "UPDATE sites SET name=?2, base_url=?3, api_key_encrypted=?4, key_prefix=?5, protocol=?6, claude_auth_key_style=?7, notes=?8, enabled=?9, sort_order=?10, selected_model_id=?11, updated_at=?12, base_urls_json=?13, capabilities_json=?14 WHERE id=?1",
        params![
            site.id,
            site.name,
            site.base_url,
            site.api_key_encrypted,
            site.key_prefix,
            site.protocol.as_str(),
            site.claude_auth_key_style.as_str(),
            site.notes,
            site.enabled as i64,
            site.sort_order,
            site.selected_model_id,
            site.updated_at,
            urls_json(&site.base_urls)?,
            capabilities_json(&site.capabilities)?
        ],
    )?;
    Ok(())
}

pub fn switch_site_route(conn: &Connection, id: &str, base_url: &str) -> AppResult<SiteRow> {
    let mut site = get_site(conn, id)?;
    let next = move_url_to_front(&site.base_urls, base_url)?;
    if next == site.base_urls && site.base_url == next[0] {
        return Ok(site);
    }
    site.base_urls = next;
    site.base_url = site.base_urls[0].clone();
    site.updated_at = Utc::now().timestamp_millis();
    persist_site(conn, &site)?;
    Ok(site)
}

pub fn delete_site(conn: &Connection, id: &str) -> AppResult<()> {
    let n = conn.execute("DELETE FROM sites WHERE id = ?1", params![id])?;
    if n == 0 {
        return Err(AppError::new("not_found", "site not found"));
    }
    Ok(())
}

pub fn set_selected_model(conn: &Connection, site_id: &str, model_id: &str) -> AppResult<()> {
    let now = Utc::now().timestamp_millis();
    let n = conn.execute(
        "UPDATE sites SET selected_model_id = ?2, updated_at = ?3 WHERE id = ?1",
        params![site_id, model_id, now],
    )?;
    if n == 0 {
        return Err(AppError::new("not_found", "site not found"));
    }
    clear_model_exclusion(conn, site_id, model_id)?;
    // ensure placeholder model cache row
    let exists: bool = conn
        .query_row(
            "SELECT 1 FROM site_models WHERE site_id = ?1 AND model_id = ?2",
            params![site_id, model_id],
            |_| Ok(true),
        )
        .optional()?
        .unwrap_or(false);
    if !exists {
        conn.execute(
            "INSERT INTO site_models (id, site_id, model_id, display_name, owned_by, raw_json, is_manual) VALUES (?1,?2,?3,?3,NULL,NULL,1)",
            params![Uuid::new_v4().to_string(), site_id, model_id],
        )?;
    }
    Ok(())
}

fn insert_site_model(
    conn: &Connection,
    site_id: &str,
    m: &SiteModelDto,
    is_manual: bool,
) -> AppResult<()> {
    conn.execute(
        "INSERT INTO site_models (id, site_id, model_id, display_name, owned_by, raw_json, is_manual) VALUES (?1,?2,?3,?4,?5,?6,?7)",
        params![
            m.id,
            site_id,
            m.model_id,
            m.display_name,
            m.owned_by,
            m.raw.as_ref().map(|v| v.to_string()),
            is_manual as i64
        ],
    )?;
    Ok(())
}

pub fn replace_models(conn: &Connection, site_id: &str, models: &[SiteModelDto]) -> AppResult<()> {
    let existing = list_models(conn, site_id)?;
    let excluded = list_exclusions(conn, site_id)?;
    let fetched_ids: std::collections::HashSet<&str> =
        models.iter().map(|m| m.model_id.as_str()).collect();
    let manuals_to_keep: Vec<SiteModelDto> = existing
        .into_iter()
        .filter(|m| m.is_manual && !fetched_ids.contains(m.model_id.as_str()))
        .collect();

    conn.execute(
        "DELETE FROM site_models WHERE site_id = ?1",
        params![site_id],
    )?;
    for m in models {
        if excluded.contains(&m.model_id) {
            continue;
        }
        insert_site_model(conn, site_id, m, false)?;
    }
    for m in &manuals_to_keep {
        insert_site_model(conn, site_id, m, true)?;
    }
    Ok(())
}

fn clear_model_exclusion(conn: &Connection, site_id: &str, model_id: &str) -> AppResult<()> {
    conn.execute(
        "DELETE FROM site_model_exclusions WHERE site_id = ?1 AND model_id = ?2",
        params![site_id, model_id],
    )?;
    Ok(())
}

fn exclude_model(conn: &Connection, site_id: &str, model_id: &str) -> AppResult<()> {
    conn.execute(
        "INSERT OR IGNORE INTO site_model_exclusions (site_id, model_id) VALUES (?1, ?2)",
        params![site_id, model_id],
    )?;
    Ok(())
}

fn list_exclusions(
    conn: &Connection,
    site_id: &str,
) -> AppResult<std::collections::HashSet<String>> {
    let mut stmt = conn.prepare("SELECT model_id FROM site_model_exclusions WHERE site_id = ?1")?;
    let rows = stmt.query_map(params![site_id], |row| row.get::<_, String>(0))?;
    let mut out = std::collections::HashSet::new();
    for r in rows {
        out.insert(r?);
    }
    Ok(out)
}

pub fn clear_models(conn: &Connection, site_id: &str) -> AppResult<()> {
    conn.execute(
        "DELETE FROM site_models WHERE site_id = ?1",
        params![site_id],
    )?;
    let now = Utc::now().timestamp_millis();
    let n = conn.execute(
        "UPDATE sites SET selected_model_id = NULL, updated_at = ?2 WHERE id = ?1",
        params![site_id, now],
    )?;
    if n == 0 {
        return Err(AppError::new("not_found", "site not found"));
    }
    Ok(())
}

pub fn delete_model(conn: &Connection, site_id: &str, model_id: &str) -> AppResult<()> {
    let n = conn.execute(
        "DELETE FROM site_models WHERE site_id = ?1 AND model_id = ?2",
        params![site_id, model_id],
    )?;
    if n == 0 {
        return Err(AppError::new("not_found", "model not found"));
    }
    exclude_model(conn, site_id, model_id)?;

    let site = get_site(conn, site_id)?;
    if site.selected_model_id.as_deref() == Some(model_id) {
        let remaining = list_models(conn, site_id)?;
        let next = remaining.first().map(|m| m.model_id.as_str());
        let now = Utc::now().timestamp_millis();
        conn.execute(
            "UPDATE sites SET selected_model_id = ?2, updated_at = ?3 WHERE id = ?1",
            params![site_id, next, now],
        )?;
    }
    Ok(())
}

pub fn list_models(conn: &Connection, site_id: &str) -> AppResult<Vec<SiteModelDto>> {
    let mut stmt = conn.prepare(
        "SELECT id, site_id, model_id, display_name, owned_by, raw_json, is_manual FROM site_models WHERE site_id = ?1 ORDER BY model_id",
    )?;
    let rows = stmt.query_map(params![site_id], |row| {
        let raw_json: Option<String> = row.get(5)?;
        Ok(SiteModelDto {
            id: row.get(0)?,
            site_id: row.get(1)?,
            model_id: row.get(2)?,
            display_name: row.get(3)?,
            owned_by: row.get(4)?,
            raw: raw_json.and_then(|s| serde_json::from_str(&s).ok()),
            is_manual: row.get::<_, i64>(6)? != 0,
        })
    })?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

pub fn update_fetch_meta(
    conn: &Connection,
    site_id: &str,
    latency_ms: i64,
    error: Option<&str>,
) -> AppResult<()> {
    let now = Utc::now().timestamp_millis();
    conn.execute(
        "UPDATE sites SET last_model_fetch_at=?2, last_model_fetch_latency_ms=?3, last_model_fetch_error=?4, updated_at=?2 WHERE id=?1",
        params![site_id, now, latency_ms, error],
    )?;
    Ok(())
}

pub fn has_encrypted_sites(conn: &Connection) -> AppResult<bool> {
    let n: i64 = conn.query_row("SELECT COUNT(*) FROM sites", [], |r| r.get(0))?;
    Ok(n > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::apply_schema(&conn).unwrap();
        conn.execute(
            "INSERT INTO sites (id, name, base_url, api_key_encrypted, key_prefix, protocol, claude_auth_key_style, notes, enabled, sort_order, selected_model_id, last_model_fetch_at, last_model_fetch_latency_ms, last_model_fetch_error, created_at, updated_at)
             VALUES ('s1', 'T', 'https://api.example.com', 'x', 'sk-xx', 'openai_compatible', 'anthropic_auth_token', NULL, 1, 0, NULL, NULL, NULL, NULL, 1, 1)",
            [],
        )
        .unwrap();
        conn
    }

    fn fetched(model_id: &str) -> SiteModelDto {
        SiteModelDto {
            id: format!("fetched-{model_id}"),
            site_id: "s1".into(),
            model_id: model_id.into(),
            display_name: model_id.into(),
            owned_by: Some("openai".into()),
            raw: None,
            is_manual: false,
        }
    }

    fn ids(conn: &Connection) -> Vec<String> {
        let mut out: Vec<String> = list_models(conn, "s1")
            .unwrap()
            .into_iter()
            .map(|m| m.model_id)
            .collect();
        out.sort();
        out
    }

    #[test]
    fn replace_models_keeps_manually_added_model() {
        let conn = setup();
        set_selected_model(&conn, "s1", "gpt-5.6-terra").unwrap();

        replace_models(&conn, "s1", &[fetched("gpt-4.1")]).unwrap();

        assert_eq!(
            ids(&conn),
            vec!["gpt-4.1".to_string(), "gpt-5.6-terra".to_string()]
        );
    }

    #[test]
    fn replace_models_drops_stale_fetched_models() {
        let conn = setup();
        replace_models(&conn, "s1", &[fetched("gpt-4.1"), fetched("old-model")]).unwrap();
        replace_models(&conn, "s1", &[fetched("gpt-4.1")]).unwrap();

        assert_eq!(ids(&conn), vec!["gpt-4.1".to_string()]);
    }

    #[test]
    fn replace_models_dedupes_when_manual_id_appears_in_fetch() {
        let conn = setup();
        set_selected_model(&conn, "s1", "gpt-4.1").unwrap();
        replace_models(&conn, "s1", &[fetched("gpt-4.1")]).unwrap();

        assert_eq!(ids(&conn), vec!["gpt-4.1".to_string()]);
    }

    #[test]
    fn delete_model_removes_it_from_the_list() {
        let conn = setup();
        replace_models(&conn, "s1", &[fetched("gpt-4.1"), fetched("gpt-4.2")]).unwrap();
        delete_model(&conn, "s1", "gpt-4.1").unwrap();
        assert_eq!(ids(&conn), vec!["gpt-4.2".to_string()]);
    }

    #[test]
    fn delete_model_reassigns_selected_to_remaining() {
        let conn = setup();
        replace_models(&conn, "s1", &[fetched("gpt-4.1"), fetched("gpt-4.2")]).unwrap();
        set_selected_model(&conn, "s1", "gpt-4.1").unwrap();
        delete_model(&conn, "s1", "gpt-4.1").unwrap();
        let site = get_site(&conn, "s1").unwrap();
        assert_eq!(site.selected_model_id.as_deref(), Some("gpt-4.2"));
    }

    #[test]
    fn delete_model_clears_selected_when_last() {
        let conn = setup();
        replace_models(&conn, "s1", &[fetched("gpt-4.1")]).unwrap();
        set_selected_model(&conn, "s1", "gpt-4.1").unwrap();
        delete_model(&conn, "s1", "gpt-4.1").unwrap();
        let site = get_site(&conn, "s1").unwrap();
        assert_eq!(site.selected_model_id, None);
        assert!(ids(&conn).is_empty());
    }

    #[test]
    fn deleted_fetched_model_stays_gone_after_replace() {
        let conn = setup();
        replace_models(&conn, "s1", &[fetched("gpt-4.1"), fetched("gpt-4.2")]).unwrap();
        delete_model(&conn, "s1", "gpt-4.1").unwrap();
        replace_models(&conn, "s1", &[fetched("gpt-4.1"), fetched("gpt-4.2")]).unwrap();
        assert_eq!(ids(&conn), vec!["gpt-4.2".to_string()]);
    }

    #[test]
    fn set_selected_model_restores_a_deleted_id() {
        let conn = setup();
        replace_models(&conn, "s1", &[fetched("gpt-4.1")]).unwrap();
        delete_model(&conn, "s1", "gpt-4.1").unwrap();
        set_selected_model(&conn, "s1", "gpt-4.1").unwrap();
        assert_eq!(ids(&conn), vec!["gpt-4.1".to_string()]);
    }

    #[test]
    fn switch_site_route_moves_selected_to_front() {
        let conn = setup();
        conn.execute(
            "UPDATE sites SET base_url = 'https://a.example.com', base_urls_json = ?1 WHERE id = 's1'",
            params![r#"["https://a.example.com","https://b.example.com"]"#],
        )
        .unwrap();
        let site = switch_site_route(&conn, "s1", "https://b.example.com").unwrap();
        assert_eq!(site.base_url, "https://b.example.com");
        assert_eq!(
            site.base_urls,
            vec!["https://b.example.com", "https://a.example.com"]
        );
        assert!(switch_site_route(&conn, "s1", "https://missing").is_err());
    }

    #[test]
    fn clear_models_empties_list_without_excluding_fetch() {
        let conn = setup();
        replace_models(&conn, "s1", &[fetched("gpt-4.1"), fetched("gpt-4.2")]).unwrap();
        set_selected_model(&conn, "s1", "gpt-4.1").unwrap();
        clear_models(&conn, "s1").unwrap();
        assert!(ids(&conn).is_empty());
        assert_eq!(get_site(&conn, "s1").unwrap().selected_model_id, None);
        replace_models(&conn, "s1", &[fetched("gpt-4.1"), fetched("gpt-4.2")]).unwrap();
        assert_eq!(
            ids(&conn),
            vec!["gpt-4.1".to_string(), "gpt-4.2".to_string()]
        );
    }

    #[test]
    fn create_and_update_persist_capabilities() {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::apply_schema(&conn).unwrap();
        let crypto = crate::crypto::Crypto::from_key([9u8; 32]);
        let mut caps = std::collections::HashMap::new();
        caps.insert("codex-vision".into(), true);
        caps.insert("claude-foo".into(), true);
        let created = create_site(
            &conn,
            &crypto,
            CreateSiteInput {
                name: "Relay".into(),
                base_url: "https://a.example.com".into(),
                base_urls: None,
                api_key: "sk-test".into(),
                protocol: None,
                claude_auth_key_style: None,
                notes: None,
                capabilities: Some(caps),
            },
        )
        .unwrap();
        assert_eq!(created.capabilities.get("codex-vision"), Some(&true));
        assert_eq!(created.capabilities.get("claude-foo"), Some(&true));

        let mut next = std::collections::HashMap::new();
        next.insert("codex-search".into(), true);
        next.insert("claude-foo".into(), true);
        let updated = update_site(
            &conn,
            &crypto,
            &created.id,
            UpdateSiteInput {
                capabilities: Some(next),
                ..UpdateSiteInput::default()
            },
        )
        .unwrap();
        assert_eq!(updated.capabilities.get("codex-search"), Some(&true));
        assert_eq!(updated.capabilities.get("claude-foo"), Some(&true));
        assert_eq!(
            get_site(&conn, &created.id)
                .unwrap()
                .capabilities
                .get("codex-search"),
            Some(&true)
        );
    }

    #[test]
    fn get_site_api_key_returns_complete_decrypted_key() {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::apply_schema(&conn).unwrap();
        let crypto = crate::crypto::Crypto::from_key([7u8; 32]);
        let created = create_site(
            &conn,
            &crypto,
            CreateSiteInput {
                name: "Relay".into(),
                base_url: "https://a.example.com".into(),
                base_urls: None,
                api_key: "sk-full-secret".into(),
                protocol: None,
                claude_auth_key_style: None,
                notes: None,
                capabilities: None,
            },
        )
        .unwrap();

        assert_eq!(
            get_site_api_key(&conn, &crypto, &created.id).unwrap(),
            "sk-full-secret"
        );
    }
}
