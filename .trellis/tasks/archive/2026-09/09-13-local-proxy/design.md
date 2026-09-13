# 技术设计

## 1. 总体结构

新增 Rust 模块 `src-tauri/src/local_proxy/`：

| 文件 | 职责 |
|------|------|
| `mod.rs` | 模块装配、`ProxyStatus` / `ProxyTargetStatus` DTO、设置项读写辅助、退出时恢复入口 |
| `routing.rs` | 纯路由计算：目标 → 真实上游 / 广告给客户端的代理地址 / 剩余路径；接管时的 `SiteRow` 克隆替换 |
| `headers.rs` | 请求头校验、受保护头名单、占位符替换、注入 |
| `log.rs` | 定长请求日志环形缓冲（200 条） |
| `server.rs` | 监听、连接循环、启停生命周期、统计计数 |
| `forward.rs` | 构造上游请求、流式双向转发、错误响应 |

依赖新增（均为本地 cargo 缓存已有的 reqwest 传递依赖，无需联网）：
`hyper = { version = "1", features = ["http1", "server"] }`、`hyper-util = { version = "0.1", features = ["tokio"] }`、`http-body-util = "0.1"`、`http = "1"`、`bytes = "1"`。服务端用 hyper 1 的 HTTP/1.1（本地客户端一律 HTTP/1.1；不支持 h2c，列入不做）。

## 2. URL 路由（核心决策）

监听 `http://127.0.0.1:{port}`。对目标 T 广告给客户端的 Base URL 形如：

```
http://127.0.0.1:{port}/{token}/t/{target}{suffix}
```

- `{token}`：安装级随机 32 hex，首次启动生成并持久化到设置；不匹配 → 404（不泄露任何信息）。作用是把"能用这个代理"限制在读过客户端配置的进程，防止本机其他程序误用站点密钥。
- `{target}`：`TargetKind::as_str()`（`claude_code` / `codex` / `pi` / `prime`）。
- `{suffix}`：**镜像真实 Base URL 的 `/v1` 形状**——先对真实 base 调 `normalize_base_url` 得到该目标使用的真实上游 base（claude 目标与 anthropic 协议用 `claude_base_url`；codex 目标与 openai 协议用 `codex_base_url`），若它以 `/v1` 结尾则 `{suffix} = "/v1"`，否则为空。随后把 `origin + /{token}/t/{target} + suffix` 再过一次**同一个** `normalize_base_url`，取同一种 base 作为广告值。

转发规则：**上游 URL = 真实上游 base + (客户端请求 path 去掉广告 base 的 path 前缀) + 原查询串**。因为两侧都由同一个 normalize 产出、且 `/v1` 形状被镜像，剩余路径与"直连模式下客户端追加的部分"逐字节一致。四种目标 × 两种协议的对照：

| 目标 / 站点协议 | 数据库 base | 真实上游 base | 广告给客户端 | 客户端实际请求 | 上游最终 URL |
|---|---|---|---|---|---|
| claude_code | `https://relay/anthropic` | `https://relay/anthropic` | `…/t/claude_code` | `…/t/claude_code/v1/messages` | `https://relay/anthropic/v1/messages` |
| claude_code | `https://relay/v1` | `https://relay/v1` | `…/t/claude_code/v1` | `…/t/claude_code/v1/messages` | `https://relay/v1/messages` |
| codex | `https://relay` | `https://relay/v1` | `…/t/codex` → normalize 后 `…/t/codex/v1` | `…/t/codex/v1/responses` | `https://relay/v1/responses` |
| codex | `https://relay/v1` | `https://relay/v1` | `…/t/codex/v1` | `…/t/codex/v1/responses` | `https://relay/v1/responses` |
| pi / prime（anthropic） | `https://relay` | `https://relay` | `…/t/pi` | `…/t/pi/v1/messages` | `https://relay/v1/messages` |
| pi / prime（openai） | `https://relay` | `https://relay/v1` | `…/t/pi/v1` | `…/t/pi/v1/chat/completions` | `https://relay/v1/chat/completions` |

实现要点：
- 只用请求 path 选择"目标"（进而选数据库里绑定的站点），**绝不接受请求侧指定的任意 URL**：上游永远来自 `target_bindings` → `sites`，仅允许 http/https（沿用 `normalize_base_url` 既有校验）。这是 SSRF 边界。
- 目标当前绑定的站点：`repo::binding::get_binding(conn, target)`；无绑定 / orphan / 站点已删 → 502 + JSON 错误体。
- 站点按请求实时解析，所以"换站点/换线路"天然热生效，无需重写客户端配置。

## 3. 接管注入点

不修改各适配器的 URL 推导逻辑，而是在**调用适配器之前**克隆 `SiteRow` 并把 `base_url` 换成代理地址：

