use crate::domain::{AppSettings, FetchModelsResult, SiteModelDto, SiteProtocol, SiteRow};
use crate::error::{AppError, AppResult};
use crate::url_normalize::normalize_base_url;
use chrono::Utc;
use serde::Deserialize;
use std::time::Instant;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
struct OpenAiModelsResponse {
    data: Option<Vec<OpenAiModel>>,
}

#[derive(Debug, Deserialize)]
struct OpenAiModel {
    id: String,
    owned_by: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AnthropicModelsResponse {
    data: Option<Vec<AnthropicModel>>,
}

#[derive(Debug, Deserialize)]
struct AnthropicModel {
    id: String,
    display_name: Option<String>,
}

const CLAUDE_CODE_UA: &str = "claude-cli/2.0.14 (external, cli)";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AuthStyle {
    Bearer,
    XApiKey,
}

/// 拉取失败时的兜底：部分中转站（如 AgentRouter）做客户端指纹检测，
/// 只放行 Claude Code 官方客户端特征，标准请求会被 401 拒绝。
fn with_claude_code_headers(
    req: reqwest::RequestBuilder,
) -> reqwest::RequestBuilder {
    req.header("User-Agent", CLAUDE_CODE_UA)
        .header("x-app", "cli")
}

fn parse_models(text: &str) -> AppResult<Vec<SiteModelDto>> {
    if let Ok(body) = serde_json::from_str::<AnthropicModelsResponse>(text) {
        return Ok(body
            .data
            .unwrap_or_default()
            .into_iter()
            .map(|m| SiteModelDto {
                id: Uuid::new_v4().to_string(),
                site_id: String::new(),
                api_key_id: String::new(),
                model_id: m.id.clone(),
                display_name: m.display_name.unwrap_or(m.id),
                owned_by: Some("anthropic".into()),
                raw: None,
                is_manual: false,
            })
            .collect());
    }
    if let Ok(body) = serde_json::from_str::<OpenAiModelsResponse>(text) {
        return Ok(body
            .data
            .unwrap_or_default()
            .into_iter()
            .map(|m| SiteModelDto {
                id: Uuid::new_v4().to_string(),
                site_id: String::new(),
                api_key_id: String::new(),
                model_id: m.id.clone(),
                display_name: m.id,
                owned_by: m.owned_by,
                raw: None,
                is_manual: false,
            })
            .collect());
    }
    Err(AppError::new(
        "invalid_response",
        "Could not parse models response. Enter model id manually.",
    ))
}

async fn attempt_models(
    client: &reqwest::Client,
    endpoint: &str,
    api_key: &str,
    style: AuthStyle,
    impersonate: bool,
) -> AppResult<Vec<SiteModelDto>> {
    let mut req = client.get(endpoint);
    req = match style {
        AuthStyle::Bearer => req.bearer_auth(api_key),
        AuthStyle::XApiKey => req
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01"),
    };
    if impersonate {
        req = with_claude_code_headers(req);
    }
    let resp = req.send().await?;
    let status = resp.status();
    if status.as_u16() == 401 || status.as_u16() == 403 {
        return Err(AppError::new("unauthorized", "unauthorized"));
    }
    if status.as_u16() == 404 {
        return Err(AppError::new("not_found", "models endpoint not found"));
    }
    if !status.is_success() {
        return Err(AppError::new(
            "network",
            format!("HTTP {}", status.as_u16()),
        ));
    }
    let text = resp.text().await?;
    parse_models(&text)
}

pub async fn fetch_models(
    site: &SiteRow,
    api_key: &str,
    settings: &AppSettings,
) -> AppResult<FetchModelsResult> {
    let preview = normalize_base_url(&site.base_url)?;
    let client = crate::http_client::build_client(settings, std::time::Duration::from_secs(15))?;

    let start = Instant::now();
    let (endpoint, models) = match site.protocol {
        SiteProtocol::OpenaiCompatible => {
            let endpoint = preview.models_url.clone();
            let models = match attempt_models(&client, &endpoint, api_key, AuthStyle::Bearer, false)
                .await
            {
                Ok(models) => models,
                Err(first_error) => attempt_models(
                    &client,
                    &endpoint,
                    api_key,
                    AuthStyle::Bearer,
                    true,
                )
                .await
                .map_err(|_| first_error)?,
            };
            (endpoint, models)
        }
        SiteProtocol::Anthropic => {
            let endpoint = preview.models_url.clone();
            // 链路：标准 x-api-key → Bearer + Claude Code 伪装 → x-api-key + 伪装
            let attempts = [
                (AuthStyle::XApiKey, false),
                (AuthStyle::Bearer, true),
                (AuthStyle::XApiKey, true),
            ];
            let mut models = None;
            let mut last_error = None;
            for (style, impersonate) in attempts {
                match attempt_models(&client, &endpoint, api_key, style, impersonate).await {
                    Ok(found) => {
                        models = Some(found);
                        break;
                    }
                    Err(error) => last_error = Some(error),
                }
            }
            match models {
                Some(models) => (endpoint, models),
                None => {
                    return Err(last_error.unwrap_or_else(|| {
                        AppError::new("invalid_response", "models fetch failed")
                    }))
                }
            }
        }
    };

    let models = models
        .into_iter()
        .map(|mut model| {
            model.site_id = site.id.clone();
            model
        })
        .collect();
    let latency_ms = start.elapsed().as_millis() as u64;
    Ok(FetchModelsResult {
        models,
        latency_ms,
        endpoint,
        fetched_at: Utc::now().timestamp_millis(),
        api_key_id: String::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{ClaudeAuthKeyStyle, SiteKeyState};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    fn site(protocol: SiteProtocol, base_url: &str) -> SiteRow {
        SiteRow {
            id: "s1".into(),
            name: "R".into(),
            base_url: base_url.into(),
            base_urls: vec![base_url.into()],
            api_key_encrypted: "x".into(),
            key_prefix: "sk-xx".into(),
            protocol,
            claude_auth_key_style: ClaudeAuthKeyStyle::AnthropicAuthToken,
            notes: None,
            enabled: true,
            sort_order: 0,
            selected_model_id: None,
            last_model_fetch_at: None,
            last_model_fetch_latency_ms: None,
            last_model_fetch_error: None,
            created_at: 1,
            updated_at: 1,
            capabilities: Default::default(),
            keys: SiteKeyState {
                active_api_key_id: None,
                api_keys: Vec::new(),
            },
            newapi_access_token_encrypted: None,
            newapi_user_id: None,
        }
    }

    fn none_proxy() -> AppSettings {
        let mut settings = AppSettings::default();
        settings.proxy_mode = "none".into();
        settings
    }

    /// 只放行 Claude Code 客户端特征（模拟 AgentRouter 客户端检测）。
    async fn cc_gated_server() -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move {
            loop {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut bytes = Vec::new();
                let mut buffer = [0_u8; 2048];
                loop {
                    let read = socket.read(&mut buffer).await.unwrap();
                    if read == 0 {
                        break;
                    }
                    bytes.extend_from_slice(&buffer[..read]);
                    if bytes.windows(4).any(|window| window == b"\r\n\r\n") {
                        break;
                    }
                }
                let request = String::from_utf8_lossy(&bytes).to_string();
                let (status, body) = if request.contains("claude-cli/") {
                    (
                        "200 OK",
                        r#"{"data":[{"id":"claude-sonnet-4","owned_by":"anthropic"}]}"#,
                    )
                } else {
                    (
                        "401 Unauthorized",
                        r#"{"error":{"message":"unauthorized client detected"}}"#,
                    )
                };
                let response = format!(
                    "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                socket.write_all(response.as_bytes()).await.unwrap();
            }
        });
        format!("http://{address}")
    }

    #[tokio::test]
    async fn falls_back_to_claude_code_headers_when_client_detection_rejects() {
        let base = cc_gated_server().await;
        let site = site(SiteProtocol::Anthropic, &base);
        let result = fetch_models(&site, "sk-real", &none_proxy()).await.unwrap();
        assert_eq!(result.models.len(), 1);
        assert_eq!(result.models[0].model_id, "claude-sonnet-4");
        assert_eq!(result.models[0].site_id, "s1");
    }

    #[tokio::test]
    async fn openai_protocol_also_falls_back_to_claude_code_headers() {
        let base = cc_gated_server().await;
        let site = site(SiteProtocol::OpenaiCompatible, &base);
        let result = fetch_models(&site, "sk-real", &none_proxy()).await.unwrap();
        assert_eq!(result.models[0].model_id, "claude-sonnet-4");
    }
}
