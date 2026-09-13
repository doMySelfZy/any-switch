# MCP 更新功能实现总结

## 功能概述

为 xiaobai-switch 应用添加了完整的 MCP Server 版本检查和更新功能。用户可以：
- 检查所有本地安装的 MCP Server 是否有新版本
- 查看哪些 Server 有可用更新
- 单个更新或批量更新 MCP Server
- 一键更新所有可更新的 Server

## 架构设计

### 后端 (Rust)

#### 1. 数据模型 (`src-tauri/src/domain/mcp_update.rs`)
- `McpUpdateStatus`: 表示单个 MCP Server 的更新状态
  - `id`: Server ID
  - `name`: Server 名称
  - `current_version`: 当前安装的版本
  - `latest_version`: 仓库中的最新版本
  - `has_update`: 是否有可用更新
  - `last_check_at`: 上次检查时间

#### 2. 仓库层 (`src-tauri/src/repo/mcp_update.rs`)
- `McpUpdateRepo`: 管理版本检查和更新操作
- 主要方法：
  - `check_updates()`: 检查所有 Server 的更新状态
  - `update_server()`: 更新单个 Server
  - `batch_update()`: 批量更新多个 Server

#### 3. 适配器层 (`src-tauri/src/adapters/mcp_update.rs`)
- 集成 MCP Registry API
- 实现版本比较逻辑（semver）
- 处理 npm/npx 包的更新操作

#### 4. 命令层 (`src-tauri/src/commands/mcp_update.rs`)
暴露给前端的 Tauri 命令：
- `check_mcp_updates()`: 检查所有更新
- `update_mcp_server(id)`: 更新单个 Server
- `batch_update_mcp_servers(ids)`: 批量更新

### 前端 (React + TypeScript)

#### 1. Store (`src/stores/mcpUpdateStore.ts`)
使用 Zustand 管理更新状态：
- 状态：
  - `updateStatuses`: 所有 Server 的更新状态
  - `checking`: 是否正在检查更新
  - `updating`: 正在更新的 Server ID 映射
  - `lastCheckTime`: 上次检查时间

- Getters:
  - `hasAnyUpdate()`: 是否有任何可用更新
  - `updateCount()`: 可更新 Server 数量
  - `updatableServers()`: 所有可更新的 Server
  - `getUpdateStatus(id)`: 获取特定 Server 的更新状态
  - `isUpdating(id)`: 检查 Server 是否正在更新

- Actions:
  - `checkUpdates()`: 触发检查更新
  - `updateServer(id)`: 更新单个 Server
  - `batchUpdate(ids)`: 批量更新
  - `updateAll()`: 更新所有可更新的 Server
  - `clearUpdateStatus(id)`: 清除更新状态
  - `reset()`: 重置所有状态

#### 2. UI 组件 (`src/pages/McpPage.tsx`)

**顶部操作栏：**
- 标题旁显示更新提示徽章（橙色）
- "检查更新" 按钮：手动触发检查
- "一键更新全部" 按钮：更新所有可更新的 Server（仅在有更新时显示）

**Server 卡片：**
- 版本信息显示：`当前版本 → 最新版本`
- 更新状态徽章（橙色 "有更新" 标签）
- "更新" 按钮：更新单个 Server
- 更新过程中显示加载状态

#### 3. 国际化 (`src/locales/*`)
添加了以下翻译键：
- `mcp.checkUpdates`: "检查更新"
- `mcp.updateAll`: "一键更新全部"
- `mcp.updatesAvailable`: "有 {count} 个更新"
- `mcp.updateAvailable`: "有更新"
- `mcp.update`: "更新"
- `mcp.updating`: "更新中..."
- `mcp.currentVersion`: "当前版本"
- `mcp.latestVersion`: "最新版本"
- `mcp.updateSuccess`: "更新成功"
- `mcp.updateFailed`: "更新失败"
- `mcp.checkUpdatesFailed`: "检查更新失败"
- `mcp.batchUpdatePartialSuccess`: "部分更新成功：{success} 成功，{failed} 失败"

## 实现细节

### 版本检查流程
1. 前端调用 `checkUpdates()`
2. 后端查询数据库中所有已安装的 MCP Server
3. 对每个 Server，从 MCP Registry 获取最新版本信息
4. 使用 semver 比较当前版本和最新版本
5. 返回更新状态列表给前端
6. 前端更新 UI 显示

### 单个更新流程
1. 用户点击 Server 卡片上的"更新"按钮
2. 前端调用 `updateServer(id)`
3. 后端执行 npm 更新命令（根据包类型）
4. 更新成功后，刷新数据库中的版本信息
5. 前端更新本地状态，移除更新徽章

### 批量更新流程
1. 用户点击"一键更新全部"按钮
2. 前端收集所有可更新的 Server ID
3. 调用 `batchUpdate(ids)`
4. 后端并发执行多个更新操作
5. 返回成功和失败的列表
6. 前端显示批量更新结果通知

## 用户体验优化

1. **非阻塞检查**：检查更新时显示加载状态，但不阻止其他操作
2. **实时反馈**：更新过程中显示加载动画和禁用按钮
3. **错误处理**：更新失败时显示清晰的错误消息
4. **批量操作**：支持一键更新所有 Server，提高效率
5. **视觉提示**：使用橙色徽章突出显示有更新的 Server
6. **版本对比**：清晰显示 "当前版本 → 最新版本"

## 测试要点

### 功能测试
- [ ] 点击"检查更新"按钮，能正确检测到有更新的 Server
- [ ] 有更新时，标题栏显示更新数量徽章
- [ ] Server 卡片上显示"有更新"标签和版本对比
- [ ] 点击单个 Server 的"更新"按钮，能成功更新
- [ ] 更新过程中显示加载状态
- [ ] 更新成功后，"有更新"标签消失
- [ ] 点击"一键更新全部"，能批量更新所有 Server
- [ ] 批量更新显示成功/失败统计
- [ ] 更新失败时显示错误提示

### 边界情况
- [ ] 没有安装任何 MCP Server 时的表现
- [ ] 所有 Server 都是最新版本时的表现
- [ ] 网络错误时的错误处理
- [ ] Registry API 返回异常数据时的处理
- [ ] 并发更新时的状态管理

### 性能测试
- [ ] 检查大量 Server 时的响应时间
- [ ] 批量更新的并发处理
- [ ] 重复检查更新时的缓存策略

## 技术亮点

1. **类型安全**：全程使用 TypeScript 和 Rust 的强类型系统
2. **状态管理**：使用 Zustand 简洁高效地管理更新状态
3. **错误处理**：完善的错误捕获和用户提示
4. **并发控制**：批量更新时合理控制并发数
5. **用户体验**：流畅的交互和清晰的状态反馈

## 未来改进方向

1. **自动检查**：应用启动时自动检查更新
2. **更新通知**：后台定期检查，有更新时通知用户
3. **版本历史**：显示 Server 的版本更新历史和 Changelog
4. **回滚功能**：支持回退到之前的版本
5. **更新日志**：记录所有更新操作的历史
6. **选择性更新**：支持更新到指定版本而非仅最新版
