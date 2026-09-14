# 桌面悬浮窗功能

## 目标
实现一个桌面悬浮窗，显示所有站点的余额信息，无需打开主窗口即可查看。

## 状态
- Status: in_progress
- Priority: P2
- Created: 2026-09-13

## 功能需求

### 核心功能
1. 桌面悬浮窗口
   - 无边框窗口
   - 始终置顶
   - 可拖动
   - 小尺寸 (280x400px)

2. 交互方式
   - 点击展开/收起
   - 收起状态：小圆形按钮 + 图标
   - 展开状态：显示所有站点余额列表

3. 数据显示
   - 站点名称
   - 剩余额度
   - 最后更新时间

4. 设置选项
   - 开启/关闭悬浮窗（默认开启）
   - 自动刷新间隔（默认 5 分钟）
   - 重置窗口位置

### 技术方案

#### 后端 (Rust)
- 新增模块：`src-tauri/src/floating_window.rs`
- 新增命令文件：`src-tauri/src/commands/floating.rs`
- 窗口管理、位置保存、定时刷新

#### 前端 (Vue)
- 新增页面：`src/pages/FloatingWindow.vue`
- UI 组件：收起/展开状态视图
- 样式：毛玻璃效果、深色主题

#### 数据结构
```rust
pub struct FloatingWindowSettings {
    pub enabled: bool,
    pub auto_refresh_seconds: u64,
    pub position_x: i32,
    pub position_y: i32,
    pub collapsed: bool,
}

pub struct SiteQuotaSummary {
    pub site_id: i64,
    pub site_name: String,
    pub quota: String,
    pub is_unlimited: bool,
}
```

## 实现步骤

### Phase 1: 基础框架 ✅
- [x] 创建任务目录
- [x] 设计数据结构
- [x] 创建 Rust 模块文件
- [ ] 添加 Tauri 命令
- [ ] 配置 tauri.conf.json

### Phase 2: 数据获取
- [ ] 实现获取所有站点余额接口
- [ ] 实现定时刷新机制
- [ ] 缓存优化

### Phase 3: UI 实现
- [ ] 创建悬浮窗页面
- [ ] 实现收起/展开状态
- [ ] 添加拖动功能
- [ ] 样式美化

### Phase 4: 设置集成
- [ ] 扩展 AppSettings
- [ ] 添加设置页面选项
- [ ] 窗口位置保存/恢复

### Phase 5: 测试优化
- [ ] Windows 平台测试
- [ ] macOS 平台测试（可选）
- [ ] 性能优化
- [ ] 边界情况处理

## 技术细节

### Windows 特性
- `set_always_on_top(true)` - 始终置顶
- `set_decorations(false)` - 无边框
- `set_skip_taskbar(true)` - 不显示在任务栏

### 性能考虑
- 使用独立 WebView
- 后台定时器只在启用时运行
- 缓存避免频繁 API 请求

## 问题与决策

### Q1: 悬浮窗默认位置？
- A: 右下角（距离屏幕边缘 20px）

### Q2: 是否支持点击站点切换？
- A: Phase 1 只支持查看，后续可考虑

### Q3: 显示哪些信息？
- A: Phase 1 只显示站点名和余额，后续可加到期时间

## 文件清单

### 新增文件
- `src-tauri/src/floating_window.rs` - ✅ 已创建
- `src-tauri/src/commands/floating.rs` - ✅ 已创建
- `src/pages/FloatingWindow.vue` - 待创建
- `src/components/floating/QuotaList.vue` - 待创建
- `src/components/floating/CollapsedView.vue` - 待创建

### 修改文件
- `src-tauri/src/lib.rs` - 注册命令
- `src-tauri/src/commands/mod.rs` - 导出模块
- `src-tauri/tauri.conf.json` - 添加悬浮窗配置
- `src/router/index.ts` - 添加路由
- Settings 相关文件 - 扩展配置

## 参考资料
- Tauri 2 多窗口文档
- Tauri Window API
