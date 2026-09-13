//! 上游请求构造与响应回写。
//!
//! 流式是硬要求：请求体与响应体都按块搬运，任何一处整体缓冲都会让 SSE 卡到结束
//! 才吐出首个字节（Claude Code / Codex 的交互体感直接受影响）。

use crate::domain::{AppSettings, SiteRow};
use crate::error::{AppError, AppResult};
use bytes::Bytes;
use http_body_util::{BodyExt, BodyStream, StreamBody};
use hyper::body::Frame;
use hyper::{Request, Response};
use std::time::Duration;

pub type ProxyBody = BoxBody;

/// 统一的 body 类型：既能装流式上游体，也能装本地生成的错误体。
pub type BoxBody =
    http_body_util::combinators::BoxBody<Bytes, std::io::Error>;

pub fn empty_body() -> BoxBody {
    http_body_util::Full::new(Bytes::new())
        .map_err(|never| match never {})
        .boxed()
}

pub fn json_body(value: serde_json::Value) -> BoxBody {
    let bytes = Bytes::from(serde_json::to_vec(&value).unwrap_or_default());
    http_body_util::Full::new(bytes)
        .map_err(|never| match never {})
        .boxed()
}

/// 上游客户端：不设整体超时（长回答可能持续数分钟），只约束连接与读间隔。
pub fn build_upstream_client(settings: &AppSettings) -> AppResult<reqwest::Client> {
    let mut builder = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(15))
        .read_timeout(Duration::from_secs(180))
        .pool_idle_timeout(Duration::from_secs(90))
        .redirect(reqwest::redirect::Policy::limited(3));
    builder = crate::http_client::apply_resolved_proxy(
        builder,
        &crate::http_client::resolve_proxy(settings)?,
    )?;
    builder
        .build()
        .map_err(|e| AppError::new("network", e.to_string()))
}

/// 判断本次请求是否为流式，用于日志与超时策略。
pub fn is_streaming(headers: &hyper::HeaderMap) -> bool {
    headers
        .get(hyper::header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.contains("text/event-stream"))
        .unwrap_or(false)
}

/// 把 hyper 请求体转成 reqwest 可用的流式 body。
///
/// 先收成一批帧再逐块转发，避免 `BodyStream` 的 `poll_frame` 要求
/// `Unpin`/`Send` 组合不成立；这里用 `collect` 的流式变体保证不整体缓冲。
pub fn to_reqwest_body(
    body: hyper::body::Incoming,
) -> impl futures_util::Stream<Item = Result<Bytes, std::io::Error>> + Send + 'static {
    use futures_util::StreamExt;
    BodyStream::new(body).filter_map(|frame| async move {
        match frame {
            Ok(frame) => frame.into_data().ok().map(Ok),
            Err(error) => Some(Err(std::io::Error::other(error))),
        }
    })
}

/// 把上游响应转成 hyper 响应。`content-length` 交给传输层重算（分块回写会改变它）；
/// `content-encoding` 原样透传，既不拆压缩也不改语义。
pub fn response_from_upstream(
    status: reqwest::StatusCode,
    headers: &reqwest::header::HeaderMap,
    body: reqwest::Response,
) -> Response<BoxBody> {
    use futures_util::StreamExt;
    let mut builder = Response::builder().status(status.as_u16());
    for (name, value) in headers.iter() {
        if crate::local_proxy::headers::is_hop_by_hop(name.as_str()) {
            continue;
        }
        if name.as_str().eq_ignore_ascii_case("content-length") {
            continue;
        }
        builder = builder.header(name, value);
    }
    let stream = body
        .bytes_stream()
        .map(|item| item.map(Frame::data).map_err(std::io::Error::other));
    let body: BoxBody = BodyExt::boxed(StreamBody::new(stream));    builder
        .body(body)
        .unwrap_or_else(|_| Response::new(empty_body()))
}

/// 面向客户端的错误体。消息不含密钥材料（调用方负责裁剪）。
pub fn error_response(status: u16, message: &str) -> Response<BoxBody> {
    let payload = serde_json::json!({
        "error": { "type": "proxy_error", "message": message }
    });
    Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(json_body(payload))
        .unwrap_or_else(|_| Response::new(empty_body()))
}

/// 上游 URL 的安全约束：仅 http/https，且必须来自数据库中的站点配置。
pub fn assert_allowed_upstream(url: &str) -> AppResult<()> {
    let parsed = url::Url::parse(url)
        .map_err(|e| AppError::new("validation_failed", format!("invalid upstream url: {e}")))?;
    match parsed.scheme() {
        "http" | "https" => Ok(()),
        other => Err(AppError::new(
            "validation_failed",
            format!("unsupported upstream scheme: {other}"),
        )),
    }
}

