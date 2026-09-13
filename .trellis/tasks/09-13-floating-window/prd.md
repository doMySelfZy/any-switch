# floating-window

## Goal

实现桌面悬浮窗显示所有站点余额，支持收起/展开、自动刷新、位置保存

## Requirements

### 功能需求

1. **桌面悬浮窗**
   - 在 Windows 桌面上创建一个始终置顶的悬浮窗口
   - macOS 支持（Tauri 2 跨平台）
   - 无边框、可拖动、透明背景
   - 不显示在任务栏
   - 支持两种状态：收起（小按钮）和展开（完整列表）

2. **余额显示**
   - 显示所有已配置站点的余额信息
   - 每个站点显示：站点名称、剩余额度
   - 支持不同货币单位显示（$、¥、Unlimited 等）
   - 显示最后更新时间
   - 余额为 0 或即将耗尽时高亮提示

3. **交互方式**
   - 点击收起状态按钮 → 展开完整列表
   - 点击展开状态的收起按钮 → 收起为小按钮
   - 拖动窗口可自由移动位置
   - 位置自动保存，重启后恢复
   - 可选：点击站点项跳转到主窗口

4. **自动刷新**
   - 后台定时自动刷新所有站点余额
   - 刷新间隔可配置（默认 5 分钟）
   - 手动刷新按钮
   - 刷新时显示加载状态

5. **设置集成**
   - 在设置页面添加「悬浮窗」配置项
   - 开关：启用/禁用悬浮窗（默认启用）
   - 配置：自动刷新间隔（分钟）
   - 操作：重置窗口位置到默认位置

### 技术需求

1. **后端（Rust）**
   - 新增 `FloatingWindowSettings` 配置结构
   - 新增 `floating_window.rs` 模块管理窗口生命周期
   - 新增 Tauri Commands：
     - `get_all_sites_quota` - 获取所有站点余额汇总
     - `toggle_floating_window` - 切换悬浮窗显示/隐藏
     - `save_floating_window_position` - 保存窗口位置
   - 后台定时刷新任务（tokio::time::interval）
   - 窗口配置：always_on_top、frameless、skip_taskbar

2. **前端（Vue）**
   - 新增 `FloatingWindow.vue` 页面
   - 新增 `QuotaList.vue` 组件（展开状态）
   - 新增 `CollapsedView.vue` 组件（收起状态）
   - 毛玻璃效果、深色主题适配
   - 动画效果（收起/展开、余额变化）

3. **数据库**
   - 扩展 `AppSettings` 表添加悬浮窗配置字段
   - 存储窗口位置（x, y）
   - 存储悬浮窗状态（启用、收起、刷新间隔）

### 约束条件

1. 悬浮窗使用独立的 WebView，不影响主窗口性能
2. 后台定时器只在悬浮窗启用时运行
3. 使用缓存避免频繁请求 API
4. 窗口默认位置：右下角（距离屏幕边缘 20px）
5. 收起状态尺寸：60x60px 圆形按钮
6. 展开状态尺寸：280x400px 列表窗口

## Acceptance Criteria

### 基础功能
- [ ] 悬浮窗可以在桌面上创建并显示
- [ ] 悬浮窗始终置顶，不显示在任务栏
- [ ] 可以正常拖动窗口，位置重启后恢复
- [ ] 显示所有站点的余额信息（名称 + 余额）
- [ ] 收起/展开状态切换正常

### 数据同步
- [ ] 余额数据从数据库正确读取
- [ ] 自动刷新功能正常工作（默认 5 分钟）
- [ ] 手动刷新按钮可用
- [ ] 刷新时显示加载状态
- [ ] 最后更新时间显示正确

### 设置集成
- [ ] 设置页面可以开启/关闭悬浮窗
- [ ] 设置页面可以配置刷新间隔
- [ ] 设置页面可以重置窗口位置
- [ ] 设置变更立即生效

### 视觉效果
- [ ] 毛玻璃背景效果正常
- [ ] 深色主题适配良好
- [ ] 收起/展开动画流畅
- [ ] 余额为 0 或低值时有视觉提示
- [ ] 整体视觉风格与主窗口一致

### 性能与稳定性
- [ ] 悬浮窗不影响主窗口性能
- [ ] 后台刷新不阻塞 UI
- [ ] 关闭悬浮窗后定时器正确停止
- [ ] 内存占用合理（< 50MB）
- [ ] 无明显内存泄漏

## Notes

- Keep `prd.md` focused on requirements, constraints, and acceptance criteria.
- Lightweight tasks can remain PRD-only.
- For complex tasks, add `design.md` for technical design and `implement.md` for execution planning before `task.py start`.
