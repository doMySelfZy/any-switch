# 额度探测：读站点自报倍率 + 支持 Sub2API /v1/usage

## Goal

补两处额度探测缺口，让账户余额换算不再依赖硬编码倍率，并让 Sub2API 风格的中转站（用户实际持有的 AiHub）能显示余额。

## 背景（2026-09-13 实测事实，勿再重复验证）

用户机器上的真实站点（用真实密钥实测）：

| 站点 | base_url | `/api/status` 自报 | `/v1/usage` | 当前 App 显示 |
|---|---|---|---|---|
| SHUAI API | api.shuaiapi.com | quota_per_unit=500000, **quota_display_type=CNY**, usd_exchange_rate=1 | 404 | ￥27.57 ✓ |
| JustWoker | api.justwoker.icu | quota_per_unit=500000, **quota_display_type=USD**, usd_exchange_rate=7.3 | 404 | $1,223.89 ✓ |
| AgentRouter | ps.air-outer.com | quota_per_unit=500000, **无 quota_display_type**, display_in_currency=true, usd_exchange_rate=7.12 | 404 | $269.43 ✓ |
| **AiHub** | aihub.top | 404（非 new-api） | **HTTP 200** | **「不支持自动获取额度」（漏支持）** |
| OpenCode | opencode.ai/zen/go/v1 | 404 | 200（三窗口，已支持） | 4% ✓ |

AiHub 的 `/v1/usage` 响应（实测，字段名以现网为准，勿臆测）：
`mode: "unrestricted"`, `isValid: true`, `planName: "钱包余额"`, `balance: 31.31783589`, `remaining: 31.31783589`, `unit: "USD"`, 另有 `usage{daily,total,...}`、`daily_usage[]`、`model_stats[]`。

**AiHub 的标准链行为（2026-09-13 实测，解释它为何现在是「不支持」）**：
- `/api/status` → 404，`/api/usage/token` → 404；
- `/dashboard/billing/credit_grants` 与 `/dashboard/billing/subscription` → **HTTP 200 但返回 65028 字节的 HTML**（SPA 兜底页，非 JSON）。
- 代码的 `looks_like_html`（`quota_probe/mod.rs:786`）把 2xx+HTML 正确归类为 `Hit::Unsupported`，因此标准链最终仍判定为 Unsupported —— **这正是设计决策 3「标准链 Unsupported 后才试 `/v1/usage`」能够触发的前提**，已被现网行为验证（AiHub 当前就显示「此站点不支持自动获取额度」）。
- 注意：该 HTML 为 65028 字节，逼近 `MAX_BODY_BYTES` = 65536 上限。若站点未来把 index.html 撑过 64KB 会截断，但 `looks_like_html` 只读开头即可识别，不影响判类。

UA 实测：该站拒绝无 UA 与 `Python-urllib` 签名（403），接受 `XiaoBaiSwitch/0.1.2`（App 现有 UA，`http_client::default_user_agent()`）与 `curl`。响应体约 3KB，远低于 `MAX_BODY_BYTES`=64KB。

## Requirements

### A. 账户余额读站点自报换算参数（`/api/user/self` 链路）

1. `user_self_amounts` 目前硬编码 `NEWAPI_QUOTA_PER_UNIT = 500_000.0` 且 `unit: Some("USD")`（`quota_probe/mod.rs:1235`、`:1330` 附近）。改为优先使用站点 `/api/status` 自报的 `quota_per_unit`，缺失或非正数时回退 500000。
2. 货币单位同样按自报 `quota_display_type` 决定（USD → `"USD"`；CNY → `"CNY"`；缺失 → `"USD"` 兜底，保持 AgentRouter 现状），复用已有 `parse_quota_status` / `raw_quota_scale` 的语义，**不要新造一套解析**。
3. `/api/status` 的获取不得使单次额度探测的请求数无节制增长：沿用探测链已有的「尽力而为」语义——拿不到 status 就用默认值，不能因为 status 请求失败而让账户余额整体失败。
4. 站点编辑里的「测试」按钮与主界面额度行必须继续共用同一条换算链路（现有注释明确要求两处一致），不得出现「测试通过但主界面数值不同」的分叉。
5. `sanitize_absurd_amounts`（1000 万阈值）与 `quota_values_are_consistent` 的既有行为保持；换新倍率后仍需保证一致性校验通过。

### B. 新增 Sub2API `/v1/usage` 探测

