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
            let resp = client.get(&endpoint).bearer_auth(api_key).send().await?;
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
            let body: OpenAiModelsResponse = resp
                .json()
                .await
                .map_err(|e| AppError::new("invalid_response", e.to_string()))?;
            let models = body
                .data
                .unwrap_or_default()
                .into_iter()
                .map(|m| SiteModelDto {
                    id: Uuid::new_v4().to_string(),
                    site_id: site.id.clone(),
                    api_key_id: String::new(),
                    model_id: m.id.clone(),
                    display_name: m.id,
                    owned_by: m.owned_by,
                    raw: None,
                    is_manual: false,
                })
                .collect();
            (endpoint, models)
        }
        SiteProtocol::Anthropic => {
            // Prefer Anthropic-style list; fall back to OpenAI-compatible path with x-api-key
            let endpoint = preview.models_url.clone();
            let resp = client
                .get(&endpoint)
                .header("x-api-key", api_key)
                .header("anthropic-version", "2023-06-01")
                .send()
                .await;
            match resp {
                Ok(resp) => {
                    let status = resp.status();
                    if status.as_u16() == 401 || status.as_u16() == 403 {
                        return Err(AppError::new("unauthorized", "unauthorized"));
                    }
                    if !status.is_success() {
                        return Err(AppError::new(
                            "network",
                            format!(
                                "Anthropic models HTTP {}. You can enter model id manually.",
                                status.as_u16()
                            ),
                        ));
                    }
                    let text = resp.text().await?;
                    if let Ok(body) = serde_json::from_str::<AnthropicModelsResponse>(&text) {
                        let models = body
                            .data
                            .unwrap_or_default()
                            .into_iter()
                            .map(|m| SiteModelDto {
                                id: Uuid::new_v4().to_string(),
                                site_id: site.id.clone(),
                                api_key_id: String::new(),
                                model_id: m.id.clone(),
                                display_name: m.display_name.unwrap_or(m.id),
                                owned_by: Some("anthropic".into()),
                                raw: None,
                                is_manual: false,
                            })
                            .collect();
                        (endpoint, models)
                    } else if let Ok(body) = serde_json::from_str::<OpenAiModelsResponse>(&text) {
                        let models = body
                            .data
                            .unwrap_or_default()
                            .into_iter()
                            .map(|m| SiteModelDto {
                                id: Uuid::new_v4().to_string(),
                                site_id: site.id.clone(),
                                api_key_id: String::new(),
                                model_id: m.id.clone(),
                                display_name: m.id,
                                owned_by: m.owned_by,
                                raw: None,
                                is_manual: false,
                            })
                            .collect();
                        (endpoint, models)
                    } else {
                        return Err(AppError::new(
                            "invalid_response",
                            "Could not parse models response. Enter model id manually.",
                        ));
                    }
                }
                Err(e) => return Err(e.into()),
            }
        }
    };

    let latency_ms = start.elapsed().as_millis() as u64;
    Ok(FetchModelsResult {
        models,
        latency_ms,
        endpoint,
        fetched_at: Utc::now().timestamp_millis(),
        api_key_id: String::new(),
    })
}
