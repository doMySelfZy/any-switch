use crate::domain::AppSettings;
use crate::error::{AppError, AppResult};
use std::time::Duration;

pub fn default_user_agent() -> &'static str {
    concat!("XiaoBaiSwitch/", env!("CARGO_PKG_VERSION"))
}

pub fn custom_proxy_url(protocol: &str, host: &str, port: u16) -> AppResult<String> {
    let proto = match protocol.trim().to_ascii_lowercase().as_str() {
        "http" | "https" | "socks5" => protocol.trim().to_ascii_lowercase(),
        _ => {
            return Err(AppError::new(
                "validation_failed",
                "unsupported proxy protocol",
            ));
        }
    };
    let host = host.trim();
    if host.is_empty() {
        return Err(AppError::new("validation_failed", "proxy host required"));
    }
    if host.chars().any(|c| c.is_whitespace() || c == '/') {
        return Err(AppError::new("validation_failed", "invalid proxy host"));
    }
    if port == 0 {
        return Err(AppError::new("validation_failed", "proxy port required"));
    }
    Ok(format!("{proto}://{host}:{port}"))
}

pub fn parse_scutil_proxy(text: &str) -> Option<String> {
    if let Some(ep) = enabled_scutil_endpoint(text, "HTTPS") {
        return Some(format!("http://{ep}"));
    }
    if let Some(ep) = enabled_scutil_endpoint(text, "HTTP") {
        return Some(format!("http://{ep}"));
    }
    if let Some(ep) = enabled_scutil_endpoint(text, "SOCKS") {
        return Some(format!("socks5://{ep}"));
    }
    None
}

fn enabled_scutil_endpoint(text: &str, kind: &str) -> Option<String> {
    if scutil_value(text, &format!("{kind}Enable")).as_deref() != Some("1") {
        return None;
    }
    let host = scutil_value(text, &format!("{kind}Proxy"))?;
    let port = scutil_value(text, &format!("{kind}Port"))?;
    if host.is_empty() || port.is_empty() {
        return None;
    }
    Some(format!("{host}:{port}"))
}

fn scutil_value(text: &str, key: &str) -> Option<String> {
    for line in text.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix(key) else {
            continue;
        };
        let rest = rest.trim().trim_start_matches(':').trim();
        if !rest.is_empty() {
            return Some(rest.to_string());
        }
    }
    None
}

/// Parse Windows `ProxyServer` (`host:port` or `http=host:port;https=host:port`).
#[allow(dead_code)]
pub fn parse_windows_proxy_server(proxy_server: &str) -> Option<String> {
    let s = proxy_server.trim();
    if s.is_empty() {
        return None;
    }
    if !s.contains('=') {
        return Some(with_scheme("http", s));
    }
    let mut https = None;
    let mut http = None;
    let mut socks = None;
    for part in s.split(';') {
        let Some((k, v)) = part.split_once('=') else {
            continue;
        };
        let key = k.trim().to_ascii_lowercase();
        let val = v.trim();
        if val.is_empty() {
            continue;
        }
        match key.as_str() {
            "https" if https.is_none() => https = Some(with_scheme("http", val)),
            "http" if http.is_none() => http = Some(with_scheme("http", val)),
            "socks" | "socks5" if socks.is_none() => socks = Some(with_scheme("socks5", val)),
            _ => {}
        }
    }
    https.or(http).or(socks)
}

fn with_scheme(scheme: &str, raw: &str) -> String {
    if raw.contains("://") {
        raw.to_string()
    } else {
        format!("{scheme}://{raw}")
    }
}

pub fn detect_env_proxy() -> Option<String> {
    for key in [
        "HTTPS_PROXY",
        "https_proxy",
        "HTTP_PROXY",
        "http_proxy",
        "ALL_PROXY",
        "all_proxy",
    ] {
        if let Ok(v) = std::env::var(key) {
            let v = v.trim();
            if !v.is_empty() {
                return Some(v.to_string());
            }
        }
    }
    None
}

pub fn detect_system_proxy() -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        if let Some(url) = detect_macos_scutil() {
            return Some(url);
        }
    }
    #[cfg(windows)]
    {
        if let Some(url) = detect_windows_inet() {
            return Some(url);
        }
    }
    detect_env_proxy()
}

#[cfg(target_os = "macos")]
fn detect_macos_scutil() -> Option<String> {
    let out = std::process::Command::new("scutil")
        .arg("--proxy")
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    parse_scutil_proxy(&text)
}

