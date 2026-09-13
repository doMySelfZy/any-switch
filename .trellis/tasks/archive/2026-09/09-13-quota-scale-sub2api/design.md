# 设计：额度探测读自报倍率 + Sub2API 支持

## 关键决策与理由

### 决策 1：`/api/status` 只在需要时取一次，且与账户余额请求并发

现状：账户余额链路（`fetch_user_self_quota`）完全不请求 `/api/status`，直接除 500000。

方案：在 `fetch_user_self_quota` 内**并发**发起两个请求——`/api/user/self`（拿余额）与 `/api/status`（拿换算参数），status 用 `Option` 语义（失败即 `None`，回退默认 500000/USD）。

理由：
- 不并发的话会串行翻倍延迟（用户已经感知到额度刷新有网络耗时）。
- status 是「锦上添花」：它失败绝不能拖垮余额显示，所以必须是 `Option` 而非 `Result` 传播。
- 只取一次、只在此链路取，避免影响 billing/token/status 既有优先级（PRD 边界要求）。

注意：`fetch_round` 里已有一次 `/api/status` 请求（用于 token 额度换算），但账户余额链路（`fetch_user_self_quota`）与 `fetch_round` 是**两条独立路径**，且账户余额命中时直接 return、不进 `fetch_round`。因此这里新增的 status 请求不会与 `fetch_round` 的重复（互斥路径）；不要为了"复用"而把两条链路耦合。

### 决策 2：复用 `parse_quota_status` + `raw_quota_scale`，不新造解析

`quota_probe/mod.rs` 已有完整实现：
- `parse_quota_status(&Value) -> Option<QuotaStatus>`（:375）解析 `quota_per_unit`、`display_type`、`usd_exchange_rate` 等。
- `raw_quota_scale(&QuotaStatus) -> Option<(f64, String)>`（:539）把 display_type 映射为 `(scale, unit)`：
  - `Tokens` → `(1.0, "TOKENS")`
  - `Usd` → `(1.0 / quota_per_unit, "USD")`
  - `Cny` → `(usd_exchange_rate / quota_per_unit, "CNY")`
  - `Custom` → `(custom_currency_exchange_rate / quota_per_unit, symbol)`
  - `Raw` → `None`

注意语义差异：`raw_quota_scale` 返回的是**乘数**（用于 token 额度），而账户余额需要的是**除数**（`quota / quota_per_unit`）。不能直接把 `raw_quota_scale` 的 scale 乘上去，否则语义错乱。实现方式二选一：
- (a) 只用 `parse_quota_status`，自己写 `quota_per_unit` 除数与单位判定（简单直接）；
- (b) 抽一个 `account_balance_scale(&QuotaStatus) -> Option<(f64 /*divisor*/, String /*unit*/)>` 与 `raw_quota_scale` 并列，语义对称。

推荐 (b)：两个函数并列、命名对称（`token_quota_scale` / `account_balance_scale`），并有独立单测锚定语义，避免后人误用。同时为每个函数补一句 doc comment 说明「乘数 vs 除数」。

兜底规则（PRD 要求 2）：
- `quota_per_unit` 缺失或 ≤0 → 除 500000；
- `display_type` 缺失 / `Raw` → 单位 `"USD"`（保持 AgentRouter 现状：它无 `quota_display_type`，当前显示 `$`）；
- `Cny` → 单位 `"CNY"`（SHUAI 的 `￥`）；
- `Custom` → symbol 或 `"USD"` 兜底。

### 决策 3：Sub2API 作为标准链之后的独立尝试

插入点：在 `probe_site_quota` 里，账户余额（`newapi_creds` 分支）之后、标准 `fetch_round` 之前或之后？

- **必须在账户余额之后**（PRD 要求 9：UserSelf 优先）。
- 放在 `fetch_round` **之前**更省：Sub2API 站点（AiHub）的 billing/status 全 404，`fetch_round` 会白跑 4+ 个请求。
- 但放之前会让每个 new-api 站点多一次 `/v1/usage` 请求（404），增加一次往返。

权衡：new-api 站点常见、Sub2API 较少。放 `fetch_round` 之后可避免给所有 new-api 站点增加请求；只有标准链判定 `Unsupported`（即 `outcome_needs_fallback` 触发、最终会走到 Unsupported 分支）时才尝试 `/v1/usage`。

推荐：**在标准链（含 origin 回退的 combine）最终结果为 Unsupported 时，再尝试 `/v1/usage`**。这样：
- new-api 站点（标准链命中）完全不受影响，零额外请求；
- AiHub（标准链全 404 → Unsupported）才多打一次，命中即用。
- 现有 `sanitize_absurd_amounts` 仍在最后统一应用。

