# OpenCode Go 渠道用量窗口支持（5小时/周/月）

## Goal
识别 base URL 为 `opencode.ai/zen/go` 的站点，改走官方 `GET https://opencode.ai/zen/go/v1/usage`（Bearer API Key）查询 **5 小时 / 每周 / 每月** 三个用量窗口，并在站点额度区展示进度与重置倒计时。

## 调研结论（已实测/查证）
- 官方端点：`GET https://opencode.ai/zen/go/v1/usage`，认证为 `Authorization: Bearer <Go API Key>`。
- 实测：带无效 key 返回 `401 {"type":"error","error":{"type":"AuthError","message":"Unauthorized"}}`（端点存在）；**不需要 User-Agent 伪装**；`/zen/v1/usage` 不存在（404 SPA）；`/zen/go/v1/models` 公开 200。
- 官方文档（opencode.ai/docs/go）：Go 计划限额为 5 小时 $12 / 每周 $30 / 每月 $60，按美元计；每模型另有月度上限。
- 渠道判断：`base_url` 的 host 为 `opencode.ai` 且路径含 `/zen/go`（用户 ZCode 里该渠道 baseURL 正是 `https://opencode.ai/zen/go/v1`）。
- 社区参考实现（opencode-quota 等）使用同一端点与 API key 认证，字段为三个窗口的用量百分比与重置时间。

## Requirements

### 后端
1. 渠道识别 `is_opencode_go_base(base_url) -> bool`：host == `opencode.ai` 且 path 含 `/zen/go`。
2. `probe_quota` 命中该渠道时**直接走 OpenCode Go 探测**（不走 new-api/billing 标准链）：
   - URL：`https://opencode.ai/zen/go/v1/usage`（由站点 base 推导 origin + 固定路径；仅允许 https 且 host 为 opencode.ai）。
   - 200 → 解析三窗口（5 小时/周/月）百分比与重置时间；401 → `Unauthorized`；404/其他 → `Unsupported`/`Error`。
3. 新增 `QuotaSource::OpencodeGo`（serde `opencode_go`）。
4. `SiteQuota` 新增 `windows: Vec<QuotaWindow>`（`#[serde(default)]`），`QuotaWindow { kind: rolling|weekly|monthly, usage_percent: Option<f64>, reset_at: Option<i64>, limit_usd: Option<f64> }`。
5. 解析器对字段命名容错（`usagePercent`/`usage_percent`/`percent`；`resetInSec`/`resets_in_seconds`/`resetAt`…），并把相对重置秒数折算为绝对时间戳（fetched_at + secs）以便 UI 稳定展示。

### 前端
6. `SiteQuotaRow` 当 `quota.windows` 非空时渲染三行窗口视图：`5 小时 / 本周 / 本月` + 进度条 + 重置倒计时 +（若有）上限金额。
7. i18n（中英）：窗口名、`{{time}} 后重置`、`上限 {{amount}}`。

## Acceptance Criteria
- [ ] base URL 含 `opencode.ai/zen/go` 的站点，点刷新能显示三个窗口的用量百分比与重置时间（用真实 key 验证）。
- [ ] 其它站点行为不变（走原标准探测链）；`https://opencode.ai/zen/v1` 等非 Go 路径不误判为该渠道。
- [ ] 无 key / key 无效 → 显示未授权，不误导为“不支持”。
- [ ] Rust 单测覆盖：渠道识别、三窗口解析（含多种字段命名）、401 分支；`cargo test` 全绿。
- [ ] `pnpm typecheck` + `pnpm test:run` 全绿（前端窗口视图不影响既有额度展示用例）。

## Notes
- 只查询用量（只读），不消耗模型额度。
- 请求仅发往固定官方域名 `opencode.ai`（https），符合出站 URL 约束。
- 用户记忆的“User-Agent 接口”经实测为 API Key 认证，已纠正。