#[cfg(windows)]
fn detect_windows_inet() -> Option<String> {
    let hkcu = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
    let key = hkcu
        .open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Internet Settings")
        .ok()?;
    let enable: u32 = key.get_value("ProxyEnable").ok()?;
    if enable == 0 {
        return None;
    }
    let server: String = key.get_value("ProxyServer").ok()?;
    parse_windows_proxy_server(&server)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolvedProxy {
    Disabled,
    Url(String),
    Unset,
}

/// Apply the same proxy rules used by model fetch / URL probe.
/// `system` reads scutil / Windows Internet Settings, then HTTP(S)_PROXY.
/// `none` disables proxies (including env). `custom` requires host+port.
pub fn resolve_proxy(settings: &AppSettings) -> AppResult<ResolvedProxy> {
    match settings.proxy_mode.as_str() {
        "none" => Ok(ResolvedProxy::Disabled),
        "custom" => {
            let port = settings.proxy_port.unwrap_or(0);
            let host = settings.proxy_host.as_deref().unwrap_or("");
            Ok(ResolvedProxy::Url(custom_proxy_url(
                &settings.proxy_protocol,
                host,
                port,
            )?))
        }
        _ => Ok(detect_system_proxy()
            .map(ResolvedProxy::Url)
            .unwrap_or(ResolvedProxy::Unset)),
    }
}

pub fn apply_resolved_proxy(
    builder: reqwest::ClientBuilder,
    resolved: &ResolvedProxy,
) -> AppResult<reqwest::ClientBuilder> {
    match resolved {
        ResolvedProxy::Disabled => Ok(builder.no_proxy()),
        ResolvedProxy::Url(url) => {
            let proxy = reqwest::Proxy::all(url)
                .map_err(|e| AppError::new("validation_failed", e.to_string()))?;
            Ok(builder.proxy(proxy))
        }
        ResolvedProxy::Unset => Ok(builder),
    }
}

pub fn build_client(settings: &AppSettings, timeout: Duration) -> AppResult<reqwest::Client> {
    build_client_with_tls(settings, timeout, false)
}

pub fn build_client_with_tls(
    settings: &AppSettings,
    timeout: Duration,
    accept_invalid_certs: bool,
) -> AppResult<reqwest::Client> {
    let mut builder = reqwest::Client::builder()
        .timeout(timeout)
        .connect_timeout(timeout)
        .redirect(reqwest::redirect::Policy::limited(5))
        .user_agent(default_user_agent())
        .danger_accept_invalid_certs(accept_invalid_certs);

    builder = apply_resolved_proxy(builder, &resolve_proxy(settings)?)?;

    builder
        .build()
        .map_err(|e| AppError::new("network", e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_proxy_url_formats() {
        assert_eq!(
            custom_proxy_url("http", "127.0.0.1", 7890).unwrap(),
            "http://127.0.0.1:7890"
        );
        assert_eq!(
            custom_proxy_url("SOCKS5", "localhost", 1080).unwrap(),
            "socks5://localhost:1080"
        );
        assert!(custom_proxy_url("ftp", "127.0.0.1", 80).is_err());
        assert!(custom_proxy_url("http", "", 80).is_err());
        assert!(custom_proxy_url("http", "127.0.0.1", 0).is_err());
    }

    #[test]
    fn scutil_prefers_https_then_http_then_socks() {
        let text = r#"
<dictionary> {
  HTTPEnable : 1
  HTTPPort : 7890
  HTTPProxy : 127.0.0.1
  HTTPSEnable : 1
  HTTPSPort : 7891
  HTTPSProxy : 127.0.0.1
  SOCKSEnable : 1
  SOCKSPort : 7892
  SOCKSProxy : 127.0.0.1
}
"#;
        assert_eq!(
            parse_scutil_proxy(text).as_deref(),
            Some("http://127.0.0.1:7891")
        );
    }

    #[test]
    fn scutil_socks_when_http_disabled() {
        let text = r#"
  HTTPEnable : 0
  HTTPSEnable : 0
  SOCKSEnable : 1
  SOCKSPort : 1080
  SOCKSProxy : 127.0.0.1
"#;
        assert_eq!(
            parse_scutil_proxy(text).as_deref(),
            Some("socks5://127.0.0.1:1080")
        );
    }

    #[test]
    fn scutil_none_when_disabled() {
        let text = "HTTPEnable : 0\nHTTPSEnable : 0\nSOCKSEnable : 0\n";
        assert_eq!(parse_scutil_proxy(text), None);
    }

    #[test]
    fn windows_proxy_server_plain() {
        assert_eq!(
            parse_windows_proxy_server("127.0.0.1:7890").as_deref(),
            Some("http://127.0.0.1:7890")
        );
    }

    #[test]
    fn windows_proxy_server_scheme_list() {
        assert_eq!(
            parse_windows_proxy_server("http=127.0.0.1:7890;https=127.0.0.1:7891").as_deref(),
            Some("http://127.0.0.1:7891")
        );
        assert_eq!(
            parse_windows_proxy_server("socks=127.0.0.1:1080").as_deref(),
            Some("socks5://127.0.0.1:1080")
        );
    }

    #[test]
    fn build_none_and_custom_clients() {
        let mut s = AppSettings::default();
        s.proxy_mode = "none".into();
        assert!(build_client(&s, Duration::from_secs(2)).is_ok());

        s.proxy_mode = "custom".into();
        s.proxy_protocol = "http".into();
        s.proxy_host = Some("127.0.0.1".into());
        s.proxy_port = Some(7890);
        assert!(build_client(&s, Duration::from_secs(2)).is_ok());

        s.proxy_host = None;
        assert!(build_client(&s, Duration::from_secs(2)).is_err());
    }

    #[test]
    fn resolve_proxy_none_disables() {
        let mut s = AppSettings::default();
        s.proxy_mode = "none".into();
        assert_eq!(resolve_proxy(&s).unwrap(), ResolvedProxy::Disabled);
    }

    #[test]
    fn resolve_proxy_custom_url() {
        let mut s = AppSettings::default();
        s.proxy_mode = "custom".into();
        s.proxy_protocol = "socks5".into();
        s.proxy_host = Some("127.0.0.1".into());
        s.proxy_port = Some(7890);
        assert_eq!(
            resolve_proxy(&s).unwrap(),
            ResolvedProxy::Url("socks5://127.0.0.1:7890".into())
        );
    }

    #[test]
    fn resolve_proxy_custom_rejects_missing_host() {
        let mut s = AppSettings::default();
        s.proxy_mode = "custom".into();
        s.proxy_host = None;
        s.proxy_port = Some(7890);
        assert!(resolve_proxy(&s).is_err());
    }

    #[test]
    fn resolve_proxy_system_follows_detector() {
        let s = AppSettings::default();
        assert_eq!(s.proxy_mode, "system");
        let expected = detect_system_proxy()
            .map(ResolvedProxy::Url)
            .unwrap_or(ResolvedProxy::Unset);
        assert_eq!(resolve_proxy(&s).unwrap(), expected);
    }
}
