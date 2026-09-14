# 悬浮窗功能实施计划

## 总览

本文档详细说明悬浮窗功能的实施步骤，按照从后端到前端、从基础到完善的顺序进行。

## Phase 1: 数据层与基础架构

### Step 1.1: 数据库迁移
**文件**: `src-tauri/src/db/migrate.rs`

```rust
// 添加新的迁移版本
pub async fn migrate_floating_window(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        r#"
        ALTER TABLE app_settings ADD COLUMN floating_window_enabled INTEGER DEFAULT 1;
        ALTER TABLE app_settings ADD COLUMN floating_window_refresh_seconds INTEGER DEFAULT 300;
        ALTER TABLE app_settings ADD COLUMN floating_window_x INTEGER DEFAULT -1;
        ALTER TABLE app_settings ADD COLUMN floating_window_y INTEGER DEFAULT -1;
        ALTER TABLE app_settings ADD COLUMN floating_window_collapsed INTEGER DEFAULT 0;
        "#
    )
    .execute(pool)
    .await?;
    
    Ok(())
}
```

**验证**:
- [ ] 迁移脚本运行成功
- [ ] 数据库表结构正确更新
- [ ] 默认值正确设置

### Step 1.2: 数据模型定义
**文件**: `src-tauri/src/domain/mod.rs`

添加两个新结构体：
1. `FloatingWindowSettings` - 悬浮窗配置
2. `SiteQuotaSummary` - 站点余额汇总

**验证**:
- [ ] 结构体编译通过
- [ ] Serialize/Deserialize 正常
- [ ] Default 实现正确

### Step 1.3: Repository 扩展
**文件**: `src-tauri/src/repo/settings.rs`

添加方法：
```rust
impl SettingsRepo {
    pub async fn get_floating_window_settings(&self) -> Result<FloatingWindowSettings>
    pub async fn update_floating_window_enabled(&self, enabled: bool) -> Result<()>
    pub async fn update_floating_window_position(&self, x: i32, y: i32) -> Result<()>
    pub async fn update_floating_window_settings(&self, settings: FloatingWindowSettings) -> Result<()>
}
```

**验证**:
- [ ] 所有方法编译通过
- [ ] CRUD 操作正确
- [ ] 错误处理完善

## Phase 2: 窗口管理

### Step 2.1: 窗口管理模块
**文件**: `src-tauri/src/floating_window.rs` (新建)

实现 `FloatingWindowManager` 结构体及其方法：
- `new()` - 创建管理器
- `create()` - 创建悬浮窗
- `show()` / `hide()` - 显示/隐藏
- `set_position()` - 设置位置
- `get_default_position()` - 获取默认位置

**验证**:
- [ ] 窗口可以成功创建
- [ ] 窗口属性正确（置顶、无边框、透明等）
- [ ] 显示/隐藏功能正常
- [ ] 位置设置生效

### Step 2.2: Tauri Commands
**文件**: `src-tauri/src/commands/floating.rs` (新建)

实现命令：
1. `get_all_sites_quota` - 获取余额汇总
2. `toggle_floating_window` - 切换显示/隐藏
3. `save_floating_window_position` - 保存位置
4. `update_floating_window_settings` - 更新设置

**验证**:
- [ ] 所有命令编译通过
- [ ] 命令可以从前端调用
- [ ] 返回值格式正确

### Step 2.3: 命令注册
**文件**: `src-tauri/src/commands/mod.rs` 和 `src-tauri/src/lib.rs`

将新命令添加到 Tauri builder。

**验证**:
- [ ] 命令成功注册
- [ ] 应用启动无错误

## Phase 3: 余额数据接口

### Step 3.1: 余额解析逻辑
**文件**: `src-tauri/src/commands/floating.rs`

实现 `parse_balance()` 辅助函数：
- 解析站点余额字段
- 判断是否无限额度
- 判断是否低余额（< $1 或 ¥10）
- 格式化显示文本

**验证**:
- [ ] 不同格式余额解析正确
- [ ] 货币符号处理正确
- [ ] 边界情况处理妥当

### Step 3.2: 余额汇总接口
完善 `get_all_sites_quota` 命令：
- 查询所有站点
- 解析每个站点余额
- 排序（低余额优先）
- 返回汇总列表

