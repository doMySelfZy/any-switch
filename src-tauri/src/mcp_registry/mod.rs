//! 官方 MCP Registry（`registry.modelcontextprotocol.io`）搜索与安装草稿解析。
//!
//! 仓库是第三方 metadata 服务：它只描述「怎么启动/连接某个 MCP」，不托管代码，
//! 也不构成安全背书。因此这里只做两件事——把搜索结果归一化成应用内的候选条目，
//! 以及把单条目换算成可直接写入客户端配置的字段。安全提示由 UI 文案承担。

use crate::domain::McpKind;
use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

/// 官方仓库的 REST 基址。固定域名 + 仅 https，不接受调用方传入地址，
/// 避免把这条出站请求变成可被指向任意内网地址的入口。
pub const REGISTRY_BASE: &str = "https://registry.modelcontextprotocol.io/v0.1";

/// 响应体积上限：仓库条目可能很大，异常响应不应该把界面拖垮。
const MAX_RESPONSE_BYTES: usize = 4 * 1024 * 1024;

const DEFAULT_LIMIT: u32 = 20;
const MAX_LIMIT: u32 = 50;

// ---------------------------------------------------------------------------
// 仓库响应形状（只声明我们真正用到的字段）
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct RegistryServerList {
    #[serde(default)]
    pub servers: Vec<RegistryServerEntry>,
}

#[derive(Debug, Deserialize)]
pub struct RegistryServerEntry {
    pub server: RegistryServer,
}

#[derive(Debug, Deserialize)]
pub struct RegistryServer {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub repository: Option<RegistryRepository>,
    #[serde(default)]
    pub packages: Vec<RegistryPackage>,
    #[serde(default)]
    pub remotes: Vec<RegistryRemote>,
}

#[derive(Debug, Deserialize)]
pub struct RegistryRepository {
    #[serde(default)]
    pub url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RegistryPackage {
    #[serde(default, rename = "registryType")]
    pub registry_type: Option<String>,
    #[serde(default)]
    pub identifier: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default, rename = "runtimeHint")]
    pub runtime_hint: Option<String>,
    #[serde(default)]
    pub transport: Option<RegistryTransport>,
    #[serde(default, rename = "runtimeArguments")]
    pub runtime_arguments: Vec<RegistryArgument>,
    #[serde(default, rename = "packageArguments")]
    pub package_arguments: Vec<RegistryArgument>,
    #[serde(default, rename = "environmentVariables")]
    pub environment_variables: Vec<RegistryEnvVar>,
}

