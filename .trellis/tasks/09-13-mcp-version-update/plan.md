# MCP 版本检查与更新功能

## 需求
为本地安装的 MCP Agent 添加版本检查和更新功能：
- 检测本地 MCP 是否有新版本
- 在 UI 上显示更新提示
- 提供单个更新和一键全部更新功能

## 实现方案

### 1. 数据模型扩展

#### 后端 (Rust)
在 `src-tauri/src/domain/mcp.rs` 中扩展：
- `McpServer` 和 `McpServerSummary` 添加字段：
  - `current_version: Option<String>` - 当前安装版本
  - `latest_version: Option<String>` - Registry 最新版本
  - `update_available: bool` - 是否有更新
  - `last_update_check_at: Option<i64>` - 上次检查时间

#### 数据库 Schema
在 `mcp_servers` 表中添加列：
```sql
ALTER TABLE mcp_servers ADD COLUMN current_version TEXT;
ALTER TABLE mcp_servers ADD COLUMN latest_version TEXT;
ALTER TABLE mcp_servers ADD COLUMN last_update_check_at INTEGER;
```

#### 前端 (TypeScript)
在 `src/types/mcp.ts` 中同步更新类型定义

### 2. 版本检查逻辑

#### 版本获取
- **当前版本**：
  - npm 包：通过 `npm list -g --json <package>` 获取
  - 远程服务：从 config 中的 URL 或存储的元数据获取
- **最新版本**：从 Registry API 查询

#### 版本比较
- 使用 `semver` crate 进行语义化版本比较
- 处理非标准版本号（如 `latest`、git hash）

### 3. 后端 API

#### 新增 Tauri Commands
```rust
// 检查所有 MCP 更新
#[tauri::command]
async fn check_mcp_updates() -> Result<Vec<McpUpdateInfo>, String>

// 检查单个 MCP 更新
#[tauri::command]
async fn check_mcp_update(id: String) -> Result<McpUpdateInfo, String>

// 更新单个 MCP
#[tauri::command]
async fn update_mcp_server(id: String) -> Result<McpSaveResult, String>

// 批量更新所有有更新的 MCP
#[tauri::command]
async fn update_all_mcp_servers() -> Result<McpBatchUpdateResult, String>
```

#### 实现文件
- `src-tauri/src/adapters/mcp_version.rs` - 版本检查与更新逻辑
- `src-tauri/src/commands/mcp.rs` - 添加新命令

### 4. 前端 UI

#### McpStore 扩展
在 `src/stores/mcpStore.ts` 中添加：
```typescript
checkUpdates: () => Promise<void>
checkUpdate: (id: string) => Promise<void>
updateServer: (id: string) => Promise<McpSaveResult>
updateAllServers: () => Promise<void>
```

#### UI 改动
在 `src/pages/McpPage.tsx` 中：
1. **列表视图**：
   - 每个条目显示版本信息和"有更新"标签
   - 操作菜单添加"更新"选项
   
2. **顶部工具栏**：
   - 添加"检查更新"按钮
   - 添加"全部更新"按钮（仅在有更新时可用）
   - 显示有多少个 MCP 可更新

3. **状态显示**：
   - 正在检查更新时显示加载状态
   - 更新完成后显示成功/失败消息

### 5. 实现细节

#### npm 包更新流程
1. 检测包名（从 config.command 解析，如 `npx -y @modelcontextprotocol/server-filesystem`）
2. 通过 npm registry API 获取最新版本
3. 更新时重新运行安装命令（npm install -g）
4. 重新应用到已配置的 targets

#### 远程 HTTP/SSE 服务
- 可能没有版本概念，或需要从特定 endpoint 查询
- 提供"检查连接"功能而非版本更新

#### 版本缓存
- 避免频繁查询 Registry
- 缓存时间：1小时
- 提供手动刷新选项

### 6. 错误处理
- 网络错误：提示无法连接到 Registry
- 版本解析失败：标记为"未知版本"
- 更新失败：回滚到之前配置，显示错误详情

## 实现顺序
1. ✅ 创建任务目录和规划文档
2. 数据库 schema 更新
3. 后端：版本检查逻辑 (mcp_version.rs)
4. 后端：Tauri commands
5. 前端：类型定义更新
6. 前端：Store 方法
7. 前端：UI 组件
8. 测试与调试
