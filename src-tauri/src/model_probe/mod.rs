use crate::crypto::key_prefix;
use crate::domain::{AppSettings, ModelProbeResult, SiteProtocol, SiteRow};
use crate::error::AppResult;
use crate::url_normalize::{normalize_base_url, UrlWritePreview};
use serde_json::{json, Value};
use std::time::{Duration, Instant};

const PROBE_TIMEOUT: Duration = Duration::from_secs(20);
const MAX_BODY_BYTES: usize = 64 * 1024;
const PROBE_MAX_TOKENS: u64 = 16;
const PROBE_USER_TEXT: &str = "Hi";

pub fn probe_endpoint(protocol: &SiteProtocol, preview: &UrlWritePreview) -> String {
    match protocol {
        SiteProtocol::OpenaiCompatible => {
            format!("{}/chat/completions", preview.codex_base_url)
        }
        SiteProtocol::Anthropic => {
            if preview
                .claude_base_url
                .to_ascii_lowercase()
                .ends_with("/v1")
            {
                format!("{}/messages", preview.claude_base_url)
            } else {
                format!("{}/v1/messages", preview.claude_base_url)
            }
        }
    }
}

pub fn probe_body(protocol: &SiteProtocol, model_id: &str) -> Value {
    match protocol {
        SiteProtocol::OpenaiCompatible => json!({
            "model": model_id,
            "messages": [{ "role": "user", "content": PROBE_USER_TEXT }],
            "max_tokens": PROBE_MAX_TOKENS,
            "stream": false
        }),
        SiteProtocol::Anthropic => json!({
            "model": model_id,
            "max_tokens": PROBE_MAX_TOKENS,
            "messages": [{ "role": "user", "content": PROBE_USER_TEXT }]
        }),
    }
}

pub fn apply_headers(
    req: reqwest::RequestBuilder,
    protocol: &SiteProtocol,
    api_key: &str,
) -> reqwest::RequestBuilder {
    match protocol {
        SiteProtocol::OpenaiCompatible => req.bearer_auth(api_key),
        SiteProtocol::Anthropic => req
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01"),
    }
}

pub fn interpret_response(protocol: &SiteProtocol, status: u16, body: &str) -> Result<(), String> {
    if !(200..300).contains(&status) {
        return Err(extract_error(status, body));
    }
    let value: Value = serde_json::from_str(body).map_err(|_| "invalid response".to_string())?;
    if response_ok(protocol, &value) {
        Ok(())
    } else {
        Err("invalid response".into())
    }
}

pub fn sanitize_error(raw: &str, api_key: &str) -> String {
    let key = api_key.trim();
    if key.is_empty() || !raw.contains(key) {
        return raw.to_string();
    }
    raw.replace(key, &key_prefix(key))
}

pub async fn probe_model(
    site: &SiteRow,
    api_key: &str,
    model_id: &str,
    settings: &AppSettings,
) -> AppResult<ModelProbeResult> {
    let preview = normalize_base_url(&site.base_url)?;
    let endpoint = probe_endpoint(&site.protocol, &preview);
    let client = crate::http_client::build_client(settings, PROBE_TIMEOUT)?;
    let start = Instant::now();

    let mut body = probe_body(&site.protocol, model_id);
    let mut retried_completion_tokens = false;

    loop {
        let request = apply_headers(
            client
                .post(&endpoint)
                .header("Content-Type", "application/json"),
            &site.protocol,
            api_key,
        )
        .json(&body);

        let outcome = match request.send().await {
            Ok(resp) => {
                let status = resp.status().as_u16();
                let bytes = resp.bytes().await.unwrap_or_default();
                let slice = if bytes.len() > MAX_BODY_BYTES {
                    &bytes[..MAX_BODY_BYTES]
                } else {
                    &bytes
                };
                let text = String::from_utf8_lossy(slice).into_owned();
                (
                    Some(status),
                    interpret_response(&site.protocol, status, &text),
                )
            }
            Err(e) => (None, Err(map_reqwest_error(&e))),
        };

        match outcome {
            (status, Ok(())) => {
                return Ok(ModelProbeResult {
                    model_id: model_id.to_string(),
                    ok: true,
                    latency_ms: elapsed_ms(start),
                    status,
                    error: None,
                    endpoint,
                });
            }
            (status, Err(msg)) => {
                let should_retry = !retried_completion_tokens
                    && matches!(site.protocol, SiteProtocol::OpenaiCompatible)
                    && status == Some(400)
                    && msg.contains("max_completion_tokens");
                if should_retry {
                    retried_completion_tokens = true;
                    swap_max_tokens_field(&mut body);
                    continue;
                }
                return Ok(ModelProbeResult {
                    model_id: model_id.to_string(),
                    ok: false,
                    latency_ms: elapsed_ms(start),
                    status,
                    error: Some(sanitize_error(&msg, api_key)),
                    endpoint,
                });
            }
        }
    }
}

fn response_ok(protocol: &SiteProtocol, value: &Value) -> bool {
    match protocol {
        SiteProtocol::OpenaiCompatible => value
            .get("choices")
            .and_then(Value::as_array)
            .is_some_and(|choices| !choices.is_empty()),
        SiteProtocol::Anthropic => {
            value.get("type").and_then(Value::as_str) == Some("message")
                || value.get("content").is_some_and(Value::is_array)
        }
    }
}

fn extract_error(status: u16, body: &str) -> String {
    if let Ok(value) = serde_json::from_str::<Value>(body) {
        if let Some(msg) = nested_error_message(&value) {
            return msg;
        }
    }
    format!("HTTP {status}")
}