#[derive(Debug, Deserialize)]
pub struct RegistryTransport {
    #[serde(default, rename = "type")]
    pub kind: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RegistryArgument {
    #[serde(default)]
    pub value: Option<String>,
    #[serde(default, rename = "type")]
    pub kind: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RegistryEnvVar {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default, rename = "isRequired")]
    pub is_required: Option<bool>,
    #[serde(default, rename = "isSecret")]
    pub is_secret: Option<bool>,
    #[serde(default)]
    pub default: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RegistryRemote {
    #[serde(default, rename = "type")]
    pub kind: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub headers: Vec<RegistryEnvVar>,
}

// ---------------------------------------------------------------------------
// 应用内投影
// ---------------------------------------------------------------------------

/// 需要用户自己填的字段（必填且仓库没给默认值）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RegistryRequiredField {
    pub name: String,
    pub description: Option<String>,
    /// 秘钥类字段（`isSecret`），UI 用密码框并提示这是敏感信息。
    pub secret: bool,
    /// 这个字段是环境变量还是请求头，决定填到表单的哪个输入框。
    pub kind: RegistryFieldKind,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RegistryFieldKind {
    Env,
    Header,
}

/// 搜索结果里的一项。
///
/// 直接带上 `draft`：官方源的搜索响应本身已包含 packages/remotes 的全部细节，
/// 客户端点「安装」时不需要再发一次请求，也让「只看本地运行」能在后端过滤。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryCandidate {
    /// 仓库里的完整名称，仅用于展示与去重。
    pub name: String,
    pub description: Option<String>,
    pub version: Option<String>,
    pub repository_url: Option<String>,
    /// 该条目支持的安装方式，UI 用来提示「本地运行」还是「远程连接」。
    pub install_kinds: Vec<RegistryInstallKind>,
    /// 安装草稿；条目没有可用安装方式时为 `None`，UI 应禁用「安装」。
    pub draft: Option<RegistryInstallDraft>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RegistryInstallKind {
    /// 本地包（npx/uvx 等），写入 command + args。
    Package,
    /// 远程服务，写入 url（可能还需要 headers）。
    Remote,
}

/// 安装草稿：表单初值 + 还需用户补的必填项。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryInstallDraft {
    pub name: String,
    pub display_name: String,
    pub kind: McpKind,
    pub config: Value,
    pub env: Value,
    pub headers: Value,
    pub required_fields: Vec<RegistryRequiredField>,
    pub repository_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistrySearchResult {
    pub candidates: Vec<RegistryCandidate>,
    pub next_cursor: Option<String>,
}

// ---------------------------------------------------------------------------
// 解析：仓库条目 → 应用内投影
// ---------------------------------------------------------------------------

/// 仓库名（`io.github.user/server-name`）不是合法的 MCP 键。
///
/// 现有校验只允许 `[A-Za-z0-9_-]`，所以取最后一段并替换非法字符。刻意不自动加序号：
/// 重名时交由 `save_mcp_server` 报错，用户自己改名比面对一个来源不明的后缀更清楚。
pub fn normalize_server_name(raw: &str) -> String {
    let tail = raw.rsplit('/').next().unwrap_or(raw);
    let mut out = String::with_capacity(tail.len());
    let mut last_dash = false;
    for ch in tail.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            out.push(ch);
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    let trimmed = out.trim_matches('-');
    if trimmed.is_empty() {
        "mcp".to_string()
    } else {
        trimmed.to_string()
    }
}

/// 参数数组按顺序取 `value`；`type` 目前只有 positional，其余形式无从表达，忽略。
fn argument_values(args: &[RegistryArgument]) -> Vec<String> {
    args.iter()
        .filter_map(|arg| arg.value.as_deref())
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .collect()
}

fn transport_kind(package: &RegistryPackage) -> McpKind {
    match package.transport.as_ref().and_then(|t| t.kind.as_deref()) {
        Some("sse") => McpKind::Sse,
        Some("http") | Some("streamable-http") => McpKind::Http,
        // 包安装默认按本地进程处理。
        _ => McpKind::Stdio,
    }
}

fn remote_kind(remote: &RegistryRemote) -> McpKind {
    match remote.kind.as_deref() {
        Some("sse") => McpKind::Sse,
        // streamable-http / http / 未知都按 http 写入，客户端会自行协商。
        _ => McpKind::Http,
    }
}

fn required_from_env(vars: &[RegistryEnvVar]) -> Vec<RegistryRequiredField> {
    vars.iter()
        .filter(|var| var.is_required.unwrap_or(false) && var.default.is_none())
        .map(|var| RegistryRequiredField {
            name: var.name.clone(),
            description: var.description.clone(),
            secret: var.is_secret.unwrap_or(false),
            kind: RegistryFieldKind::Env,
        })
        .collect()
}

fn required_from_headers(headers: &[RegistryEnvVar]) -> Vec<RegistryRequiredField> {
    headers
        .iter()
        .filter(|header| header.is_required.unwrap_or(false) && header.default.is_none())
        .map(|header| RegistryRequiredField {
            name: header.name.clone(),
            description: header.description.clone(),
            secret: header.is_secret.unwrap_or(false),
            kind: RegistryFieldKind::Header,
        })
        .collect()
}

/// 带默认值的环境变量直接预填，用户可以改但不必自己想。
fn default_env(vars: &[RegistryEnvVar]) -> Value {
    let mut map = Map::new();
    for var in vars {
        if let Some(default) = &var.default {
            map.insert(var.name.clone(), Value::String(default.clone()));
        }
    }
    Value::Object(map)
}

pub fn to_candidate(server: &RegistryServer) -> RegistryCandidate {
    let mut install_kinds = Vec::new();
    if !server.packages.is_empty() {
        install_kinds.push(RegistryInstallKind::Package);
    }
    if !server.remotes.is_empty() {
        install_kinds.push(RegistryInstallKind::Remote);
    }
    RegistryCandidate {
        name: server.name.clone(),
        description: server.description.clone(),
        version: server.version.clone(),
        repository_url: server.repository.as_ref().and_then(|repo| repo.url.clone()),
        install_kinds,
        // 没有可用安装方式的条目仍然列出来（让用户知道它存在），但不给草稿。
        draft: to_install_draft(server).ok(),
    }
}

/// 该条目是否只跑在用户自己的机器上（本地包，不经过第三方服务器）。
pub fn is_local_only(candidate: &RegistryCandidate) -> bool {
    candidate.draft.as_ref().is_some_and(|draft| {
        draft.kind == McpKind::Stdio && draft.config.get("command").is_some()
    })
}

/// 远程条目会连接到的域名，用于在界面上如实告知请求会发往哪里。
pub fn remote_host(candidate: &RegistryCandidate) -> Option<String> {
    let draft = candidate.draft.as_ref()?;
    let url = draft.config.get("url")?.as_str()?;
    let parsed = url::Url::parse(url).ok()?;
    parsed.host_str().map(str::to_string)
}

/// 把仓库条目换算成安装草稿。
///
/// 优先本地包（`packages`）：它能携带精确的启动命令；没有包时才用远程地址。
/// 两者都没有的条目无法安装，返回可读错误而不是产出半截配置。
pub fn to_install_draft(server: &RegistryServer) -> AppResult<RegistryInstallDraft> {
    let name = normalize_server_name(&server.name);

    if let Some(package) = server.packages.iter().find(|p| {
        p.identifier
            .as_deref()
            .is_some_and(|id| !id.trim().is_empty())
            && p.runtime_hint
                .as_deref()
                .is_some_and(|hint| !hint.trim().is_empty())
    }) {
        let mut args = argument_values(&package.runtime_arguments);
        args.extend(argument_values(&package.package_arguments));
        // runtimeHint 是运行时（npx/uvx），identifier 是包名：缺了包名的条目我们不会走到这里。
        let identifier = package.identifier.clone().unwrap_or_default();
        // 包名通常紧跟运行时参数（`npx -y <pkg>`）——参数在前时把包名插到末尾并不对，
        // 所以统一排成「运行时参数 → 包名 → 包参数」，这与 registry 的示例一致。
        let runtime_len = argument_values(&package.runtime_arguments).len();
        let mut ordered = args[..runtime_len.min(args.len())].to_vec();
        ordered.push(identifier);
        ordered.extend(args[runtime_len.min(args.len())..].iter().cloned());

        let command = package.runtime_hint.clone().unwrap_or_default();
        let mut config = Map::new();
        config.insert("command".into(), Value::String(command));
        config.insert(
            "args".into(),
            Value::Array(ordered.into_iter().map(Value::String).collect()),
        );

        return Ok(RegistryInstallDraft {
            name,
            display_name: server.name.clone(),
            kind: transport_kind(package),
            config: Value::Object(config),
            env: default_env(&package.environment_variables),
            headers: json!({}),
            required_fields: required_from_env(&package.environment_variables),
            repository_url: server.repository.as_ref().and_then(|repo| repo.url.clone()),
        });
    }

    if let Some(remote) = server
        .remotes
        .iter()
        .find(|remote| remote.url.as_deref().is_some_and(|url| !url.trim().is_empty()))
    {
        let mut config = Map::new();
        config.insert(
            "url".into(),
            Value::String(remote.url.clone().unwrap_or_default()),
        );
        return Ok(RegistryInstallDraft {
            name,
            display_name: server.name.clone(),
            kind: remote_kind(remote),
            config: Value::Object(config),
            env: json!({}),
            headers: json!({}),
            required_fields: required_from_headers(&remote.headers),
            repository_url: server.repository.as_ref().and_then(|repo| repo.url.clone()),
        });
    }

    Err(AppError::new(
        "mcp_registry_entry_unsupported",
        format!(
            "'{}' 在官方仓库里没有可用的安装方式（既没有包，也没有远程地址）",
            server.name
        ),
    ))
}

/// 搜索 URL。`cursor` 由仓库返回，原样回传做翻页。
///
/// 必须带 `version=latest`：不带的话仓库会把每个历史版本都作为独立条目返回
/// （实测 100 条结果只对应 20 个服务），用户会看到大量重复。
pub fn search_url(query: &str, cursor: Option<&str>, limit: Option<u32>) -> String {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let mut url = format!("{REGISTRY_BASE}/servers?limit={limit}&version=latest");
    let query = query.trim();
    if !query.is_empty() {
        url.push_str("&search=");
        url.push_str(&urlencode(query));
    }
    if let Some(cursor) = cursor.map(str::trim).filter(|c| !c.is_empty()) {
        url.push_str("&cursor=");
        url.push_str(&urlencode(cursor));
    }
    url
}

pub fn detail_url(name: &str) -> String {
    // 名称里的 `/` 必须编码，否则会被当成路径分隔符。
    format!("{REGISTRY_BASE}/servers/{}", urlencode(name))
}

/// 最小百分号编码：只保留 unreserved 字符，其余（含 `/`）全部编码。
fn urlencode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

/// 解析搜索结果响应，带体积上限。
///
/// 结果按「本地运行优先」排序：本地包跑在用户自己机器上，不把请求交给第三方；
/// 仓库里远程条目占多数，不排序的话第一页很容易整屏都是别人的服务器。
pub fn parse_search_response(bytes: &[u8]) -> AppResult<RegistrySearchResult> {
    if bytes.len() > MAX_RESPONSE_BYTES {
        return Err(AppError::new(
            "mcp_registry_response_too_large",
            "官方仓库返回内容过大",
        ));
    }
    let list: RegistryServerList = serde_json::from_slice(bytes).map_err(|error| {
        AppError::new(
            "mcp_registry_invalid",
            format!("无法解析官方仓库响应: {error}"),
        )
    })?;
    let mut candidates: Vec<RegistryCandidate> =
        list.servers.iter().map(|e| to_candidate(&e.server)).collect();
    dedupe_by_name(&mut candidates);
    // 稳定排序：仅本地在前，其余保持仓库返回顺序。
    candidates.sort_by_key(|candidate| !is_local_only(candidate));
    Ok(RegistrySearchResult {
        candidates,
        next_cursor: parse_next_cursor(bytes),
    })
}

/// 同名条目只留一个（保留先出现的那个）。
///
/// 搜索请求已经带 `version=latest`，正常不会重复；但仓库目前是 preview 服务，
/// 官方明说可能有行为变更，这里兜一层，避免重复条目刷屏。
fn dedupe_by_name(candidates: &mut Vec<RegistryCandidate>) {
    let mut seen = std::collections::HashSet::new();
    candidates.retain(|candidate| seen.insert(candidate.name.clone()));
}

/// 按安装方式过滤候选。「只看本地运行」由界面默认开启。
pub fn filter_candidates(
    candidates: Vec<RegistryCandidate>,
    local_only: bool,
) -> Vec<RegistryCandidate> {
    if local_only {
        candidates
            .into_iter()
            .filter(is_local_only)
            .collect()
    } else {
        candidates
    }
}

/// `metadata.nextCursor` 是非必需字段，单独松散解析，避免为一个游标让整个结构变脆。
fn parse_next_cursor(bytes: &[u8]) -> Option<String> {
    let value: Value = serde_json::from_slice(bytes).ok()?;
    value
        .get("metadata")?
        .get("nextCursor")?
        .as_str()
        .filter(|cursor| !cursor.is_empty())
        .map(str::to_string)
}

/// 解析详情响应（`GET /servers/{name}` 返回单个 server 对象）。
pub fn parse_detail_response(bytes: &[u8]) -> AppResult<RegistryServer> {
    if bytes.len() > MAX_RESPONSE_BYTES {
        return Err(AppError::new(
            "mcp_registry_response_too_large",
            "官方仓库返回内容过大",
        ));
    }
    // 详情端点可能直接返回 server，也可能包一层 `{ server: {...} }`，两种都接受。
    if let Ok(entry) = serde_json::from_slice::<RegistryServerEntry>(bytes) {
        return Ok(entry.server);
    }
    serde_json::from_slice::<RegistryServer>(bytes).map_err(|error| {
        AppError::new(
            "mcp_registry_invalid",
            format!("无法解析官方仓库条目: {error}"),
        )
    })
}

pub fn max_response_bytes() -> usize {
    MAX_RESPONSE_BYTES
}

#[cfg(test)]
mod tests {
    use super::*;