```rust
// routing.rs
pub fn takeover_targets(settings: &AppSettings) -> &[TargetKind];
pub fn effective_site(site: &SiteRow, target: TargetKind, settings: &AppSettings) -> SiteRow;
```

`effective_site` 在接管未开启、或 token/端口缺失时原样返回克隆。调用点（全部集中在既有链路，共 3 处 × 4 目标）：

- `commands/apply.rs` 每个目标分支取 `let site_t = local_proxy::routing::effective_site(&site, target, &settings);` 后传给适配器；
- `key_switch.rs` 的 `apply_claude/apply_codex/apply_pi/apply_prime`（`settings` 已在作用域）；
- `route_switch.rs::sync_applied_urls` 每个 binding 分支（重写路径）。

好处：适配器、`expected_fields`、`detect_status`、`rewrite_base_url`、备份与自检全部沿用原逻辑，代理开关切换等价于"站点 base_url 变了"，由既有 `sync_applied_urls` 机制完成重写；数据库里始终保存真实上游（SSOT 不被污染），模型探测/额度查询继续直连真实上游。

新增 `route_switch::sync_applied_target(state, target)`（按目标重写其唯一 binding），与 `sync_applied_urls` 共享抽出的 `rewrite_binding(...)`。

## 4. 请求头改写

站点级有序列表：`ProxyHeader { name: String, value: String, enabled: bool }`（数据库加密存储；`sites.proxy_header_count` 明文计数供列表展示）。

- 校验：name 必须匹配 RFC 9110 token（`^[!#$%&'*+\-.^_`|~0-9A-Za-z]+$`）；value 不得含控制字符（除 tab）；`enabled=false` 跳过。
- 受保护头（保存与转发两层都拒绝）：`host`、`content-length`、`transfer-encoding`、`connection`、`keep-alive`、`te`、`trailer`、`upgrade`、`proxy-authorization`、`proxy-authenticate`、`content-type`。允许覆盖 `authorization` / `x-api-key`（这正是"认证头名不匹配"场景的解法）。
- 占位符（仅在转发时按请求替换，不落库）：
  - `${API_KEY}` → 该站点当前激活密钥（内存解密，禁止进日志）；
  - `${SESSION}` → `sha256("xiaobai-proxy-session|" + site_id)` 前 32 hex，按站点稳定、跨重启不变（供 OpenCode Go 之类做 provider 亲和/缓存命中）；
  - `${UUID}` → 每请求新 v4 UUID。
- 客户端原始请求头默认全部透传；改写只做新增/覆盖。丢弃 hop-by-hop 头与 `host`/`content-length`（由客户端重算），保留 `accept-encoding` 与 `content-encoding` 透传压缩语义。

## 5. 转发与流式

- 请求体：`http_body_util::BodyStream` → `try_filter_map(frame.into_data())` → `reqwest::Body::wrap_stream`，不整体缓冲。
- 响应体：`hyper::body::Body::from_stream(resp.bytes_stream())`，逐块透传（SSE 必须保证首字节不被缓冲）。
- 上游 client（启动时构造一次）：`connect_timeout(15s)`、`read_timeout(180s)`、`pool_idle_timeout(90s)`、`redirect(limited(3))`、**不设总超时**；出站代理沿用 `http_client::resolve_proxy` / `apply_resolved_proxy`，并加自环保护（自定义代理指向本机代理端口时忽略并 warn）。
- 响应头：透传上游头，去掉 hop-by-hop 与 `transfer-encoding`；上游给出 `content-length` 时带上，否则 chunked。
- 错误：绑定缺失/站点缺失 → 502；token 不匹配 → 404；上游连接失败 → 502；均返回 `{"error":{"type":"proxy_error","message":"…"}}`，消息中不得包含密钥；对应日志记录 status 与错误摘要。
- 日志条目 `{id, at, target, method, path, status, duration_ms, error}`：path 记录**去掉 token 前缀后的剩余路径**，不记录任何 header/body。

## 6. 设置与数据库

`AppSettings` 新增（全部 `#[serde(default)]`，旧 settings JSON 可解析）：

| 字段 | 默认 | 说明 |
|---|---|---|
| `localProxyEnabled` | `false` | 运行意图；应用启动时自动拉起 |
| `localProxyPort` | `18087` | 监听端口，normalize 夹到 1024..=65535 |
| `localProxyTargets` | `[]` | 接管目标集合，normalize 去重保序 |
| `localProxyPathToken` | `None` | 首次启动生成 32 hex 并持久化 |

`repo/settings.rs::normalize` 增加端口夹取与目标去重（保持纯函数，不写库）。

数据库：`SCHEMA_VERSION` 2 → 3。`sites` 增列 `proxy_headers_encrypted TEXT`、`proxy_header_count INTEGER NOT NULL DEFAULT 0`；同时更新 `V1_SCHEMA`（新库）与 `ensure_incremental_schema` 新增的 `ensure_sites_proxy_headers_columns`（老库）——按 AGENTS.md，所有"库已存在"分支都要走增量补齐。