**验证**:
- [ ] 返回所有站点数据
- [ ] 数据格式正确
- [ ] 性能可接受（< 100ms）

## Phase 4: 前端基础

### Step 4.1: 路由配置
**文件**: `src/router/index.ts`

添加悬浮窗路由：`/floating` → `FloatingWindow.vue`

**验证**:
- [ ] 路由注册成功
- [ ] 可以通过 URL 访问

### Step 4.2: 悬浮窗主页面
**文件**: `src/pages/FloatingWindow.vue` (新建)

基础结构：
```vue
<template>
  <div class="floating-window" :class="{ collapsed: isCollapsed }">
    <CollapsedView v-if="isCollapsed" @expand="expand" />
    <ExpandedView v-else @collapse="collapse" :sites="sites" />
  </div>
</template>
```

**验证**:
- [ ] 页面可以正常渲染
- [ ] 状态切换正常
- [ ] 窗口可拖动

### Step 4.3: 收起状态组件
**文件**: `src/components/floating/CollapsedView.vue` (新建)

圆形按钮 + 图标，点击展开。

**验证**:
- [ ] 圆形按钮显示正常
- [ ] 图标居中
- [ ] 点击事件触发

### Step 4.4: 展开状态组件
**文件**: `src/components/floating/ExpandedView.vue` (新建)

包含：
- Header（标题 + 刷新 + 收起按钮）
- QuotaList（余额列表）
- Footer（最后更新时间）

**验证**:
- [ ] 布局正确
- [ ] 所有子组件显示
- [ ] 交互正常

### Step 4.5: 余额列表组件
**文件**: `src/components/floating/QuotaList.vue` (新建)

显示所有站点余额，低余额高亮。

**验证**:
- [ ] 列表渲染正常
- [ ] 滚动流畅
- [ ] 高亮生效

## Phase 5: 数据刷新

### Step 5.1: 数据获取
**文件**: `src/pages/FloatingWindow.vue`

添加方法：
```typescript
const fetchQuota = async () => {
  loading.value = true
  try {
    const data = await invoke('get_all_sites_quota')
    sites.value = data
    lastUpdate.value = Date.now()
  } finally {
    loading.value = false
  }
}
```

**验证**:
- [ ] 数据获取成功
- [ ] 加载状态显示
- [ ] 错误处理完善

### Step 5.2: 手动刷新
添加刷新按钮，点击调用 `fetchQuota()`。

**验证**:
- [ ] 刷新按钮可点击
- [ ] 数据正确更新
- [ ] 加载动画显示

### Step 5.3: 后台定时刷新
**文件**: `src-tauri/src/lib.rs`

启动后台任务：
```rust
start_floating_window_refresh_task(app.handle());
```

**验证**:
- [ ] 定时任务启动
- [ ] 按配置间隔刷新
- [ ] 悬浮窗禁用时任务暂停

### Step 5.4: 事件监听
**文件**: `src/pages/FloatingWindow.vue`

监听后端的 `quota-updated` 事件：
```typescript
listen('quota-updated', () => {
  fetchQuota()
})
```

**验证**:
- [ ] 事件监听正常
- [ ] 后端触发时前端更新
- [ ] 无内存泄漏

## Phase 6: 窗口交互

### Step 6.1: 窗口拖动
**文件**: `src/pages/FloatingWindow.vue`

使用 Tauri 的 `appWindow.startDragging()`。

**验证**:
- [ ] 窗口可拖动
- [ ] 不会移出屏幕
- [ ] 拖动流畅

### Step 6.2: 位置保存
拖动结束时保存位置：
```typescript
const savePosition = async () => {
  const position = await appWindow.outerPosition()
  await invoke('save_floating_window_position', {
    x: position.x,
    y: position.y
  })
}
```

**验证**:
- [ ] 位置正确保存
- [ ] 重启后恢复
- [ ] 多显示器场景正常

### Step 6.3: 启动时恢复
**文件**: `src-tauri/src/lib.rs`

应用启动时检查配置，自动显示悬浮窗：
```rust
if settings.enabled {
    let manager = FloatingWindowManager::new(app.handle());
    manager.show()?;
    if settings.position_x >= 0 && settings.position_y >= 0 {
        manager.set_position(settings.position_x, settings.position_y)?;
    }
}
```

**验证**:
- [ ] 启动时自动显示
- [ ] 位置正确恢复
- [ ] 禁用时不显示