    fn server(json_str: &str) -> RegistryServer {
        serde_json::from_str(json_str).unwrap()
    }

    const NPM_PACKAGE: &str = r#"{
      "name": "com.example/filesystem",
      "description": "Local filesystem access",
      "version": "1.2.3",
      "repository": { "url": "https://github.com/example/fs" },
      "packages": [{
        "registryType": "npm",
        "identifier": "@modelcontextprotocol/server-filesystem",
        "version": "1.2.3",
        "runtimeHint": "npx",
        "transport": { "type": "stdio" },
        "runtimeArguments": [{ "value": "-y", "type": "positional" }],
        "packageArguments": [{ "value": "/tmp", "type": "positional" }],
        "environmentVariables": [
          { "name": "API_KEY", "isRequired": true, "isSecret": true, "description": "token" },
          { "name": "MODE", "default": "readonly" },
          { "name": "OPTIONAL", "description": "not required" }
        ]
      }]
    }"#;

    const REMOTE_ONLY: &str = r#"{
      "name": "ai.example/memory",
      "description": "Hosted memory",
      "version": "0.4.0",
      "remotes": [{
        "type": "streamable-http",
        "url": "https://mcp.example.ai",
        "headers": [
          { "name": "Authorization", "isRequired": true, "isSecret": true },
          { "name": "X-Trace", "isRequired": false }
        ]
      }]
    }"#;

    #[test]
    fn normalizes_registry_names_into_legal_mcp_keys() {
        // 斜杠、点号都不是合法的 MCP 键字符；取最后一段并规范化。
        assert_eq!(normalize_server_name("io.github.upstash/context7-mcp"), "context7-mcp");
        assert_eq!(normalize_server_name("ai.smithery/Foo.Bar"), "Foo-Bar");
        assert_eq!(normalize_server_name("plain"), "plain");
        assert_eq!(normalize_server_name("a//b"), "b");
        // 连续非法字符合并成一个短横线，且不保留首尾短横线。
        assert_eq!(normalize_server_name("x/a...b"), "a-b");
        assert_eq!(normalize_server_name("!!!///@@@"), "mcp");
        assert_eq!(normalize_server_name(""), "mcp");
    }

    #[test]
    fn npm_package_becomes_stdio_command_with_ordered_args() {
        let draft = to_install_draft(&server(NPM_PACKAGE)).unwrap();
        assert_eq!(draft.kind, McpKind::Stdio);
        assert_eq!(draft.name, "filesystem");
        assert_eq!(draft.config["command"], "npx");
        // 顺序必须是「运行时参数 → 包名 → 包参数」，否则 npx 找不到包。
        assert_eq!(
            draft.config["args"],
            json!(["-y", "@modelcontextprotocol/server-filesystem", "/tmp"])
        );
        // 有默认值的环境变量直接预填，用户不必自己想。
        assert_eq!(draft.env["MODE"], "readonly");
        assert_eq!(draft.repository_url.as_deref(), Some("https://github.com/example/fs"));
    }

    #[test]
    fn required_env_excludes_those_with_defaults_and_marks_secrets() {
        let draft = to_install_draft(&server(NPM_PACKAGE)).unwrap();
        assert_eq!(draft.required_fields.len(), 1, "only API_KEY is required without default");
        let field = &draft.required_fields[0];
        assert_eq!(field.name, "API_KEY");
        assert!(field.secret);
        assert_eq!(field.kind, RegistryFieldKind::Env);
        // 非必填、带默认值的都不该出现在待填列表里。
        assert!(!draft.required_fields.iter().any(|f| f.name == "OPTIONAL"));
        assert!(!draft.required_fields.iter().any(|f| f.name == "MODE"));
    }

    #[test]
    fn remote_entry_becomes_http_with_required_headers() {
        let draft = to_install_draft(&server(REMOTE_ONLY)).unwrap();
        assert_eq!(draft.kind, McpKind::Http);
        assert_eq!(draft.config["url"], "https://mcp.example.ai");
        assert!(draft.config.get("command").is_none());
        assert_eq!(draft.required_fields.len(), 1);
        assert_eq!(draft.required_fields[0].name, "Authorization");
        assert_eq!(draft.required_fields[0].kind, RegistryFieldKind::Header);
        assert!(draft.required_fields[0].secret);
    }

    #[test]
    fn entry_without_installable_transport_is_rejected() {
        let error = to_install_draft(&server(
            r#"{ "name": "io.example/empty", "description": "nothing" }"#,
        ))
        .unwrap_err();
        assert!(error.to_string().contains("没有可用的安装方式"));
    }

    #[test]
    fn package_without_runtime_hint_falls_back_to_remote() {
        // 有包但缺 runtimeHint 时无法拼出可运行命令，应该退回远程而不是产出半截命令。
        let draft = to_install_draft(&server(
            r#"{
              "name": "io.example/both",
              "packages": [{ "identifier": "some-pkg" }],
              "remotes": [{ "type": "sse", "url": "https://example.com/sse" }]
            }"#,
        ))
        .unwrap();
        assert_eq!(draft.kind, McpKind::Sse);
        assert_eq!(draft.config["url"], "https://example.com/sse");
    }

    #[test]
    fn candidate_lists_both_install_kinds() {
        let candidate = to_candidate(&server(
            r#"{
              "name": "io.example/both",
              "packages": [{ "identifier": "p", "runtimeHint": "npx" }],
              "remotes": [{ "url": "https://example.com/mcp" }]
            }"#,
        ));
        assert_eq!(
            candidate.install_kinds,
            vec![RegistryInstallKind::Package, RegistryInstallKind::Remote]
        );
        // 有本地包时按本地安装：请求不经过第三方。
        assert!(is_local_only(&candidate));
        assert!(candidate.draft.is_some());

        let remote_only = to_candidate(&server(REMOTE_ONLY));
        assert_eq!(remote_only.install_kinds, vec![RegistryInstallKind::Remote]);
        assert!(!is_local_only(&remote_only));
    }

    #[test]
    fn candidate_for_uninstallable_entry_has_no_draft() {
        // 没有可用安装方式的条目仍要能被搜到并展示，但不能让用户点安装。
        let candidate = to_candidate(&server(r#"{ "name": "io.example/empty" }"#));
        assert!(candidate.draft.is_none());
        assert!(candidate.install_kinds.is_empty());
        assert!(!is_local_only(&candidate));
    }

    #[test]
    fn remote_host_reports_where_requests_go() {
        // 界面要如实说明请求发往哪个域名，而不是含糊地说「远程服务」。
        let candidate = to_candidate(&server(
            r#"{
              "name": "io.example/hosted",
              "remotes": [{ "type": "streamable-http", "url": "https://mcp.example.ai/v1" }]
            }"#,
        ));
        assert_eq!(remote_host(&candidate).as_deref(), Some("mcp.example.ai"));

        let local = to_candidate(&server(NPM_PACKAGE));
        assert!(remote_host(&local).is_none(), "local packages have no remote host");
    }

    #[test]
    fn search_results_put_local_entries_first() {
        let body = br#"{
          "servers": [
            { "server": { "name": "io.example/remote", "remotes": [{ "url": "https://a.example.com/mcp" }] } },
            { "server": { "name": "io.example/local", "packages": [{ "identifier": "pkg", "runtimeHint": "npx" }] } }
          ]
        }"#;
        let result = parse_search_response(body).unwrap();
        assert_eq!(result.candidates.len(), 2);
        assert_eq!(
            result.candidates[0].name, "io.example/local",
            "local packages must sort first"
        );
        assert_eq!(result.candidates[1].name, "io.example/remote");
    }

    #[test]
    fn local_only_filter_keeps_only_own_machine_entries() {
        let body = br#"{
          "servers": [
            { "server": { "name": "io.example/remote", "remotes": [{ "url": "https://a.example.com/mcp" }] } },
            { "server": { "name": "io.example/local", "packages": [{ "identifier": "pkg", "runtimeHint": "npx" }] } },
            { "server": { "name": "io.example/none" } }
          ]
        }"#;
        let result = parse_search_response(body).unwrap();

        let filtered = filter_candidates(result.candidates.clone(), true);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "io.example/local");

        // 关闭过滤时全部保留，含无法安装的条目。
        let all = filter_candidates(result.candidates, false);
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn sse_transport_is_preserved_for_packages() {
        let draft = to_install_draft(&server(
            r#"{
              "name": "io.example/sse-pkg",
              "packages": [{
                "identifier": "pkg",
                "runtimeHint": "uvx",
                "transport": { "type": "sse" }
              }]
            }"#,
        ))
        .unwrap();
        assert_eq!(draft.kind, McpKind::Sse);
        assert_eq!(draft.config["args"], json!(["pkg"]));
    }

    #[test]
    fn search_url_encodes_query_and_cursor() {
        let url = search_url("filesystem server", Some("a/b:c"), Some(20));
        assert!(url.starts_with("https://registry.modelcontextprotocol.io/v0.1/servers?limit=20"));
        assert!(url.contains("search=filesystem%20server"), "{url}");
        // 游标里的斜杠必须编码，否则会被当成路径段。
        assert!(url.contains("cursor=a%2Fb%3Ac"), "{url}");
        // 不带 version=latest 时仓库会为每个历史版本返回一条，界面全是重复项。
        assert!(url.contains("version=latest"), "{url}");
    }

    #[test]
    fn duplicate_versions_collapse_to_one_candidate() {
        // 实测仓库会对同一服务返回多个历史版本；即使请求漏了 version=latest，
        // 解析也不该把同一个服务重复展示给用户。
        let body = br#"{
          "servers": [
            { "server": { "name": "io.example/dup", "version": "1.0.0",
              "packages": [{ "identifier": "pkg", "runtimeHint": "npx" }] } },
            { "server": { "name": "io.example/dup", "version": "1.0.1",
              "packages": [{ "identifier": "pkg", "runtimeHint": "npx" }] } },
            { "server": { "name": "io.example/other",
              "packages": [{ "identifier": "pkg2", "runtimeHint": "uvx" }] } }
          ]
        }"#;
        let result = parse_search_response(body).unwrap();
        assert_eq!(result.candidates.len(), 2, "one entry per unique server name");
        assert_eq!(result.candidates[0].name, "io.example/dup");
        assert_eq!(result.candidates[1].name, "io.example/other");
    }

    #[test]
    fn search_url_omits_empty_query_and_clamps_limit() {
        let url = search_url("   ", None, Some(999));
        assert!(url.contains("limit=50"), "limit must be clamped: {url}");
        assert!(!url.contains("search="), "{url}");
        assert!(!url.contains("cursor="), "{url}");
    }

    #[test]
    fn detail_url_encodes_the_name_slash() {
        let url = detail_url("io.github.x/y");
        assert_eq!(
            url,
            "https://registry.modelcontextprotocol.io/v0.1/servers/io.github.x%2Fy"
        );
    }

    #[test]
    fn parses_search_response_with_cursor() {
        let body = br#"{
          "servers": [{ "server": { "name": "io.example/a", "description": "d" } }],
          "metadata": { "nextCursor": "io.example/a:1", "count": 1 }
        }"#;
        let result = parse_search_response(body).unwrap();
        assert_eq!(result.candidates.len(), 1);
        assert_eq!(result.candidates[0].name, "io.example/a");
        assert_eq!(result.next_cursor.as_deref(), Some("io.example/a:1"));
    }

    #[test]
    fn empty_cursor_is_reported_as_none() {
        let body = br#"{ "servers": [], "metadata": { "count": 0 } }"#;
        let result = parse_search_response(body).unwrap();
        assert!(result.candidates.is_empty());
        assert!(result.next_cursor.is_none());
    }

    #[test]
    fn malformed_response_is_reported_not_panicking() {
        let error = parse_search_response(b"{ not json").unwrap_err();
        assert!(error.to_string().contains("无法解析"));
    }

    #[test]
    fn oversized_response_is_rejected() {
        let body = vec![b' '; max_response_bytes() + 1];
        let error = parse_search_response(&body).unwrap_err();
        assert!(error.to_string().contains("过大"));
        assert!(parse_detail_response(&body).is_err());
    }

    #[test]
    fn detail_accepts_both_wrapped_and_bare_shapes() {
        let wrapped = br#"{ "server": { "name": "io.example/a", "packages": [] } }"#;
        let bare = br#"{ "name": "io.example/a", "packages": [] }"#;
        assert_eq!(parse_detail_response(wrapped).unwrap().name, "io.example/a");
        assert_eq!(parse_detail_response(bare).unwrap().name, "io.example/a");
    }
}