站点命令：`get_site_proxy_headers(site_id)` 按需解密返回；`CreateSiteInput` / `UpdateSiteInput` 增加 `proxy_headers: Option<Vec<ProxyHeader>>`（`None` = 不改，`Some` = 覆盖并重算计数）；`SiteDto.proxy_header_count` 供列表/表单展示，列表不返回明文。

## 7. 生命周期与退出

- `AppState.local_proxy: tokio::sync::Mutex<ProxyRuntime>`；`ProxyRuntime { shutdown_tx, join, started_at, stats: Arc<ProxyStats>, last_error }`。
- 命令：`start_local_proxy` / `stop_local_proxy` / `local_proxy_status` / `set_local_proxy_takeover` / `list_local_proxy_requests`。
- `setup()` 中 bootstrap：`localProxyEnabled == true` 则启动（失败只记日志，不阻塞应用启动）。
- 退出：`ExitRequested` 处理器里，若接管非空则尽力对每个接管目标执行"关接管 + `sync_applied_target`"恢复直连（失败仅记日志，不阻塞退出），然后停止监听。
- 端口被占用返回 `AppError::new("proxy_bind_failed", …)`；已在运行时重复 start 返回当前状态（幂等）。

## 8. 前端

- 新增页面 `src/pages/ProxyPage.tsx`（侧栏 `Network` 图标，`AppPage` 增 `"proxy"`，`App.tsx` KeepAlive 挂载，`nav.proxy` + `proxy.*` 文案 zh-CN/en-US 双份）。
  - 状态卡：启停 Switch、监听地址、运行时长、总数/成功/失败/活跃连接、最近错误、"打开我时自动刷新"。
  - 接管卡：四目标行（图标 + 名称 + 当前绑定站点名 + 代理地址预览 + Switch），提示"应用需保持运行；退出时会自动恢复直连"。
  - 请求日志卡：表（时间/目标/路径/状态/耗时/错误），刷新按钮 + 页面可见时轮询（沿用 SitesPage 的 visibilitychange 模式）。
- `src/stores/proxyStore.ts`：状态与动作，遵循 mcpStore 约定（invoke + loading，错误抛给页面弹 message）。
- 站点编辑 `SiteFormModal` 高级配置内新增"本地代理请求头"JSON 文本域（与 MCP 表单同款 `parseJsonObject` 校验风格），编辑时调 `get_site_proxy_headers` 回填，保存走 `create_site` / `update_site` 的 `proxyHeaders`；文案说明支持的三个占位符与"仅对走本地代理的请求生效"。
- `src/types/domain.ts`、`src/lib/browserMock.ts`（DEFAULT_SETTINGS + 各新命令 case）、`src/stores/settingsStore.ts` DEFAULT 三处同步加字段。

## 9. 安全

- 仅回环监听（host 固定 `127.0.0.1`），不提供局域网/公网暴露。
- 上游仅来自数据库；仅 http/https；不做请求侧 URL 透传。
- 日志与错误不出现密钥；`${API_KEY}` 只在内存中替换；请求日志不含 header/body；token 在日志路径中脱敏。
- 请求头密文入库（`Crypto`），列表只回计数，明文只经 `get_site_proxy_headers` 按需返回。

## 10. 测试矩阵

Rust 单测：routing（上表 6 种组合的广告地址 + 剩余路径 + 上游 URL 拼接；token 匹配/不匹配；接管开关的 `effective_site` 行为）、headers（非法名/值拒绝、受保护头拒绝、三个占位符、`${SESSION}` 稳定性、`enabled=false`）、settings normalize（端口夹取、目标去重、旧 JSON 默认值）、迁移（新库有列、v2 老库补齐）、加密往返。
Rust 集成测试（`#[tokio::test]` + `tokio::net::TcpListener` mock 上游，参照 `models_fetch` 现有写法）：注入的请求头真的出现在上游收到的请求里；SSE 分块顺序与内容一致；无绑定 → 502；token 错误 → 404。
前端：`proxyStore` / `ProxyPage`（启停、接管开关、日志渲染、错误提示）测试 + 站点请求头 JSON 校验测试；`pnpm typecheck`、`pnpm test:run`、`cargo test` 全绿。

## 11. 兼容与回滚

- 不触碰任何协议红线（`xiaobai_` 命名空间、备份前缀、WebDAV manifest/远端目录、`FINGERPRINT_TABLES` 现有成员）。
- 新增列走增量补齐；旧备份/旧版本仍可恢复。回滚方式：不配置接管即为完全无感；卸载/降级新版本后旧版本读到多余列不影响解析（SELECT 显式列名）。