## Phase 7: 设置集成

### Step 7.1: 设置页面 UI
**文件**: `src/pages/Settings.vue` 或相关设置组件

添加「悬浮窗」配置区域：
```vue
<div class="setting-section">
  <h3>悬浮窗</h3>
  <el-switch v-model="floatingEnabled" @change="updateEnabled" />
  <el-input-number v-model="refreshInterval" :min="1" :max="60" />
  <el-button @click="resetPosition">重置位置</el-button>
</div>
```

**验证**:
- [ ] UI 显示正常
- [ ] 控件交互正常
- [ ] 布局协调

### Step 7.2: 设置变更处理
实现方法：
- `updateEnabled()` - 更新启用状态
- `updateInterval()` - 更新刷新间隔
- `resetPosition()` - 重置窗口位置

**验证**:
- [ ] 设置立即生效
- [ ] 持久化到数据库
- [ ] 悬浮窗响应变更

## Phase 8: 视觉打磨

### Step 8.1: 毛玻璃效果
**文件**: CSS 样式

应用 `backdrop-filter: blur(20px)` 和半透明背景。

**验证**:
- [ ] 毛玻璃效果显示
- [ ] 透明度适中
- [ ] 性能可接受

### Step 8.2: 深色主题
适配项目现有的深色主题变量。

**验证**:
- [ ] 深色模式显示正常
- [ ] 颜色对比度足够
- [ ] 与主窗口风格一致

### Step 8.3: 动画效果
添加过渡动画：
- 收起/展开动画
- 余额变化提示
- 刷新加载动画

**验证**:
- [ ] 动画流畅
- [ ] 时长合适（200-300ms）
- [ ] 不影响性能

### Step 8.4: 低余额高亮
余额低于阈值时特殊样式。

**验证**:
- [ ] 高亮颜色醒目
- [ ] 不过于刺眼
- [ ] 阈值合理

## Phase 9: 测试与优化

### Step 9.1: 功能测试
- [ ] 所有功能正常工作
- [ ] 边界情况处理
- [ ] 错误恢复机制

### Step 9.2: 性能测试
- [ ] 内存占用 < 50MB
- [ ] CPU 使用率正常
- [ ] 无内存泄漏

### Step 9.3: 用户体验测试
- [ ] 交互流畅
- [ ] 视觉美观
- [ ] 信息清晰

### Step 9.4: 多场景测试
- [ ] Windows 10/11
- [ ] 不同 DPI 设置
- [ ] 多显示器
- [ ] macOS（如果支持）

## 关键文件清单

### 新建文件
- `src-tauri/src/floating_window.rs`
- `src-tauri/src/commands/floating.rs`
- `src/pages/FloatingWindow.vue`
- `src/components/floating/CollapsedView.vue`
- `src/components/floating/ExpandedView.vue`
- `src/components/floating/QuotaList.vue`

### 修改文件
- `src-tauri/src/db/migrate.rs` - 添加迁移
- `src-tauri/src/domain/mod.rs` - 添加数据模型
- `src-tauri/src/repo/settings.rs` - 扩展 Repository
- `src-tauri/src/commands/mod.rs` - 导出新命令
- `src-tauri/src/lib.rs` - 注册命令、启动后台任务
- `src/router/index.ts` - 添加路由
- `src/pages/Settings.vue` - 添加设置项

## 预估工时

| Phase | 描述 | 预估时间 |
|-------|------|---------|
| Phase 1 | 数据层与基础架构 | 2-3 小时 |
| Phase 2 | 窗口管理 | 2-3 小时 |
| Phase 3 | 余额数据接口 | 1-2 小时 |
| Phase 4 | 前端基础 | 2-3 小时 |
| Phase 5 | 数据刷新 | 2 小时 |
| Phase 6 | 窗口交互 | 2 小时 |
| Phase 7 | 设置集成 | 1-2 小时 |
| Phase 8 | 视觉打磨 | 2-3 小时 |
| Phase 9 | 测试与优化 | 2-3 小时 |
| **总计** | | **16-23 小时** |

## 里程碑

1. **MVP（Phase 1-4）** - 基础功能可用，能显示余额列表
2. **Beta（Phase 5-7）** - 完整功能，包括刷新和设置
3. **RC（Phase 8-9）** - 视觉完善，通过测试
