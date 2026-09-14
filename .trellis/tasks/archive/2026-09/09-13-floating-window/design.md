# 悬浮窗技术设计

## 架构概览

```
┌─────────────────────────────────────────┐
│         Main Window (主窗口)             │
│  - 设置页面控制悬浮窗开关                 │
│  - 管理悬浮窗配置                        │
└─────────────────────────────────────────┘
                    │
                    │ Tauri Commands
                    ↓
┌─────────────────────────────────────────┐
│         AppState (Rust 后端)             │
│  - floating_window.rs (窗口管理)         │
│  - commands/floating.rs (命令接口)       │
│  - 后台定时刷新任务                      │
└─────────────────────────────────────────┘
                    │
                    │ Window API
                    ↓
┌─────────────────────────────────────────┐
│      Floating Window (悬浮窗)            │
│  - FloatingWindow.vue                   │
│  - 独立 WebView                         │
│  - 显示余额列表                          │
└─────────────────────────────────────────┘
```

## 数据结构

### Rust 数据模型

```rust
// domain/mod.rs - 悬浮窗配置
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct FloatingWindowSettings {
    pub enabled: bool,              // 是否启用
    pub auto_refresh_seconds: u64,  // 自动刷新间隔（秒）
    pub position_x: i32,            // X 坐标
    pub position_y: i32,            // Y 坐标
    pub collapsed: bool,            // 是否收起
}

impl Default for FloatingWindowSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            auto_refresh_seconds: 300, // 5 分钟
            position_x: -1,            // -1 表示使用默认位置
            position_y: -1,
            collapsed: false,
        }
    }
}

// domain/mod.rs - 站点余额汇总
#[derive(Debug, Clone, Serialize)]
pub struct SiteQuotaSummary {
    pub id: i64,
    pub name: String,
    pub balance: String,           // 格式化后的余额（如 "$10.50"）
    pub raw_balance: Option<f64>,  // 原始数值
    pub is_unlimited: bool,
    pub is_low: bool,              // 余额低于阈值
    pub last_updated: Option<i64>, // Unix 时间戳
}
```

### 数据库 Schema

```sql
-- 扩展 app_settings 表
ALTER TABLE app_settings ADD COLUMN floating_window_enabled INTEGER DEFAULT 1;
ALTER TABLE app_settings ADD COLUMN floating_window_refresh_seconds INTEGER DEFAULT 300;
ALTER TABLE app_settings ADD COLUMN floating_window_x INTEGER DEFAULT -1;
ALTER TABLE app_settings ADD COLUMN floating_window_y INTEGER DEFAULT -1;
ALTER TABLE app_settings ADD COLUMN floating_window_collapsed INTEGER DEFAULT 0;
```

## 模块设计

### 1. 窗口管理模块 (`floating_window.rs`)

```rust
use tauri::{AppHandle, Manager, Window, WindowBuilder, WindowUrl};

pub struct FloatingWindowManager {
    handle: AppHandle,
}

impl FloatingWindowManager {
    pub fn new(handle: AppHandle) -> Self {
        Self { handle }
    }

    /// 创建悬浮窗
    pub fn create(&self) -> Result<Window, Error> {
        let window = WindowBuilder::new(
            &self.handle,
            "floating",
            WindowUrl::App("/floating".into())
        )
        .title("XiaoBaiSwitch Plus - 悬浮窗")
        .inner_size(280.0, 400.0)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .transparent(true)
        .build()?;

        Ok(window)
    }

    /// 显示悬浮窗
    pub fn show(&self) -> Result<(), Error> {
        if let Some(window) = self.handle.get_window("floating") {
            window.show()?;
        } else {
            self.create()?.show()?;
        }
        Ok(())
    }

    /// 隐藏悬浮窗
    pub fn hide(&self) -> Result<(), Error> {
        if let Some(window) = self.handle.get_window("floating") {
            window.hide()?;
        }
        Ok(())
    }

    /// 设置窗口位置
    pub fn set_position(&self, x: i32, y: i32) -> Result<(), Error> {
        if let Some(window) = self.handle.get_window("floating") {
            window.set_position(tauri::LogicalPosition::new(x, y))?;
        }
        Ok(())
    }

    /// 获取默认位置（右下角）
    pub fn get_default_position(&self) -> Result<(i32, i32), Error> {
        // 获取主显示器尺寸
        let monitor = self.handle.primary_monitor()?.ok_or(anyhow!("No monitor"))?;
        let size = monitor.size();
        
        // 右下角，距离边缘 20px
        let x = (size.width as i32) - 280 - 20;
        let y = (size.height as i32) - 400 - 20;
        
        Ok((x, y))
    }
}
```

### 2. Tauri Commands (`commands/floating.rs`)

