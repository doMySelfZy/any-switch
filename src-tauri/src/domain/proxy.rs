use serde::{Deserialize, Serialize};

/// 站点级本地代理请求头覆盖。
///
/// `value` 支持三个占位符，只在转发时按请求替换，不落库：
/// `${API_KEY}` 站点当前激活密钥、`${SESSION}` 按站点稳定派生的会话值、
/// `${UUID}` 每请求新生成。`enabled = false` 的条目保留但不参与转发。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProxyHeader {
    pub name: String,
    pub value: String,
    #[serde(default = "crate::domain::default_true")]
    pub enabled: bool,
}

/// 单个目标的接管状态，供 UI 展示"这个目标现在指向哪里"。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalProxyTargetStatus {
    pub target: String,
    pub takeover: bool,
    pub site_id: Option<String>,
    pub site_name: Option<String>,
    pub client_base_url: Option<String>,
}

/// 代理运行状态。`path_token` 属于本机凭据，只用于拼装接管地址，UI 只展示截断形式。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LocalProxyStatus {
    pub running: bool,
    pub address: String,
    pub port: u16,
    pub path_token: String,
    pub started_at: Option<i64>,
    pub uptime_seconds: u64,
    pub total_requests: u64,
    pub success_requests: u64,
    pub failed_requests: u64,
    pub active_connections: u64,
    pub last_error: Option<String>,
    pub targets: Vec<LocalProxyTargetStatus>,
}

/// 请求日志条目。刻意不包含任何请求头与请求体；`path` 已剥掉 token 前缀。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalProxyRequestLogEntry {
    pub id: u64,
    pub at: i64,
    pub target: String,
    pub method: String,
    pub path: String,
    pub status: u16,
    pub duration_ms: u64,
    pub error: Option<String>,
}

/// 接管开关请求体：`targets` 为接管集合，`enabled` 决定该目标是否加入。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalProxyTakeoverInput {
    pub target: crate::domain::TargetKind,
    pub enabled: bool,
}
