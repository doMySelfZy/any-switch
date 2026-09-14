# 实现计划

按「编译驱动」推进：先加枚举与字段，让编译器把所有需要补的 match 分支列出来，逐个填。

## 阶段 1：后端骨架
- [ ] `domain::TargetKind` 加 `ZCode`（serde 名 `zcode`）
- [ ] `AppSettings` 加 `zcode_home_override`；`paths` 加 `default_zcode_home` / `resolve_zcode_home`
- [ ] `sites` 加列 `zcode_api_type`（`ensure_incremental_schema` 里 `ensure_column`，不动 SCHEMA_VERSION）
- [ ] 编译驱动：补齐 `backup.rs`（备份 id 前缀、文件解析）、`commands/*`、`tray*` 等所有 `TargetKind` match
- [ ] 单测：路径优先级、backup id 往返

## 阶段 2：ZCode 站点适配器（`adapters/zcode.rs`）
- [ ] 纯函数构造两份内容：
      `build_provider_entry(site, model_ids, api_type, api_key)` →
      `build_provider_config(provider_id, name, base_url, api_key, api_type, models)`
- [ ] `apply_site(...)`：备份两个文件 → 合并（只动 `xiaobai_`）→ 原子替换
- [ ] `revert_site(...)`：移除 `xiaobai_` provider（两处）
- [ ] 单测：新增/更新/移除、`providerOrder` 占位保留、用户 provider 不动、非法形状报错且不写
- [ ] 变异验证关键断言

## 阶段 3：MCP
- [ ] `adapters/mcp.rs` 加 `apply_to_zcode`（根路径 `mcp.servers`）
- [ ] `mcp_scan.rs` 加 ZCode 读取（provider + mcp）
- [ ] 单测：合并保留用户 MCP、清理托管条目、纳管读取

## 阶段 4：前端
- [ ] `types/domain.ts`：`TargetKind` 加 `zcode`；`AppSettings` 加 `zcodeHomeOverride`
- [ ] `ApplySidebar` / `uiStore` / 目标标签：加第 5 项
- [ ] `ZCodeApplyPanel.tsx`：协议选择、默认模型、写入模型目录开关
- [ ] `siteStore` / `browserMock`：zcode 的 apply/revert/status 契约
- [ ] i18n（zh-CN + en-US 成对）

## 阶段 5：验证
- [ ] `cargo test`、`pnpm typecheck`、`pnpm test:run`
- [ ] 真机：把某个站点应用到 ZCode，确认两份文件都被写入且 ZCode 能列出该 provider
- [ ] 确认既有四端行为不变（回归）
- [ ] 打包安装，请用户确认

## 风险与对策
- **改动面大**：`TargetKind` 遍布 backup/adapters/commands。用编译器穷尽检查驱动，不靠人工搜索。
- **写坏用户 ZCode 配置**：所有写入前备份 + 形状校验 + 只动 `xiaobai_`；先在本机用副本验证再上真机。
