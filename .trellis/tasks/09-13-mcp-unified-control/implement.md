# 实现计划

1. 盘点现有 domain、migration、repo、command、apply result 类型，新增 MCP schema 与加密字段迁移。
2. 实现 MCP repository 与 Tauri CRUD/list/apply commands，建立稳定 ownership 与目标绑定快照。
3. 扩展 paths 和四个 adapters：Claude JSON、Codex TOML、Pi JSON、Prime settings JSON；加入合并、清理、备份、原子写和单测。
4. 接入 apply orchestration，支持按目标选择、目标级结果、禁用/删除清理及恢复后可重应用。
5. 增加前端 store/types、MCP 管理界面、表单、目标选择、应用状态和 zh-CN/en-US 文案。
6. 验证数据库备份/WebDAV round-trip 不改既有协议；运行 cargo test、pnpm typecheck、pnpm test:run。
7. 由复核代理检查兼容红线、路径安全、敏感信息处理和四目标行为一致性。
