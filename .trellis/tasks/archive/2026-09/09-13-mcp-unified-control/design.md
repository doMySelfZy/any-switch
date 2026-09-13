# 技术设计

## 关键决策
MCP 记录存入 SQLite，秘密字段使用现有 Crypto；同步沿用数据库 + master.key 的现有 WebDAV 备份协议。目标文件是派生输出，应用时按目标分别备份并原子合并。

四个目标采用原生配置位置：Claude 使用用户级 `~/.claude.json` 的 `mcpServers`，Codex 使用 `~/.codex/config.toml` 的 `[mcp_servers.*]`，Pi 使用其 agent 目录下 `mcp.json` 的 `mcpServers`，Prime 使用 `settings.json` 的 `mcpServers`。路径均支持现有 override 解析。

统一领域模型支持 stdio、sse、http，并保留扩展字段。XiaoBaiSwitch 以稳定名称前缀 `xiaobai_` 管理写入条目，数据库记录保存目标集合和应用快照；冲突名称不覆盖非托管条目，应用结果逐目标返回。

## 数据流
前端 MCP 管理页 → typed Tauri commands → repo/migration + Crypto → MCP service/target adapters → atomic backup/write → target result；WebDAV 只同步数据库备份，因此恢复后 MCP 定义可重新应用。

## 兼容与安全
保留用户未管理的 MCP 和未知 JSON/TOML 字段；删除仅删除带有 XiaoBaiSwitch ownership 标记且快照匹配的条目。所有外部路径通过现有路径解析和原子写工具处理。UI 对密钥只显示脱敏值，并提示应用后客户端配置可能含明文。

## 分阶段
1. Schema/domain/repository/commands and shared typed payloads.
2. Per-target paths and merge/revert adapters with unit tests.
3. Apply orchestration, backups, target status and restore/reconcile behavior.
4. Frontend MCP center, forms, target selection, i18n.
5. Full Rust/frontend validation and cross-layer review.
