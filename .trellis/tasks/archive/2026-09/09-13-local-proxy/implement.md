# 实现计划

按顺序执行；每步完成后跑对应验证，失败先修再往下（不要带着红灯前进）。

## 1. 依赖与数据层（后端地基）

1. `src-tauri/Cargo.toml` 增加 `hyper`(http1,server) / `hyper-util`(tokio) / `http-body-util` / `http` / `bytes`。
2. `domain`：新增 `ProxyHeader`（camelCase）、`ProxyStatus` / `ProxyTargetStatus` DTO；`AppSettings` 增 4 个字段 + 默认值；`SiteDto.proxy_header_count`；`CreateSiteInput` / `UpdateSiteInput` 增 `proxy_headers`。
3. `db/migrate.rs`：`SCHEMA_VERSION` → 3；`V1_SCHEMA.sites` 增 `proxy_headers_encrypted`、`proxy_header_count`；新增 `ensure_sites_proxy_headers_columns` 并挂进 `ensure_incremental_schema`。
4. `repo/site.rs`：`SiteRow` 增两字段并在所有 SELECT/INSERT/UPDATE 中同步；`proxy_headers` 的校验、加密、计数落库；`get_site_proxy_headers` 解密读取。
5. `repo/settings.rs::normalize`：端口夹取 + 目标去重。
6. `commands/sites.rs` + `commands/mod.rs` + `lib.rs` 注册 `get_site_proxy_headers`。

验证：`cargo test`（迁移与设置的新老库用例必须先绿）。

## 2. local_proxy 核心（纯逻辑优先）

7. `local_proxy/routing.rs`：`proxy_origin`、`takeover_targets`、`effective_site`、`client_base_url`、`resolve_route`（广告地址/剩余路径/上游 URL），配 design.md 第 2 节 6 种组合的表格测试。
8. `local_proxy/headers.rs`：校验、受保护头、占位符（`${API_KEY}` / `${SESSION}` / `${UUID}`）、`apply_overrides` 测试（含 `${SESSION}` 跨调用稳定）。
9. `local_proxy/log.rs`：200 条环形缓冲 + 脱敏路径格式化。

验证：`cargo test local_proxy` 全绿。

## 3. 服务端与转发

10. `local_proxy/server.rs`：`start(app)`（token 生成、绑定、accept 循环、每连接 `http1::Builder` + `TokioIo`）、`stop()`、`ProxyRuntime` 入 `AppState`、统计计数、`ExitRequested` 尽力恢复 + 停止。
11. `local_proxy/forward.rs`：上游请求构造（流式 body、header 拷贝与 hop-by-hop 剥离、自环保护）、响应流式回写、404/502 错误体。
12. `commands/proxy.rs`：`start_local_proxy` / `stop_local_proxy` / `local_proxy_status` / `set_local_proxy_takeover` / `list_local_proxy_requests`；`set_local_proxy_takeover` 内部调用 `route_switch::sync_applied_target` 重写该目标配置。
13. `route_switch.rs`：抽出 `rewrite_binding`，新增 `sync_applied_target`；`lib.rs` setup 里 bootstrap 自动启动。

验证：`cargo test`（含 mock 上游的集成测试：注入头到上游、SSE 分块、502/404、日志条目）。

## 4. 接管注入

14. `commands/apply.rs`、`key_switch.rs`、`route_switch.rs::sync_applied_urls` 三处接入 `effective_site`（仅在接管开启时替换 `site.base_url`）。

验证：`cargo test`；补一个测试断言"接管开启时适配器写入的是代理地址、关闭时是真实上游"。

## 5. 前端

15. `types/domain.ts` + `browserMock.ts` + `settingsStore.ts` 三处同步新字段与新命令 case。
16. `stores/proxyStore.ts` + `pages/ProxyPage.tsx` + `uiStore`(AppPage) + `App.tsx` KeepAlive + `SideNav` 入口。
17. `SiteFormModal` 高级配置加请求头 JSON 编辑（含占位符说明）。
18. i18n zh-CN / en-US 双份 `nav.proxy` 与 `proxy.*`。

验证：`pnpm typecheck`、`pnpm test:run`（新增 proxyStore / ProxyPage / 表单校验用例）。

## 6. 复核与收尾

19. 派复核代理（`trellis-check`）按 AGENTS.md 红线与本任务验收标准逐条核对，重点：SSRF 边界、密钥不出日志、迁移分支补齐、四目标一致性、流式不缓冲、退出恢复。
20. 修复复核问题后跑最后一轮全量：`cd src-tauri && cargo test`、`pnpm typecheck`、`pnpm test:run`（含 `cargo clippy` 视既有习惯）。
21. 更新 `.trellis/spec/`（若产生新约定）→ 按仓库提交纪律提交（`feat(proxy): …`），不 push 除非用户要求。

## 回滚点

- 第 1 步后（数据层）即可回滚：删列/降 `SCHEMA_VERSION` 不影响旧版本读取。
- 第 4 步（接管注入）是行为分界线：此前代理可独立测试运行，客户端完全不受影响；若要收缩范围，可只保留第 1–3 步 + 前端状态页，把接管注入单独拆任务。