注意 `outcome_needs_fallback` / `combine_round_outcomes` 的既有逻辑不要改动，只在最终 `RoundOutcome::Fallback` 或 quiet Unsupported 的出口前插入尝试。

### 决策 4：Sub2API 响应解析

```rust
pub struct Sub2ApiUsage {
    pub remaining: Option<f64>,
    pub used: Option<f64>,
    pub total: Option<f64>,
    pub unit: String,
    pub unlimited: bool,
}
pub fn parse_sub2api_usage(value: &Value) -> Option<Sub2ApiUsage>
```

规则（实测依据 PRD 背景表）：
- `isValid == false` → `None`（不得当余额）。
- 上游失败结构（`response_indicates_failure` 已能识别 new-api 风格 `success:false`；Sub2API 的 error 结构需一并识别，如顶层 `error` 对象）→ `None`。
- `remaining` 取 `remaining`，缺失回退 `balance`（实测同值）；都为非数 → `None`。
- `unit` 取 `unit` 字符串（缺省 `"USD"`）。
- 钱包模式下 `used`/`total` 无法可靠得知（实测响应无 used/total 语义字段）：`used = None`、`total = None`。**不要**把 `balance` 当 total 填进去——那会让前端算出 0% 的进度条或误导性的"已用 0"。
- `mode == "unrestricted"` 时的 `remaining == -1`（cc-switch 社区脚本的判据）→ `unlimited = true`，其余金额清空。若实测该站点不出现此值，仍按防御性实现并加单测。

### 决策 5：QuotaSource 枚举与前端

`QuotaSource` 增 `Sub2Api`（serde `snake_case` → `"sub2_api"`）。检查前端是否有对该枚举的穷举 switch；若有需同步（`grep -rn "opencode_go\|user_self\|credit_grants" src/`）。前端展示层应无需改动（走既有余额型分支），但需确认没有 TS 联合类型穷举导致类型错误。

## 测试策略

Rust 单测（`src-tauri/src/quota_probe/mod.rs` 的 `mod tests`，沿用现有 `json!` 风格）：
1. `account_balance_scale`：USD/CNY/Raw/缺失/quota_per_unit≤0 各分支 + 除数语义（不是乘数）。
2. `user_self_amounts` 带自报参数的换算：`quota=1000000, quota_per_unit=100000` → 10.0（验证不是 500000）。
3. 回退：无 status 时 `quota=135193229` → `270.386458`（锁定 500000 兜底不回归）。
4. `parse_sub2api_usage`：实测样本（`balance`/`remaining`/`unit`/`isValid:true`）；`isValid:false` → None；仅 `balance`（无 remaining）→ 可用；缺 unit → USD；`remaining:-1` 且 unrestricted → unlimited；NEWAPI 风格 `success:false` → None。
5. 降级：`/v1/usage` 404 / HTML / 非法 JSON → `Unsupported`（复用现有 `classify_status` 语义，补 `Expected::Sub2ApiUsage` 分支）。
6. 集成：模拟 new-api 站点标准链命中时**不**发生 `/v1/usage` 请求（保护"零额外请求"承诺）。

前端：确认无类型穷举破坏（`pnpm typecheck`），必要时补 `siteStore`/展示层冒烟测试。

## 真机验证（实施后由主会话做，需真实密钥）

1. 重启 App，读窗口 UIA 文本：AiHub 应显示 `剩余 $31.xx`（Sub2API 通道），其余四站显示不变。
2. 对 AgentRouter 单独核对：`/api/user/self` 的 `quota/500000` 与界面数值一致（允许余额实时下降的差值）。
3. 站点编辑「测试」按钮数值与列表一致。

## 风险

- **status 请求带来的额外往返**：账户余额链路增加一次并发请求；若站点对并发敏感可能有偶发失败——status 是 Option，失败不影响余额。
- **AiHub 的 UA 依赖**：现网拒绝无 UA / python-urllib；App 的 `XiaoBaiSwitch/0.1.2` 实测可用。若未来该站升级拦截，会安静降级为「不支持」（不报错），用户可感知为额度消失。
- **Sub2API 响应字段变体**：社区文档提到 `subscription`/`quota`/`rate_limits` 三种模式；本期只覆盖钱包余额模式。若某站点走 `quota` 模式（`{quota:{used,limit,remaining,unit}}`），解析会返回 None 而降级——这是已知取舍，PRD 已列为不做项。
