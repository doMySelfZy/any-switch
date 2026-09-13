# MCP 更新功能实现

## 任务目标
为本地安装的 MCP 服务器添加版本检测和更新功能：
1. 检测已安装 MCP 是否有新版本
2. 在界面上显示更新提示
3. 支持单个更新和批量更新

## 实现架构

### 1. 数据库层 (src-tauri/src/repo/mcp_version.rs)
- `mcp_versions` 表存储版本信息
- 字段：id, current_version, latest_version, checked_at
- 提供 CRUD 操作

### 2. 领域层 (src-tauri/src/domain/mod.rs)
- `McpUpdateInfo`: 更新信息结构
- `McpBatchUpdateResult`: 批量更新结果

### 3. 适配器层
- **mcp_version.rs**: 版本检测逻辑
  - 从 package.json 读取本地版本
  - 调用 npm/npx 获取最新版本
  - 支持 npm 和 npx 两种包管理器
  
- **mcp_update.rs**: 更新执行逻辑
  - 执行 npm install -g 更新包
  - 更新数据库版本信息

### 4. 命令层 (src-tauri/src/commands/mcp_update.rs)
暴露给前端的 Tauri 命令：
- `check_mcp_updates`: 检查所有 MCP 更新
- `update_mcp_server`: 更新单个 MCP
- `batch_update_mcp_servers`: 批量更新多个 MCP

### 5. 前端层
- **mcpUpdateStore.ts**: Pinia store 管理更新状态
  - `updateStatuses`: 更新状态映射
  - `checking/updating`: 加载状态
  - `hasAnyUpdate()`: 是否有更新
  - `updateCount()`: 更新数量
  - `updatableServers()`: 可更新列表

- **McpPage.tsx**: 界面集成
  - 顶部显示更新数量和操作按钮
  - 每个 MCP 卡片显示更新徽章
  - 单个更新按钮
  - 批量更新所有

## 版本检测逻辑

### NPM 包
1. 从 `command` 或 `args[0]` 提取包名
2. 本地版本：解析 package.json
   ```bash
   npm list -g <package> --json
   ```
3. 最新版本：查询 npm registry
   ```bash
   npm view <package> version
   ```

### NPX 包
1. 从 `args` 提取包名（通常是第一个参数）
2. 本地版本：通过 npx 执行包的 --version
3. 最新版本：npm view 查询

## 更新流程

### 单个更新
1. 获取 MCP 配置
2. 提取包名
3. 执行 `npm install -g <package>@latest`
4. 更新数据库版本信息
5. 返回新版本号

### 批量更新
1. 遍历所有 ID
2. 逐个执行更新
3. 收集成功和失败结果
4. 返回汇总信息

## UI 设计

### 顶部状态栏
```
[状态: X 个可更新] [检查更新] [全部更新]
```

### MCP 卡片
```
┌─────────────────────────────┐
│ MCP Name         [有更新]    │
│ v1.0.0 → v1.1.0            │
│                 [更新] [×]  │
└─────────────────────────────┘
```

## 测试要点

1. ✅ 数据库迁移成功创建 mcp_versions 表
2. ✅ 后端编译通过（MCP 相关模块）
3. ⏳ 前端类型检查
4. ⏳ 版本检测功能测试
5. ⏳ 单个更新测试
6. ⏳ 批量更新测试
7. ⏳ 错误处理测试

## 已完成
- ✅ 数据库表和 repo 层
- ✅ 领域模型定义
- ✅ 版本检测适配器
- ✅ 更新执行适配器
- ✅ Tauri 命令层
- ✅ 前端 Store
- ✅ UI 界面集成
- ✅ 后端编译修复

## 待完成
- ⏳ 前端 TypeScript 类型修复
- ⏳ 实际运行测试
- ⏳ 错误处理优化
