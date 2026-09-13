# 实现清单

前置：读 `prd.md`（需求与验收标准）与 `design.md`（关键决策，尤其决策 1/2 的乘数 vs 除数语义、决策 3 的插入点、决策 4 的 used/total 处理）。背景表里的实测数据是事实，可直接用于构造测试 JSON，但测试必须用假值、不得提交真实密钥。

## 步骤

### 1. Rust：账户余额换算参数（需求 A）

文件 `src-tauri/src/quota_probe/mod.rs`

1. 新增 `account_balance_scale(status: &QuotaStatus) -> Option<(f64 /*divisor*/, String /*unit*/)>`，与 `raw_quota_scale` 并列（后者是乘数语义，勿混用）。doc comment 明确乘数/除数差异。
2. 改 `user_self_amounts` 签名，接收 `Option<&QuotaStatus>`（或直接接 divisor+unit），用自报 `quota_per_unit` 作除数；缺失/≤0 回退 `NEWAPI_QUOTA_PER_UNIT`（500000）；unit 缺失 → `"USD"`。
3. 改 `fetch_user_self_quota`：与 `/api/user/self` **并发**请求 `/api/status`（`tokio::join!`），解析为 `Option<QuotaStatus>`（失败即 None），传给金额换算；`SiteQuota.unit` 用换算出的单位。
4. 保持 `request_user_self` / `user_self_success` / `test_newapi_access`（测试按钮）共用同一条换算链路——两处必须一致（PRD 要求 4）。
5. 不改 `sanitize_absurd_amounts` 与 `quota_values_are_consistent` 行为；换新倍率后一致性校验仍须通过。

### 2. Rust：Sub2API 探测（需求 B）

同文件：

6. `QuotaSource` 增 `Sub2Api`（`src-tauri/src/domain/mod.rs`，serde snake_case）。
7. 新增 `sub2api_usage_url(origin) -> String`（`{origin}/v1/usage`）+ `Expected::Sub2ApiUsage` 分类分支（沿用 `classify_status`；404/405/501/HTML/非法 JSON → 静默不支持）。
8. 新增 `parse_sub2api_usage(&Value) -> Option<Sub2ApiUsage>`：按 design 决策 4 的规则（`isValid:false`/error 结构 → None；`remaining` 回退 `balance`；`unit` 缺省 USD；`used`/`total` 一律 None；unrestricted+remaining<0 → unlimited）。
9. 在 `probe_site_quota` 的**标准链最终判定为 Unsupported/Fallback 之后**尝试 `/v1/usage`（design 决策 3），命中则返回 `Available`；未命中维持原 Unsupported。确保标准链命中的 new-api 站点**不产生** `/v1/usage` 请求。
10. 候选 origin 复用 `public_api_bases`（先去 `/v1` 再回退根），不要另造。

### 3. 前端（需求 B.10 / C.11）

11. `grep -rn "opencode_go\|user_self\|credit_grants" src/`：若 TS 有对 `QuotaSource` 的穷举（switch/Record/联合类型），同步加 `sub2_api`；否则不改。
12. 确认余额型展示（`SiteListItem` 摘要、`SiteQuotaRow` 详情）对 Sub2API 结果自动生效，无需新增形态。
13. 仅当需要新增用户可见文案时才动 i18n（zh-CN + en-US 同步、`{{}}` 双花括号）；**只追加自己的 key**，不重排他人行。

### 4. 测试（design 测试策略 1–6）

14. Rust 单测覆盖：`account_balance_scale` 各分支与除数语义、自报倍率换算（`quota_per_unit != 500000`）、回退 500000 基准值、`parse_sub2api_usage` 全分支、`/v1/usage` 降级、new-api 站点不额外请求 `/v1/usage`。
15. 前端 `pnpm typecheck`；跑 `pnpm test:run`。

## 验证命令

```
cd src-tauri && cargo test
pnpm typecheck
pnpm test:run
```

cargo 需先前置 MSVC 路径（见 AGENTS/memory）：
`export PATH="/c/Program Files (x86)/Microsoft Visual Studio/2022/BuildTools/VC/Tools/MSVC/14.44.35207/bin/Hostx64/x64:$HOME/.cargo/bin:$PATH"`

已知本机预存在失败（不算失败）：`src/lib/generateUpdaterManifest.test.ts`、`src/lib/validateUpdaterSigningSecret.test.ts`（SyntaxError）。

## 完成标准

- PRD 的 11 条 Acceptance Criteria 全部满足，并在汇报时逐条给出证据（测试名或命令输出）。
- 未越界：不实现订阅窗口/rate_limits，不加设置项，不改 UI 视觉，不动 billing/token 既有优先级。
- 不提交 git（由主会话统一处理）。
