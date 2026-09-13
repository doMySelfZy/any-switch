# 站点列表显示剩余额度摘要

## Goal

左侧「上游站点」列表条目直接显示额度摘要，一眼看到每个站点还剩多少额度；数据层完全复用现有额度探测（不改 Rust、不改 invoke 契约、不改右侧详情面板 `SiteQuotaRow` 的行为）。

## 背景（现状事实）

- 额度已按站点缓存在 `siteStore`：`quotaBySite`（成功结果）、`quotaAttemptBySite`（含失败尝试）、`probeQuota(siteId, opts)` 自带 **in-flight 去重** 与 **5 分钟 TTL 缓存**（`QUOTA_TTL_MS = 5 * 60 * 1000`，`isQuotaCacheFresh`）。
- 目前只有右侧详情触发加载（选中站点时），列表条目（`src/components/sites/SiteListItem.tsx`）不显示任何额度信息。
- 列表项右侧「…」按钮（`.site-more-btn`）是 `opacity: 0 → hover 时 1`，**布局占位始终保留**，因此 URL 行右端的摘要文字不会被它覆盖，无需淡出让位逻辑。
- 额度两类形态（见 `SiteQuota` 类型与 `SiteQuotaRow`）：
  - **余额型**：`status === "available"`、`windows` 为空、`remainingUsd != null`（new-api 站点）；只有剩余金额，**没有可靠百分比**（new-api 总额常是无限额度哨兵值，代码已注释不画进度条）。
  - **窗口型**：`windows` 非空（OpenCode：rolling 5 小时 / weekly / monthly，含 `usagePercent` 与 `resetAt`）。
  - 其余：`unsupported` / `error` / `unlimited` / 加载中——列表一律**安静不显示**（不写「额度未知」，避免一列表灰字）。

## Requirements

1. **摘要展示**（`SiteListItem` URL 行右端、右对齐、`text-xs`）：
   - 余额型：复用 `sites.quotaRemaining`（"剩余 {{amount}}"），金额用 `formatQuotaAmountParts` 格式化。
   - 窗口型：显示 rolling 窗口（`kind === "rolling"`；无 rolling 取 `windows[0]`）的用量百分比，如 `83%`；窗口名不放列表，放 Tooltip。
   - 非 available / unlimited / unsupported：不渲染任何摘要。
2. **变色告警**（用 `theme.useToken()` token 色，禁止硬编码 hex）：
   - 窗口型按用量百分比：`>= 90%` 用 `colorError`，`>= 80%` 用 `colorWarning`，其余中性（`colorTextTertiary`）。
   - 余额型保持中性灰（无可靠总额基准，不做变色）。
   - 刷新失败不标红——摘要只基于最近一次成功数据（`quotaBySite`），失败态留给右侧详情。
3. **Tooltip**（悬停摘要时）：
   - 余额型：`剩余 {{amount}}` + 更新时间（复用 `sites.quotaUpdatedJustNow` / `sites.quotaMinutesAgo` / `sites.quotaUpdated` 的相对时间逻辑）。
   - 窗口型：每个窗口一行或一段：`{窗口名} {percent}%`（窗口名复用 `sites.quotaWindowRolling/Weekly/Monthly`），末尾附更新时间。
   - 窗口名/格式化逻辑如需共享，抽到 `src/lib/quotaProbe.ts` 或共享组件，不要在 `SiteQuotaRow` 和 `SiteListItem` 间复制粘贴两份。
4. **批量预取**：
   - `SitesPage` 挂载且站点列表非空时，对全部站点逐个调用 `probeQuota(site.id)`（非 force）。`probeQuota` 已有的 TTL + in-flight 去重就是节流，不要再造一层缓存。
   - 预取失败静默（`probeQuota` 本身不弹 message，符合预期；不要在预取路径加错误提示）。
   - 选中站点后的刷新仍走现有逻辑，不因预取重复请求（in-flight map 保证）。
5. **i18n**：新文案 zh-CN + en-US 同步补 key；插值必须双花括号 `{{name}}`（此仓库踩过单花括号的坑）；技术标识（%、$ 除外）不硬编码中文。
6. **交互约束**：摘要文字不拦截点击——点击条目仍是选中站点；摘要放在现有 `onSelect` 按钮内部即可，不要另加可点击元素（Tooltip 允许）。

## Acceptance Criteria

- [ ] 列表中 new-api 站点（如 SHUAI/JustWoker/AgentRouter）显示「剩余 $xx」；OpenCode 显示 5 小时窗口百分比；不支持的站点（如 aihub）与加载前不显示任何摘要。
- [ ] 窗口用量 ≥80% / ≥90% 时摘要变橙/变红。
- [ ] 悬停摘要出现 Tooltip 详情（余额：金额+更新时间；窗口：各窗口百分比+更新时间）。
- [ ] 进入站点页时列表站点被预取（测试可用 mock invoke 断言 `probe_site_quota` 按站点 id 调用），同一次挂载内不重复请求。
- [ ] 右侧详情面板行为与改动前一致（`SiteQuotaRow` 未被破坏）。
- [ ] `pnpm typecheck` 通过；`pnpm test:run` 通过（预存在失败仅限 `generateUpdaterManifest.test.ts` 与 `validateUpdaterSigningSecret.test.ts` 这两个本机 SyntaxError）。
- [ ] 新增测试覆盖：余额型摘要、窗口型摘要+变色、不支持站点不渲染。

## 边界

- 不改 `src-tauri/`（Rust 侧零改动）。
- 不改右侧 `SiteQuotaRow` 的现有渲染/刷新行为；如抽共享逻辑，其对外 Props 与渲染不变。
- 并行会话警告：工作区有其他任务（MCP）的未提交改动（`src/stores/index.ts`、`src/App.tsx`、i18n locale 文件等）。只改本任务所需文件；i18n locale 文件只追加自己的 key。

## Notes

- 轻量任务，PRD-only。
