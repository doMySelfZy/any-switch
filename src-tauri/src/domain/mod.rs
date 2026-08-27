use crate::capabilities::SiteCapabilities;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SiteProtocol {
    OpenaiCompatible,
    Anthropic,
}

impl SiteProtocol {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::OpenaiCompatible => "openai_compatible",
            Self::Anthropic => "anthropic",
        }
    }
    pub fn parse(s: &str) -> Self {
        match s {
            "anthropic" => Self::Anthropic,
            _ => Self::OpenaiCompatible,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClaudeAuthKeyStyle {
    AnthropicAuthToken,
    AnthropicApiKey,
}

impl ClaudeAuthKeyStyle {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::AnthropicAuthToken => "anthropic_auth_token",
            Self::AnthropicApiKey => "anthropic_api_key",
        }
    }
    pub fn parse(s: &str) -> Self {
        match s {
            "anthropic_api_key" => Self::AnthropicApiKey,
            _ => Self::AnthropicAuthToken,
        }
    }
    pub fn env_key(&self) -> &'static str {
        match self {
            Self::AnthropicAuthToken => "ANTHROPIC_AUTH_TOKEN",
            Self::AnthropicApiKey => "ANTHROPIC_API_KEY",
        }
    }
    pub fn other_env_key(&self) -> &'static str {
        match self {
            Self::AnthropicAuthToken => "ANTHROPIC_API_KEY",
            Self::AnthropicApiKey => "ANTHROPIC_AUTH_TOKEN",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum TargetKind {
    ClaudeCode,
    Codex,
}

impl TargetKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ClaudeCode => "claude_code",
            Self::Codex => "codex",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "claude_code" => Some(Self::ClaudeCode),
            "codex" => Some(Self::Codex),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApplyStatus {
    Applied,
    Stale,
    Orphan,
    NotApplied,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SiteDto {
    pub id: String,
    pub name: String,
    pub base_url: String,
    #[serde(default)]
    pub base_urls: Vec<String>,
    pub key_prefix: String,
    pub has_key: bool,
    pub protocol: String,
    pub claude_auth_key_style: String,
    pub notes: Option<String>,
    pub enabled: bool,
    pub sort_order: i64,
    pub selected_model_id: Option<String>,
    pub last_model_fetch_at: Option<i64>,
    pub last_model_fetch_latency_ms: Option<i64>,
    pub last_model_fetch_error: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    #[serde(default)]
    pub capabilities: SiteCapabilities,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SiteModelDto {
    pub id: String,
    pub site_id: String,
    pub model_id: String,
    pub display_name: String,
    pub owned_by: Option<String>,
    pub raw: Option<serde_json::Value>,
    #[serde(default)]
    pub is_manual: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSiteInput {
    pub name: String,
    #[serde(default)]
    pub base_url: String,
    pub base_urls: Option<Vec<String>>,
    pub api_key: String,
    pub protocol: Option<String>,
    pub claude_auth_key_style: Option<String>,
    pub notes: Option<String>,
    #[serde(default)]
    pub capabilities: Option<SiteCapabilities>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeepLinkSiteImportInput {
    pub name: String,
    pub base_urls: Vec<String>,
    pub api_key: String,
    pub protocol: Option<String>,
    pub notes: Option<String>,
    #[serde(default)]
    pub capabilities: Option<SiteCapabilities>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeepLinkSiteImportResult {
    pub site: SiteDto,
    pub created: bool,
    pub updated_key: bool,
    pub reused: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSiteInput {
    pub name: Option<String>,
    pub base_url: Option<String>,
    pub base_urls: Option<Vec<String>>,
    pub api_key: Option<String>,
    pub protocol: Option<String>,
    pub claude_auth_key_style: Option<String>,
    pub notes: Option<String>,
    pub enabled: Option<bool>,
    pub selected_model_id: Option<String>,
    pub sort_order: Option<i64>,
    #[serde(default)]
    pub capabilities: Option<SiteCapabilities>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchModelsResult {
    pub models: Vec<SiteModelDto>,
    pub latency_ms: u64,
    pub endpoint: String,
    pub fetched_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelProbeResult {
    pub model_id: String,
    pub ok: bool,
    pub latency_ms: u64,
    pub status: Option<u16>,
    pub error: Option<String>,
    pub endpoint: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum QuotaProbeStatus {
    Available,
    Unsupported,
    Unauthorized,
    Error,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum QuotaSource {
    CreditGrants,
    SubscriptionUsage,
    SubscriptionOnly,
    UsageOnly,
    TokenUsage,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SiteQuota {
    pub status: QuotaProbeStatus,
    pub remaining_usd: Option<f64>,
    pub used_usd: Option<f64>,
    pub total_usd: Option<f64>,
    pub unlimited: bool,
    #[serde(default)]
    pub unit: Option<String>,
    pub expires_at: Option<i64>,
    pub source: Option<QuotaSource>,
    pub endpoint: Option<String>,
    pub fetched_at: i64,
    pub latency_ms: u64,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetLiveStatus {
    pub kind: TargetKind,
    pub installed: bool,
    pub version: Option<String>,
    pub config_path: String,
    pub status: ApplyStatus,
    pub applied_site_id: Option<String>,
    pub applied_site_name: Option<String>,
    pub applied_model_id: Option<String>,
    pub provider_id: Option<String>,
    pub orphan: bool,
    pub live_summary: HashMap<String, Option<String>>,
    pub last_applied_at: Option<i64>,
    pub stale_reason: Option<String>,
}

/// Claude Code effort / thinking level written to settings + env.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClaudeEffortLevel {
    Low,
    Medium,
    High,
    Max,
}

impl ClaudeEffortLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Max => "max",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "low" => Some(Self::Low),
            "medium" => Some(Self::Medium),
            "high" => Some(Self::High),
            "max" => Some(Self::Max),
            _ => None,
        }
    }
}

/// Codex reasoning effort in config.toml.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CodexReasoningEffort {
    Minimal,
    Low,
    Medium,
    High,
    Xhigh,
}

impl CodexReasoningEffort {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Minimal => "minimal",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Xhigh => "xhigh",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "minimal" => Some(Self::Minimal),
            "low" => Some(Self::Low),
            "medium" => Some(Self::Medium),
            "high" => Some(Self::High),
            "xhigh" => Some(Self::Xhigh),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CapabilitySource {
    #[default]
    Site,
    Custom,
}

impl CapabilitySource {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Site => "site",
            Self::Custom => "custom",
        }
    }

    pub fn parse(raw: Option<&str>) -> Self {
        match raw.map(str::trim).map(|s| s.to_ascii_lowercase()) {
            Some(value) if value == "custom" => Self::Custom,
            _ => Self::Site,
        }
    }
}

/// Extra options for Claude Code apply.
#[derive(Debug, Clone, Default)]
pub struct ClaudeApplyOptions {
    pub opus_model_id: Option<String>,
    pub sonnet_model_id: Option<String>,
    pub haiku_model_id: Option<String>,
    pub effort_level: Option<ClaudeEffortLevel>,
}

/// Extra options for Codex apply.
#[derive(Debug, Clone, Default)]
pub struct CodexApplyOptions {
    pub write_all_models: bool,
    pub reasoning_effort: Option<CodexReasoningEffort>,
    /// Site models used when `write_all_models` is true.
    pub catalog_models: Vec<(String, String)>, // (model_id, display_name)
    pub remote_compaction: bool,
    pub image_understanding: bool,
    pub image_generation: bool,
    pub web_search: bool,
    pub capability_source: CapabilitySource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyTargetResult {
    pub target: TargetKind,
    pub ok: bool,
    pub status: ApplyStatus,
    pub backup_paths: Vec<String>,
    pub message: String,
    pub live_summary: Option<HashMap<String, Option<String>>>,
    pub touched_keys: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyResult {
    pub site_id: String,
    pub model_id: String,
    pub results: Vec<ApplyTargetResult>,
    pub applied_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub language: String,
    pub theme_mode: String,
    pub primary_color: String,
    pub auto_start: bool,
    pub always_on_top: bool,
    pub claude_home_override: Option<String>,
    pub codex_home_override: Option<String>,
    pub codex_env_inject_mode: String,
    pub force_exclusive_claude_auth_key: bool,
    #[serde(default = "default_true")]
    pub auto_check_update: bool,
    #[serde(default = "default_update_check_interval")]
    pub update_check_interval: u32,
    /// Max backup copies kept per target under ~/.xiaobai-switch/backups/{target}.
    #[serde(default = "default_max_backup_copies")]
    pub max_backup_copies: u32,
    #[serde(default = "default_proxy_mode")]
    pub proxy_mode: String,
    #[serde(default = "default_proxy_protocol")]
    pub proxy_protocol: String,
    #[serde(default)]
    pub proxy_host: Option<String>,
    #[serde(default)]
    pub proxy_port: Option<u16>,
    #[serde(default = "default_route_probe_ttl")]
    pub route_probe_ttl_minutes: u32,
    /// Hide the main window instead of quitting when the user closes it.
    #[serde(default = "default_true")]
    pub close_to_tray: bool,
    /// Keep the main window hidden on launch (only meaningful with close_to_tray).
    #[serde(default)]
    pub start_in_tray: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebDavConfigView {
    pub base_url: String,
    pub username: String,
    pub remote_path: String,
    pub accept_invalid_certs: bool,
    pub has_password: bool,
    pub auto_sync_enabled: bool,
    pub sync_interval_minutes: u32,
    pub max_remote_backups: u32,
}

impl Default for WebDavConfigView {
    fn default() -> Self {
        Self {
            base_url: String::new(),
            username: String::new(),
            remote_path: "xiaobai-switch".into(),
            accept_invalid_certs: false,
            has_password: false,
            auto_sync_enabled: false,
            sync_interval_minutes: 60,
            max_remote_backups: 10,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveWebDavConfigInput {
    pub base_url: String,
    pub username: String,
    #[serde(default)]
    pub password: Option<String>,
    pub remote_path: String,
    #[serde(default)]
    pub accept_invalid_certs: bool,
    #[serde(default)]
    pub auto_sync_enabled: bool,
    pub sync_interval_minutes: u32,
    pub max_remote_backups: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestWebDavConnectionInput {
    pub base_url: String,
    pub username: String,
    #[serde(default)]
    pub password: Option<String>,
    pub remote_path: String,
    #[serde(default)]
    pub accept_invalid_certs: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteBackupInfo {
    pub file_name: String,
    pub size: u64,
    pub last_modified: String,
    pub device_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebDavSyncStatus {
    pub last_attempt_at: Option<i64>,
    pub last_success_at: Option<i64>,
    pub status: String,
    pub error: Option<String>,
}

impl Default for WebDavSyncStatus {
    fn default() -> Self {
        Self {
            last_attempt_at: None,
            last_success_at: None,
            status: "never".into(),
            error: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupOverview {
    pub latest_local_backup_at: Option<i64>,
    pub webdav_configured: bool,
    pub webdav_auto_sync_enabled: bool,
    pub webdav_sync: WebDavSyncStatus,
    pub next_scheduled_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupOperationResult {
    pub file_name: String,
    pub local_path: Option<String>,
    pub uploaded: bool,
    pub warning: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreStartupResult {
    pub status: String,
    pub message: String,
}

pub fn default_max_backup_copies() -> u32 {
    30
}

pub fn clamp_max_backup_copies(n: u32) -> u32 {
    n.clamp(1, 200)
}

pub fn default_proxy_mode() -> String {
    "system".into()
}

pub fn default_proxy_protocol() -> String {
    "http".into()
}

pub fn default_route_probe_ttl() -> u32 {
    10
}

pub fn clamp_route_probe_ttl(n: u32) -> u32 {
    n.clamp(1, 1440)
}

pub fn default_update_check_interval() -> u32 {
    60
}

pub fn clamp_update_check_interval(n: u32) -> u32 {
    n.clamp(1, 1440)
}

pub fn default_true() -> bool {
    true
}

pub fn normalize_proxy_mode(s: &str) -> String {
    match s.trim() {
        "none" | "custom" => s.trim().into(),
        _ => default_proxy_mode(),
    }
}

pub fn normalize_proxy_protocol(s: &str) -> String {
    match s.trim().to_ascii_lowercase().as_str() {
        "https" | "socks5" => s.trim().to_ascii_lowercase(),
        _ => default_proxy_protocol(),
    }
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            language: "zh-CN".into(),
            theme_mode: "system".into(),
            primary_color: "#1677ff".into(),
            auto_start: false,
            always_on_top: false,
            claude_home_override: None,
            codex_home_override: None,
            codex_env_inject_mode: "auto".into(),
            force_exclusive_claude_auth_key: false,
            auto_check_update: true,
            update_check_interval: default_update_check_interval(),
            max_backup_copies: default_max_backup_copies(),
            proxy_mode: default_proxy_mode(),
            proxy_protocol: default_proxy_protocol(),
            proxy_host: None,
            proxy_port: None,
            route_probe_ttl_minutes: default_route_probe_ttl(),
            close_to_tray: true,
            start_in_tray: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwitchRouteResult {
    pub site: SiteDto,
    pub results: Vec<ApplyTargetResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UrlProbeResult {
    pub url: String,
    pub ok: bool,
    pub latency_ms: u64,
    pub status: Option<u16>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpBytesResult {
    pub status: u16,
    pub content_type: String,
    pub final_url: String,
    pub base64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyRecordDto {
    pub id: String,
    pub site_id: Option<String>,
    pub site_name_snapshot: String,
    pub target: String,
    pub model_id: String,
    pub provider_id: Option<String>,
    pub status: String,
    pub backup_dir: Option<String>,
    pub error: Option<String>,
    pub applied_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupInfo {
    pub id: String,
    pub target: String,
    pub dir: String,
    pub created_at: i64,
    pub files: Vec<String>,
    pub apply_record_id: Option<String>,
    pub site_name_snapshot: Option<String>,
    #[serde(default)]
    pub model_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupFileInfo {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupPreview {
    pub id: String,
    pub summary: HashMap<String, Option<String>>,
    pub files: Vec<BackupFileInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CliToolInfo {
    pub kind: TargetKind,
    pub installed: bool,
    pub version: Option<String>,
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TouchedKeys {
    pub paths: Vec<String>,
    pub created_paths: Vec<String>,
    pub env_keys: Vec<String>,
    pub claude_env_keys: Vec<String>,
    pub shell_rc_paths: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct TargetBinding {
    pub target: TargetKind,
    pub site_id: Option<String>,
    pub site_name_snapshot: String,
    pub model_id: String,
    pub provider_id: Option<String>,
    pub key_fingerprint: String,
    pub managed_paths: Vec<String>,
    pub managed_env_keys: Vec<String>,
    pub expected_fields: HashMap<String, String>,
    pub orphan: bool,
    pub applied_at: i64,
    pub apply_record_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SiteRow {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub base_urls: Vec<String>,
    pub api_key_encrypted: String,
    pub key_prefix: String,
    pub protocol: SiteProtocol,
    pub claude_auth_key_style: ClaudeAuthKeyStyle,
    pub notes: Option<String>,
    pub enabled: bool,
    pub sort_order: i64,
    pub selected_model_id: Option<String>,
    pub last_model_fetch_at: Option<i64>,
    pub last_model_fetch_latency_ms: Option<i64>,
    pub last_model_fetch_error: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub capabilities: SiteCapabilities,
}

impl SiteRow {
    pub fn to_dto(&self) -> SiteDto {
        SiteDto {
            id: self.id.clone(),
            name: self.name.clone(),
            base_url: self.base_url.clone(),
            base_urls: if self.base_urls.is_empty() {
                vec![self.base_url.clone()]
            } else {
                self.base_urls.clone()
            },
            key_prefix: self.key_prefix.clone(),
            has_key: !self.api_key_encrypted.is_empty(),
            protocol: self.protocol.as_str().into(),
            claude_auth_key_style: self.claude_auth_key_style.as_str().into(),
            notes: self.notes.clone(),
            enabled: self.enabled,
            sort_order: self.sort_order,
            selected_model_id: self.selected_model_id.clone(),
            last_model_fetch_at: self.last_model_fetch_at,
            last_model_fetch_latency_ms: self.last_model_fetch_latency_ms,
            last_model_fetch_error: self.last_model_fetch_error.clone(),
            created_at: self.created_at,
            updated_at: self.updated_at,
            capabilities: self.capabilities.clone(),
        }
    }
}

pub fn provider_id_for_site(site_id: &str) -> String {
    let compact: String = site_id.chars().filter(|c| *c != '-').collect();
    let short = if compact.len() >= 12 {
        compact[..12].to_lowercase()
    } else {
        compact.to_lowercase()
    };
    format!("xiaobai_{short}")
}

pub fn env_key_for_site(site_id: &str) -> String {
    let compact: String = site_id.chars().filter(|c| *c != '-').collect();
    let short = if compact.len() >= 12 {
        compact[..12].to_uppercase()
    } else {
        compact.to_uppercase()
    };
    format!("XIAOBAI_SITE_{short}_API_KEY")
}