```rust
/// 获取所有站点余额汇总
#[tauri::command]
pub async fn get_all_sites_quota(
    state: State<'_, AppState>
) -> Result<Vec<SiteQuotaSummary>, String> {
    let sites = state.repo.site.list_all().await
        .map_err(|e| e.to_string())?;
    
    let mut summaries = Vec::new();
    
    for site in sites {
        // 解析余额信息
        let (balance, raw_balance, is_unlimited, is_low) = parse_balance(&site);
        
        summaries.push(SiteQuotaSummary {
            id: site.id,
            name: site.name,
            balance,
            raw_balance,
            is_unlimited,
            is_low,
            last_updated: site.last_check_time,
        });
    }
    
    Ok(summaries)
}

/// 切换悬浮窗显示/隐藏
#[tauri::command]
pub async fn toggle_floating_window(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let settings = state.repo.settings.get_floating_window_settings().await
        .map_err(|e| e.to_string())?;
    
    let manager = FloatingWindowManager::new(app);
    
    if settings.enabled {
        manager.hide().map_err(|e| e.to_string())?;
        state.repo.settings.update_floating_window_enabled(false).await
            .map_err(|e| e.to_string())?;
        Ok(false)
    } else {
        manager.show().map_err(|e| e.to_string())?;
        state.repo.settings.update_floating_window_enabled(true).await
            .map_err(|e| e.to_string())?;
        Ok(true)
    }
}

/// 保存悬浮窗位置
#[tauri::command]
pub async fn save_floating_window_position(
    state: State<'_, AppState>,
    x: i32,
    y: i32,
) -> Result<(), String> {
    state.repo.settings.update_floating_window_position(x, y).await
        .map_err(|e| e.to_string())
}

/// 更新悬浮窗设置
#[tauri::command]
pub async fn update_floating_window_settings(
    state: State<'_, AppState>,
    settings: FloatingWindowSettings,
) -> Result<(), String> {
    state.repo.settings.update_floating_window_settings(settings).await
        .map_err(|e| e.to_string())
}
```

### 3. 后台刷新任务

```rust
// lib.rs - 启动后台刷新
pub fn start_floating_window_refresh_task(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            // 读取设置
            let state = app.state::<AppState>();
            let settings = match state.repo.settings.get_floating_window_settings().await {
                Ok(s) => s,
                Err(_) => {
                    tokio::time::sleep(Duration::from_secs(60)).await;
                    continue;
                }
            };
            
            if !settings.enabled {
                tokio::time::sleep(Duration::from_secs(60)).await;
                continue;
            }
            
            // 刷新余额（这里可以调用现有的同步逻辑）
            let _ = refresh_all_sites_quota(&state).await;
            
            // 通知前端更新
            if let Some(window) = app.get_window("floating") {
                let _ = window.emit("quota-updated", ());
            }
            
            // 等待下一次刷新
            tokio::time::sleep(Duration::from_secs(settings.auto_refresh_seconds)).await;
        }
    });
}
```

## 前端设计

### 路由配置

```typescript
// router/index.ts
{
  path: '/floating',
  name: 'Floating',
  component: () => import('@/pages/FloatingWindow.vue')
}
```

### 组件结构

```
FloatingWindow.vue
├── CollapsedView.vue (收起状态)
│   └── 圆形按钮 + 图标
└── ExpandedView.vue (展开状态)
    ├── Header (标题栏 + 刷新 + 收起按钮)
    ├── QuotaList (余额列表)
    │   └── QuotaItem × N
    └── Footer (最后更新时间)
```

### 样式设计

```css
/* 毛玻璃背景 */
.floating-window {
  background: rgba(15, 19, 28, 0.85);
  backdrop-filter: blur(20px);
  border-radius: 16px;
  border: 1px solid rgba(255, 255, 255, 0.1);
}

/* 收起状态 */
.collapsed {
  width: 60px;
  height: 60px;
  border-radius: 50%;
}

/* 展开状态 */
.expanded {
  width: 280px;
  height: 400px;
  border-radius: 16px;
}

/* 余额项 */
.quota-item {
  padding: 12px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.05);
}

.quota-item.low {
  background: rgba(239, 68, 68, 0.1);
  border-left: 3px solid #ef4444;
}
```

## 实现顺序

### Phase 1: 基础架构（2-3小时）
1. ✅ 数据库迁移 - 添加悬浮窗配置字段
2. ✅ Rust 数据模型 - `FloatingWindowSettings` + `SiteQuotaSummary`
3. ✅ Repository 扩展 - 悬浮窗配置的 CRUD
4. ✅ 窗口管理模块 - `floating_window.rs`
5. ✅ Tauri Commands - 基础命令注册

### Phase 2: 核心功能（3-4小时）
1. ✅ 余额汇总接口 - `get_all_sites_quota`
2. ✅ 前端页面 - `FloatingWindow.vue` 基础框架
3. ✅ 展开状态 UI - 余额列表显示
4. ✅ 收起/展开切换
5. ✅ 窗口拖动与位置保存

### Phase 3: 自动刷新（2小时）
1. ✅ 后台定时任务
2. ✅ 事件驱动更新
3. ✅ 手动刷新按钮
4. ✅ 加载状态显示

### Phase 4: 设置集成（1-2小时）
1. ✅ 设置页面 UI
2. ✅ 开关控制
3. ✅ 刷新间隔配置
4. ✅ 重置位置功能

### Phase 5: 视觉打磨（2-3小时）
1. ✅ 毛玻璃效果
2. ✅ 深色主题适配
3. ✅ 动画效果
4. ✅ 低余额高亮
5. ✅ 整体视觉调优

## 风险与注意事项

1. **窗口管理**
   - Tauri 2 的窗口 API 可能有平台差异，需要测试
   - 窗口位置保存需要考虑多显示器场景
   - 窗口拖动时需要防止移出屏幕

2. **性能**
   - 独立 WebView 会增加内存占用（约 30-50MB）
   - 后台定时器要确保正确清理，避免泄漏
   - 余额刷新要节流，避免频繁请求

3. **用户体验**
   - 窗口默认位置要避免遮挡常用区域
   - 收起状态要足够小，不打扰
   - 展开状态要清晰显示所有信息

4. **兼容性**
   - macOS 支持需要测试（Tauri 2 跨平台）
   - 不同 DPI 下的显示效果
   - 不同屏幕尺寸下的默认位置
