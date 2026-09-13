//! 本地代理请求头：校验、受保护名单、占位符替换。
//!
//! 站点级请求头是用户配置的"新增/覆盖"列表；客户端原始请求头默认透传。
//! 转发时只做覆盖，不改动其余头，也绝不改写 hop-by-hop 语义或重新编码响应体。

use crate::domain::ProxyHeader;

/// 不允许被覆盖的头：要么由传输层/代理自己决定，要么会破坏报文语义。
///
/// `content-type` 在列：请求体是客户端原样透传的，若被改成别的类型，
/// 上游会按错误的 wire 契约解析（CC Switch 的同类实现踩过这个坑）。
/// `authorization` / `x-api-key` 刻意**不在**名单里——"渠道要求用特定鉴权头名"
/// 正是本功能要解决的场景。
const PROTECTED_HEADERS: &[&str] = &[
    "host",
    "content-length",
    "transfer-encoding",
    "connection",
    "keep-alive",
    "te",
    "trailer",
    "upgrade",
    "proxy-authorization",
    "proxy-authenticate",
    "content-type",
];

/// hop-by-hop 头，转发时丢弃（由 reqwest/hyper 按新连接重建）。
const HOP_BY_HOP: &[&str] = &[
    "connection",
    "keep-alive",
    "proxy-authenticate",
    "proxy-authorization",
    "te",
    "trailer",
    "transfer-encoding",
    "upgrade",
];

pub const PLACEHOLDER_API_KEY: &str = "${API_KEY}";
pub const PLACEHOLDER_SESSION: &str = "${SESSION}";
pub const PLACEHOLDER_UUID: &str = "${UUID}";

/// RFC 9110 field-name token。与前端校验保持一致。
pub fn is_valid_header_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "!#$%&'*+-.^_`|~".contains(c))
}

/// 值里不允许除 tab 以外的控制字符，避免请求头注入。
pub fn is_valid_header_value(value: &str) -> bool {
    !value
        .chars()
        .any(|c| c.is_control() && c != '\t')
}

pub fn is_protected_header(name: &str) -> bool {
    let lower = name.trim().to_ascii_lowercase();
    PROTECTED_HEADERS.contains(&lower.as_str())
}

pub fn is_hop_by_hop(name: &str) -> bool {
    let lower = name.trim().to_ascii_lowercase();
    HOP_BY_HOP.contains(&lower.as_str())
}

/// 保存前的整体校验。返回用户可读的错误（前端直接展示）。
pub fn validate_proxy_headers(headers: &[ProxyHeader]) -> Result<(), String> {
    let mut seen: Vec<String> = Vec::new();
    for header in headers {
        let name = header.name.trim();
        if name.is_empty() {
            return Err("请求头名不能为空".into());
        }
        if !is_valid_header_name(name) {
            return Err(format!("请求头名 \"{}\" 不是合法的 HTTP token", header.name));
        }
        if !is_valid_header_value(&header.value) {
            return Err(format!("请求头 \"{}\" 的值包含控制字符", header.name));
        }
        let lower = name.to_ascii_lowercase();
        if is_protected_header(&lower) {
            return Err(format!("请求头 \"{}\" 由代理或传输层管理，不能覆盖", header.name));
        }
        if seen.contains(&lower) {
            return Err(format!("请求头 \"{}\" 与另一条重复（忽略大小写）", header.name));
        }
        seen.push(lower);
    }
    Ok(())
}

/// 按站点稳定派生会话值：同一站点跨重启结果一致，不同站点互相独立。
///
/// OpenCode Go 之类的网关用会话值把同一会话的请求固定到同一上游以提升缓存命中；
/// 它要求"稳定且不透明"，并不解析内容。
pub fn derive_session_id(site_id: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(b"xiaobai-proxy-session|");
    hasher.update(site_id.as_bytes());
    let digest = hasher.finalize();
    hex::encode(&digest[..16])
}

/// 单条请求头取值替换。`api_key` 只在内存中参与替换，绝不写入日志。
pub fn resolve_placeholder(value: &str, site_id: &str, api_key: &str) -> String {
    value
        .replace(PLACEHOLDER_API_KEY, api_key)
        .replace(PLACEHOLDER_SESSION, &derive_session_id(site_id))
        .replace(PLACEHOLDER_UUID, &uuid::Uuid::new_v4().to_string())
}

/// 计算最终生效的请求头列表（已过滤 disabled、已替换占位符）。
pub fn effective_headers(
    headers: &[ProxyHeader],
    site_id: &str,
    api_key: &str,
) -> Vec<(String, String)> {
    headers
        .iter()
        .filter(|h| h.enabled)
        .map(|h| {
            (
                h.name.trim().to_string(),
                resolve_placeholder(&h.value, site_id, api_key),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(name: &str, value: &str) -> ProxyHeader {
        ProxyHeader {
            name: name.into(),
            value: value.into(),
            enabled: true,
        }
    }

    #[test]
    fn accepts_valid_headers_including_auth_overrides() {
        let list = vec![
            header("x-opencode-session", "${SESSION}"),
            header("User-Agent", "my-agent/1.0"),
            header("authorization", "Bearer ${API_KEY}"),
        ];
        assert!(validate_proxy_headers(&list).is_ok());
    }

    #[test]
    fn rejects_invalid_names_and_values() {
        assert!(validate_proxy_headers(&[header("X Foo", "v")]).is_err());
        assert!(validate_proxy_headers(&[header("Authorization:", "v")]).is_err());
        assert!(validate_proxy_headers(&[header("", "v")]).is_err());
        assert!(validate_proxy_headers(&[header("x-ok", "bad\nvalue")]).is_err());
    }

    #[test]
    fn rejects_protected_and_duplicate_headers() {
        assert!(validate_proxy_headers(&[header("host", "evil")]).is_err());
        assert!(validate_proxy_headers(&[header("Content-Type", "text/plain")]).is_err());
        assert!(validate_proxy_headers(&[header("content-length", "0")]).is_err());
        assert!(
            validate_proxy_headers(&[header("X-Foo", "a"), header("x-foo", "b")]).is_err(),
            "大小写不同的重复头必须被拒，否则生效值取决于遍历顺序"
        );
    }

    #[test]
    fn placeholder_substitution_and_session_stability() {
        assert_eq!(
            resolve_placeholder("Bearer ${API_KEY}", "s1", "sk-secret"),
            "Bearer sk-secret"
        );
        let a = resolve_placeholder("${SESSION}", "site-a", "");
        let b = resolve_placeholder("${SESSION}", "site-a", "");
        let c = resolve_placeholder("${SESSION}", "site-b", "");
        assert_eq!(a, b, "同一站点会话值必须稳定");
        assert_ne!(a, c, "不同站点会话值必须独立");
        assert_eq!(a.len(), 32);

        let u1 = resolve_placeholder("${UUID}", "site-a", "");
        let u2 = resolve_placeholder("${UUID}", "site-a", "");
        assert_ne!(u1, u2, "UUID 占位符每次都应不同");
    }

    #[test]
    fn disabled_entries_are_skipped() {
        let list = vec![
            header("x-on", "1"),
            ProxyHeader {
                name: "x-off".into(),
                value: "2".into(),
                enabled: false,
            },
        ];
        let resolved = effective_headers(&list, "s1", "");
        assert_eq!(resolved, vec![("x-on".to_string(), "1".to_string())]);
    }
}
