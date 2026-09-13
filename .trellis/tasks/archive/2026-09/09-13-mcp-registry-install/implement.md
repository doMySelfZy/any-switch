# 实现计划

1. **后端解析层**（可与前端并行）
   - 新增 `src-tauri/src/mcp_registry/`（或等价模块）：仓库 HTTP 客户端、响应反序列化类型、`parse_registry_entry` 纯函数。
   - 覆盖：packages 型（npx/uvx + runtimeArguments + packageArguments）、remotes 型（streamable-http/sse）、必填 env/header 识别、名称归一化、缺字段与异常响应。
2. **后端命令**
   - `search_mcp_registry(query, cursor)`：搜索并返回归一化候选列表。
   - `resolve_mcp_registry_entry(name)`：按名称取详情并产出安装草稿（若搜索已返回足够信息，可由前端直接使用，命令作为刷新/兜底）。
   - 在 `lib.rs` 注册；错误归一化（网络失败、404、超大响应）。
3. **前端搜索与回填**
   - `mcpStore` 增加搜索状态与动作；`types/mcp.ts` 增加草稿/候选类型。
   - `McpPage` 增加搜索入口（主按钮），结果列表可选中；浏览器 mock 补对应命令。
4. **表单分层**
   - 简单层：名称、类型、命令/地址（人话展示）、必填密钥逐项输入。
   - 高级折叠：env/headers/config 原始 JSON。
   - 从仓库安装时必填项未填则阻止保存并指出缺哪一项。
5. **弱化手动添加**：主按钮改成「从官方仓库添加」，手动添加降为次级按钮并附说明。
6. **文案**：zh-CN / en-US 全量补齐（注意插值必须双花括号）。
7. **验证**
   - `cd src-tauri && cargo test`、`pnpm typecheck`、`pnpm test:run`。
   - 真机：搜一个 remote 型和一个 package 型条目，安装后确认写入目标客户端结构正确。
8. **复核**：派 check 代理核对红黑线（既有写入路径未改、托管命名空间未改、同步未受影响）与测试非空断言。
