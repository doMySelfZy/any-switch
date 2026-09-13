# 调研记录（外部 + 代码落点）

## 1. CC Switch 的本地代理（借鉴对象）

- 架构：Tauri + axum 本地 HTTP 服务，默认 `127.0.0.1:15721`；`server.rs` 管生命周期，`forwarder.rs` 管转发与请求头/请求体覆盖，`handlers.rs` 按应用分发，`circuit_breaker.rs` / `failover_switch.rs` 管熔断与故障转移。
- 接管方式：改写各 CLI 的 live 配置——Claude `~/.claude/settings.json` 的 `ANTHROPIC_BASE_URL`、Codex `config.toml` 的 `base_url`、Gemini `.env`；改写前备份 live 配置，停止代理时恢复。
- 请求头覆盖（PR #4589）踩过的坑，直接作为我们的设计输入：
  1. header 名必须按 HTTP token 校验，后端 `HeaderName::from_bytes` 失败会**静默丢弃**；
  2. 保护名单要含 `content-type`（否则 JSON body 被标成 `text/plain`）、`authorization`、`x-api-key` 等；
  3. body 覆盖 `stream` 会让流式/非流式路径与客户端预期脱节 → 我们**不做 body 覆盖**；
  4. header 名大小写归一后重复要报错，否则 HashMap 迭代顺序导致结果不确定；
  5. 头大小写与 wire 级一致性：CC Switch 用 `preserve_header_case` 兜底；我们不在 v1 做（列入不做）。
- 另有协议转换（Anthropic ⇄ OpenAI Chat/Responses）与提示缓存注入，工作量大，本次明确不做。

## 2. 目标场景：OpenCode Go 的请求头要求

- 官方文档（opencode.ai/docs/go）：客户端应"用自己的 UA 标识（如 `my-coding-agent/1.0`）而不是通用 SDK/HTTP 库名"，并"为每段会话发送稳定的 `x-opencode-session`"，用于 provider 亲和与提示缓存。
- 缺头时的实际报错：`400 Model is unavailable`（部分后端）或 `Request is missing x-opencode-session and cannot be routed efficiently`；有社区实践在客户端配置里硬编码 `x-opencode-session: <固定值>`（如 pi issue #4847），代价是绑定安装而非会话。
- 另有 Cloudflare 403 案例：数据中心 IP 上用通用 UA 访问 `/zen/go/v1/chat/completions` 被拦，需要 `User-Agent: opencode-cli/1.0.0` + `x-opencode-client: cli` 等身份头；说明"UA / 身份头可配置"是必需的。
- 结论：v1 需要"自定义请求头（可覆盖 UA）+ 稳定会话值"两件事即可覆盖主诉；占位符 `${SESSION}` 提供按站点稳定值，避免硬编码的安装级绑定问题（同一站点确定性派生，跨重启不变）。

## 3. 本仓库落点（已核对源码）

- 后端：Tauri 2 + tokio(full) + reqwest 0.12（rustls + socks + stream，无 gzip 自动解压 → 压缩语义天然透传）；**没有 axum/hyper 直接依赖**，但 cargo 缓存与 `Cargo.lock` 已有 hyper 1.11 / hyper-util 0.1.20 / http-body-util 0.1.4 / http 1.5 / bytes 1.12（reqwest 传递依赖），可直接提升为直接依赖，无需联网下载。
- 无 `tests/` 目录，测试一律文件内 `#[cfg(test)] mod tests`；异步测试 `#[tokio::test]`；mock HTTP 上游用 `tokio::net::TcpListener` 手写（`models_fetch/mod.rs` 是范例）。
- `AppState`（`state.rs`）：`db`/`crypto` + WebDAV 任务句柄，可照抄"可启停常驻任务"模式（`commands/webdav.rs` 存 `JoinHandle`、重启前 `abort`）。
- 错误：`AppError::Coded{code,message,details}` 序列化为结构体；命令返回 `AppResult<T>`；前端 `errorText()` 取 `message`。
- 日志：`tracing`，无前端日志回显 → 代理日志走"内存环形缓冲 + 查询命令"。
- 路径推导：四适配器都经 `url_normalize::normalize_base_url(&site.base_url)` 得到 `claude_base_url` / `codex_base_url`（claude 目标与 anthropic 协议用前者；codex 目标与 openai 协议用后者）。
- 适配器调用点：`commands/apply.rs`（4 目标分支）、`key_switch.rs`（4 个 apply_*）、`route_switch.rs::sync_applied_urls`（4 目标重写）；`tray_apply.rs` 不直接调适配器。→ 接管注入只需这 3 处。
- 设置：单行 settings.json（`AppSettings`，camelCase，`preview_merge` 局部合并），新增字段加 `#[serde(default)]` 即可被旧数据兼容；`repo/settings.rs::normalize` 做夹取。
- 数据库：`SCHEMA_VERSION` 2；`sites` 表；迁移红线——每个"库已存在"分支都要跑 `ensure_incremental_schema`（AGENTS.md 明文要求）。
- 密钥：`Crypto`（AES-256-GCM）加密入库；MCP 先例是"列表只返回摘要、明文按需 `get_mcp_server` 取"，站点请求头沿用该模式。
- 同步指纹：`FINGERPRINT_TABLES` 已含 `settings` 与 `sites` → 新增字段自然进入同步指纹，无需改动同步协议。
- 前端：无 router，`uiStore.AppPage` + `KeepAlivePages`；页面壳复用 `SettingsGroup`；store 惯例（mcpStore）；i18n 双 locale 顶层块；测试用 `resetBrowserMock()` + browserMock 命令分发（新命令必须补 case，否则浏览器模式报 internal）。