fn nested_error_message(value: &Value) -> Option<String> {
    if let Some(msg) = value
        .pointer("/error/message")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        return Some(msg.to_string());
    }
    if let Some(msg) = value
        .get("error")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        return Some(msg.to_string());
    }
    if let Some(msg) = value
        .get("message")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        return Some(msg.to_string());
    }
    None
}

fn swap_max_tokens_field(body: &mut Value) {
    if let Some(obj) = body.as_object_mut() {
        if let Some(tokens) = obj.remove("max_tokens") {
            obj.insert("max_completion_tokens".into(), tokens);
        }
    }
}

fn map_reqwest_error(err: &reqwest::Error) -> String {
    if err.is_timeout() {
        return "request timed out".into();
    }
    err.to_string()
}

fn elapsed_ms(start: Instant) -> u64 {
    start.elapsed().as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::url_normalize::normalize_base_url;

    #[test]
    fn openai_chat_url_from_bare_and_v1() {
        let bare = normalize_base_url("https://api.example.com").unwrap();
        assert_eq!(
            probe_endpoint(&SiteProtocol::OpenaiCompatible, &bare),
            "https://api.example.com/v1/chat/completions"
        );

        let with_v1 = normalize_base_url("https://api.example.com/v1").unwrap();
        assert_eq!(
            probe_endpoint(&SiteProtocol::OpenaiCompatible, &with_v1),
            "https://api.example.com/v1/chat/completions"
        );

        let anthropic_path = normalize_base_url("https://relay.example.com/anthropic").unwrap();
        assert_eq!(
            probe_endpoint(&SiteProtocol::OpenaiCompatible, &anthropic_path),
            "https://relay.example.com/anthropic/v1/chat/completions"
        );
    }

    #[test]
    fn anthropic_messages_url_from_bare_v1_and_anthropic_path() {
        let bare = normalize_base_url("https://api.anthropic.com").unwrap();
        assert_eq!(
            probe_endpoint(&SiteProtocol::Anthropic, &bare),
            "https://api.anthropic.com/v1/messages"
        );

        let with_v1 = normalize_base_url("https://relay.example.com/v1").unwrap();
        assert_eq!(
            probe_endpoint(&SiteProtocol::Anthropic, &with_v1),
            "https://relay.example.com/v1/messages"
        );

        let path = normalize_base_url("https://relay.example.com/anthropic").unwrap();
        assert_eq!(
            probe_endpoint(&SiteProtocol::Anthropic, &path),
            "https://relay.example.com/anthropic/v1/messages"
        );
    }

    #[test]
    fn openai_body_is_hi_chat() {
        let body = probe_body(&SiteProtocol::OpenaiCompatible, "gpt-4.1");
        assert_eq!(body["model"], "gpt-4.1");
        assert_eq!(body["messages"][0]["role"], "user");
        assert_eq!(body["messages"][0]["content"], "Hi");
        assert_eq!(body["max_tokens"], 16);
        assert_eq!(body["stream"], false);
    }

    #[test]
    fn openai_success_requires_non_empty_choices() {
        assert!(interpret_response(
            &SiteProtocol::OpenaiCompatible,
            200,
            r#"{"choices":[{"message":{"content":"hello"}}]}"#
        )
        .is_ok());
        assert!(interpret_response(
            &SiteProtocol::OpenaiCompatible,
            200,
            r#"{"choices":[{"message":{"content":""}}]}"#
        )
        .is_ok());
        assert_eq!(
            interpret_response(&SiteProtocol::OpenaiCompatible, 200, r#"{"choices":[]}"#)
                .unwrap_err(),
            "invalid response"
        );
        assert_eq!(
            interpret_response(&SiteProtocol::OpenaiCompatible, 200, "not-json").unwrap_err(),
            "invalid response"
        );
    }

    #[test]
    fn anthropic_success_accepts_message_type_or_content_array() {
        assert!(interpret_response(
            &SiteProtocol::Anthropic,
            200,
            r#"{"type":"message","content":[]}"#
        )
        .is_ok());
        assert!(interpret_response(
            &SiteProtocol::Anthropic,
            200,
            r#"{"content":[{"type":"text","text":"hi"}]}"#
        )
        .is_ok());
        assert_eq!(
            interpret_response(&SiteProtocol::Anthropic, 200, r#"{"type":"error"}"#).unwrap_err(),
            "invalid response"
        );
    }

    #[test]
    fn extract_error_prefers_error_message() {
        assert_eq!(
            interpret_response(
                &SiteProtocol::OpenaiCompatible,
                429,
                r#"{"error":{"message":"rate limited"}}"#
            )
            .unwrap_err(),
            "rate limited"
        );
        assert_eq!(
            interpret_response(&SiteProtocol::Anthropic, 401, r#"{"error":"bad key"}"#)
                .unwrap_err(),
            "bad key"
        );
        assert_eq!(
            interpret_response(
                &SiteProtocol::OpenaiCompatible,
                500,
                r#"{"message":"boom"}"#
            )
            .unwrap_err(),
            "boom"
        );
        assert_eq!(
            interpret_response(&SiteProtocol::OpenaiCompatible, 502, "upstream down").unwrap_err(),
            "HTTP 502"
        );
    }

    #[test]
    fn redact_replaces_raw_api_key_in_error() {
        let key = "sk-abcdefghijklmnop";
        let redacted = sanitize_error(&format!("invalid token {key} used"), key);
        assert!(!redacted.contains(key));
        assert!(redacted.contains(&key_prefix(key)));
        assert_eq!(sanitize_error("plain error", key), "plain error");
    }

    #[test]
    fn swap_max_tokens_prepares_completion_tokens_retry() {
        let mut body = probe_body(&SiteProtocol::OpenaiCompatible, "o3");
        swap_max_tokens_field(&mut body);
        assert!(body.get("max_tokens").is_none());
        assert_eq!(body["max_completion_tokens"], 16);
    }
}