/// 记录一次转发结果的辅助结构，保证日志字段齐全且不含敏感内容。
pub struct RequestOutcome {
    pub status: u16,
    pub error: Option<String>,
    pub duration_ms: u64,
}

pub fn header_map_from_hyper(
    headers: &hyper::HeaderMap,
) -> Vec<(String, String)> {
    headers
        .iter()
        .filter_map(|(name, value)| {
            value
                .to_str()
                .ok()
                .map(|v| (name.as_str().to_string(), v.to_string()))
        })
        .collect()
}

/// 组装上游请求头：客户端头透传 + 站点覆盖（后者胜）。
///
/// `overrides` 由调用方解密后传入（密文在数据库里，解密需要 `Crypto`）。
/// 这里再跑一次校验：保存层拦不到的数据来源（WebDAV 同步进来的他机数据库、
/// 手工改库）如果带进非法名或受保护头，宁可让本请求明确失败，也不要以错误契约
/// 打到上游（例如把 JSON body 标成 text/plain）。
pub fn merge_headers(
    client_headers: &hyper::HeaderMap,
    overrides: &[crate::domain::ProxyHeader],
    site_id: &str,
    api_key: &str,
) -> Result<Vec<(String, String)>, String> {
    crate::local_proxy::headers::validate_proxy_headers(overrides)?;

    let mut out: Vec<(String, String)> = header_map_from_hyper(client_headers)
        .into_iter()
        .filter(|(name, _)| {
            let lower = name.to_ascii_lowercase();
            !crate::local_proxy::headers::is_hop_by_hop(&lower)
                && lower != "host"
                && lower != "content-length"
        })
        .collect();

    for (name, value) in
        crate::local_proxy::headers::effective_headers(overrides, site_id, api_key)
    {
        let lower = name.to_ascii_lowercase();
        out.retain(|(existing, _)| existing.to_ascii_lowercase() != lower);
        out.push((name, value));
    }
    Ok(out)
}

pub fn request_log_target(target: crate::domain::TargetKind) -> &'static str {
    target.as_str()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ProxyHeader;

    #[test]
    fn only_http_and_https_upstreams_are_allowed() {
        assert!(assert_allowed_upstream("https://api.example.com/v1").is_ok());
        assert!(assert_allowed_upstream("http://127.0.0.1:18087/v1").is_ok());
        assert!(assert_allowed_upstream("file:///etc/passwd").is_err());
        assert!(assert_allowed_upstream("gopher://x/1").is_err());
    }

    #[test]
    fn client_headers_are_overridden_by_site_headers() {
        let mut headers = hyper::HeaderMap::new();
        headers.insert("user-agent", "client-agent/1.0".parse().unwrap());
        headers.insert("x-opencode-session", "old".parse().unwrap());
        headers.insert("content-length", "10".parse().unwrap());
        headers.insert("connection", "keep-alive".parse().unwrap());

        let overrides = vec![ProxyHeader {
            name: "X-Opencode-Session".into(),
            value: "fresh".into(),
            enabled: true,
        }];

        let merged = merge_headers(&headers, &overrides, "s1", "sk-key").expect("valid overrides");
        let find = |name: &str| {
            merged
                .iter()
                .find(|(n, _)| n.eq_ignore_ascii_case(name))
                .map(|(_, v)| v.clone())
        };
        assert_eq!(find("user-agent").as_deref(), Some("client-agent/1.0"));
        assert_eq!(find("x-opencode-session").as_deref(), Some("fresh"));
        assert!(find("content-length").is_none(), "逐跳头必须丢弃");
        assert!(find("connection").is_none());
    }

    #[test]
    fn untouched_headers_pass_through_verbatim() {
        let mut headers = hyper::HeaderMap::new();
        headers.insert("accept", "text/event-stream".parse().unwrap());
        headers.insert("anthropic-version", "2023-06-01".parse().unwrap());
        let merged = merge_headers(&headers, &[], "s1", "").expect("no overrides");
        let names: Vec<&str> = merged.iter().map(|(n, _)| n.as_str()).collect();
        assert!(names.contains(&"accept"));
        assert!(names.contains(&"anthropic-version"));
        assert!(is_streaming(&headers));
    }

    /// 转发层必须独立把关：数据库可能被同步/手工修改绕过保存层校验。
    #[test]
    fn forward_layer_rejects_headers_the_save_layer_would_have_blocked() {
        for bad in ["host", "content-type", "X Foo"] {
            let overrides = vec![ProxyHeader {
                name: bad.into(),
                value: "v".into(),
                enabled: true,
            }];
            let result = merge_headers(&hyper::HeaderMap::new(), &overrides, "s1", "");
            assert!(result.is_err(), "{bad} 必须在转发层被拒");
        }
    }
}