6. 新增一个探测来源（`QuotaSource` 增加枚举值，如 `Sub2Api`），在标准链之后、判定「不支持」之前尝试：`GET {origin}/v1/usage`，`Authorization: Bearer <API Key>`。
7. 响应解析以 `remaining`/`balance` 为准（实测二者同值），取 `unit`（缺省 USD），映射为 `remaining_usd` + `unit`；`planName` 等描述字段不在本期展示范围。`isValid: false` 或上游 error 结构时**不得**当作可用余额。
8. 端点不可用例（404/405/501/HTML/非法 JSON）必须安静降级为「不支持」，与现有 `quiet(Unsupported)` 语义一致——不能让 new-api 站点因为多打这一枪而变成错误态。认证失败（401/403 且非 UA 拦截）按现有 `Unauthorized` 语义处理。
9. 该探测不得影响既有来源优先级：TokenUsage / UserSelf（账户余额）仍优先于 Sub2API 余额；OpenCode Go 专用链路不受影响。
10. 前端无需新增额度形态：Sub2API 钱包余额复用现有「余额型」展示（剩余金额），列表摘要与右侧详情都应正确显示（`unit` 走 `formatQuotaAmountParts` 既有 CNY/USD/其他 分支）。

### C. 通用约束

11. **零硬编码文案 / 零硬编码颜色**：新增错误或状态文案走 i18n（zh-CN + en-US 同步，插值必须 `{{value}}` 双花括号）；Rust 侧不产生用户可见中文。
12. 不改变既有 `SiteQuota` JSON 契约的字段名与含义（前端 `quotaProbe.ts`、`SiteQuotaRow`、`SiteListItem` 均依赖）；如需新增字段必须可选且有 serde 默认。
13. 安全：任何日志/错误回显不得包含原始 API key 或访问令牌，错误路径复用现有 `sanitize_error` / `truncate_message` 脱敏。
14. 并行会话纪律：工作区可能同时存在其他任务的未提交改动（如 `*mcp*`）。只改本任务所需文件，i18n locale 文件只追加自己的 key，不整理/重排他人行。

## Acceptance Criteria

- [ ] AgentRouter 显示金额与 `/api/user/self` 换算值一致（实测基准：`quota/500000`），且符号仍为 `$`（无 `quota_display_type` 时 USD 兜底）。
- [ ] JustWoker 显示 `$`、SHUAI 显示 `￥`（各自自报货币），金额与自报倍率换算一致。
- [ ] 构造 `quota_per_unit != 500000` 的响应时（单元测试足够），换算使用自报值而非硬编码 500000。
- [ ] `/api/status` 请求失败或字段缺失时，账户余额仍能正常显示（回退 500000/USD），不报错。
- [ ] AiHub 这类 Sub2API 站点显示「剩余 $31.32」（或探测时的实时值），符号与单位正确，且右侧详情、列表摘要一致。
- [ ] new-api 站点（SHUAI/JustWoker/AgentRouter）行为不因新增 `/v1/usage` 请求而改变——仍显示各自账户余额，无错误态、无额外延迟感（可接受多一次请求的开销）。
- [ ] OpenCode 三窗口展示不受影响。
- [ ] `/v1/usage` 返回 HTML、404、非法 JSON 时安静降级为「不支持」，不出现错误文案。
- [ ] 站点编辑「测试」按钮与主界面额度行数值一致（共用链路）。
- [ ] Rust 单元测试覆盖：自报倍率换算、缺失回退、Sub2API 响应解析（含 `isValid:false`、`balance`/`remaining` 取一、`unit` 缺省）、非 Sub2API 站点降级。`cd src-tauri && cargo test` 通过。
- [ ] `pnpm typecheck` 与 `pnpm test:run` 通过（仅允许本机预存在失败：`generateUpdaterManifest.test.ts`、`validateUpdaterSigningSecret.test.ts` 的 SyntaxError）。

## 边界（不做）

- 不实现 Sub2API 的订阅窗口（`subscription` 的日/周/月三段）与 `rate_limits` 展示——本期只取钱包余额；用户站点实测为 `mode: "unrestricted"` 钱包模式。（可作为后续任务）
- 不新增设置项、不加站点级开关；自动探测，失败安静降级。
- 不改 `/api/user/self` 之外的 new-api 链路（billing/token/status 的现有优先级不动）。
- 不做 UI 视觉改动。

## Notes

- 复杂度中等偏上（Rust 后端 + 契约 + 真实站点行为），需 `design.md`；实现清单见 implement.md。
- 实测站点数据仅用于理解格式，**不要提交任何真实密钥或令牌**；测试一律用假值构造 JSON。
