use tauri::{App, AppHandle, Manager, PhysicalPosition, WebviewUrl, WebviewWindowBuilder};
use crate::state::AppState;
use crate::repo;
use crate::error::{AppResult, AppError};

/// 悬浮窗标识符
pub const FLOATING_WINDOW_LABEL: &str = "floating";

/// 初始化悬浮窗（如果设置中已启用）
pub fn init_floating_window(app: &App) -> AppResult<()> {
    let state = app.state::<AppState>();
    let enabled = state.db.with_conn(|conn| {
        let settings = repo::settings::get_settings(conn)?;
        Ok(settings.floating_window.enabled)
    })?;

    if enabled {
        create_floating_window(app.handle().clone())?;
    }

    Ok(())
}

/// 创建悬浮窗
pub fn create_floating_window(app: AppHandle) -> AppResult<()> {
    // 检查窗口是否已存在
    if app.get_webview_window(FLOATING_WINDOW_LABEL).is_some() {
        return Ok(());
    }

    let state = app.state::<AppState>();
    let (position_x, position_y) = state.db.with_conn(|conn| {
        let settings = repo::settings::get_settings(conn)?;
        Ok((settings.floating_window.position_x, settings.floating_window.position_y))
    })?;

    // 创建悬浮窗
    let window = WebviewWindowBuilder::new(
        &app,
        FLOATING_WINDOW_LABEL,
        WebviewUrl::App("/floating".into())
    )
    .title("小白Switch - 余额")
    .inner_size(280.0, 400.0)
    .min_inner_size(280.0, 200.0)
    .max_inner_size(400.0, 800.0)
    .resizable(true)
    .decorations(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .visible(false) // 先隐藏，加载完成后再显示
    .build()
    .map_err(|e| AppError::new("window_error", format!("Failed to create floating window: {}", e)))?;

    // 设置窗口位置
    if let (Some(x), Some(y)) = (position_x, position_y) {
        window.set_position(PhysicalPosition::new(x, y))
            .map_err(|e| AppError::new("window_error", format!("Failed to set window position: {}", e)))?;
    } else {
        // 默认位置：右下角
        if let Some(monitor) = window.current_monitor()
            .map_err(|e| AppError::new("window_error", format!("Failed to get monitor: {}", e)))? {
            let monitor_size = monitor.size();
            let window_size = window.inner_size()
                .map_err(|e| AppError::new("window_error", format!("Failed to get window size: {}", e)))?;
            let x = monitor_size.width as i32 - window_size.width as i32 - 20;
            let y = monitor_size.height as i32 - window_size.height as i32 - 60; // 留出任务栏空间
            window.set_position(PhysicalPosition::new(x, y))
                .map_err(|e| AppError::new("window_error", format!("Failed to set window position: {}", e)))?;
        }
    }

    // 窗口加载完成后显示
    window.show()
        .map_err(|e| AppError::new("window_error", format!("Failed to show window: {}", e)))?;

    Ok(())
}

/// 显示悬浮窗
pub fn show_floating_window(app: AppHandle) -> AppResult<()> {
    if let Some(window) = app.get_webview_window(FLOATING_WINDOW_LABEL) {
        window.show()
            .map_err(|e| AppError::new("window_error", format!("Failed to show window: {}", e)))?;
        window.set_focus()
            .map_err(|e| AppError::new("window_error", format!("Failed to focus window: {}", e)))?;
    } else {
        create_floating_window(app)?;
    }
    Ok(())
}

/// 隐藏悬浮窗
pub fn hide_floating_window(app: AppHandle) -> AppResult<()> {
    if let Some(window) = app.get_webview_window(FLOATING_WINDOW_LABEL) {
        window.hide()
            .map_err(|e| AppError::new("window_error", format!("Failed to hide window: {}", e)))?;
    }
    Ok(())
}

/// 关闭悬浮窗
pub fn close_floating_window(app: AppHandle) -> AppResult<()> {
    if let Some(window) = app.get_webview_window(FLOATING_WINDOW_LABEL) {
        window.close()
            .map_err(|e| AppError::new("window_error", format!("Failed to close window: {}", e)))?;
    }
    Ok(())
}
