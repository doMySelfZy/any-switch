use tauri::{AppHandle, Manager, State};

use crate::floating_window;
use crate::state::AppState;
use crate::domain::SiteQuotaSummary;
use crate::repo;
use crate::error::AppResult;

/// 获取所有站点的余额汇总
#[tauri::command]
pub async fn get_all_sites_quota(state: State<'_, AppState>) -> AppResult<Vec<SiteQuotaSummary>> {
    state.db.with_conn(|conn| {
        let sites = repo::site::list_sites(conn)?;
        let mut summaries = Vec::new();

        for site in sites {
            // 尝试获取最新的余额信息
            let quota = if !site.api_key_encrypted.is_empty() {
                // 这里可以调用 probe_quota，但为了避免阻塞，我们可以从缓存或数据库读取
                // 暂时返回 None，后续可以添加缓存机制
                None
            } else {
                None
            };

            summaries.push(SiteQuotaSummary {
                site_id: site.id.clone(),
                site_name: site.name.clone(),
                quota,
                enabled: site.enabled,
                sort_order: site.sort_order,
            });
        }

        Ok(summaries)
    })
}

/// 切换悬浮窗显示/隐藏
#[tauri::command]
pub async fn toggle_floating_window(
    app: AppHandle,
    state: State<'_, AppState>,
) -> AppResult<bool> {
    let enabled = state.db.with_conn(|conn| {
        let settings = repo::settings::get_settings(conn)?;
        Ok(settings.floating_window.enabled)
    })?;

    if enabled {
        if let Some(window) = app.get_webview_window(floating_window::FLOATING_WINDOW_LABEL) {
            let visible = window.is_visible()
                .map_err(|e| crate::error::AppError::new("window_error", e.to_string()))?;
            if visible {
                floating_window::hide_floating_window(app)?;
                Ok(false)
            } else {
                floating_window::show_floating_window(app)?;
                Ok(true)
            }
        } else {
            floating_window::create_floating_window(app)?;
            Ok(true)
        }
    } else {
        Err(crate::error::AppError::new("validation_failed", "Floating window is disabled in settings"))
    }
}

/// 显示悬浮窗
#[tauri::command]
pub async fn show_floating_window_cmd(app: AppHandle) -> AppResult<()> {
    floating_window::show_floating_window(app)?;
    Ok(())
}

/// 隐藏悬浮窗
#[tauri::command]
pub async fn hide_floating_window_cmd(app: AppHandle) -> AppResult<()> {
    floating_window::hide_floating_window(app)?;
    Ok(())
}

/// 保存悬浮窗位置
#[tauri::command]
pub async fn save_floating_window_position(
    app: AppHandle,
    state: State<'_, AppState>,
    x: i32,
    y: i32,
) -> AppResult<()> {
    state.db.with_conn(|conn| {
        let mut settings = repo::settings::get_settings(conn)?;
        settings.floating_window.position_x = Some(x);
        settings.floating_window.position_y = Some(y);
        repo::settings::save_settings(conn, &settings)?;
        Ok(())
    })
}

/// 启用/禁用悬浮窗
#[tauri::command]
pub async fn set_floating_window_enabled(
    app: AppHandle,
    state: State<'_, AppState>,
    enabled: bool,
) -> AppResult<()> {
    state.db.with_conn(|conn| {
        let mut settings = repo::settings::get_settings(conn)?;
        settings.floating_window.enabled = enabled;
        repo::settings::save_settings(conn, &settings)?;
        Ok(())
    })?;

    if enabled {
        floating_window::show_floating_window(app)?;
    } else {
        floating_window::close_floating_window(app)?;
    }

    Ok(())
}

/// 设置自动刷新间隔（分钟）
#[tauri::command]
pub async fn set_floating_window_refresh_interval(
    state: State<'_, AppState>,
    minutes: u32,
) -> AppResult<()> {
    state.db.with_conn(|conn| {
        let mut settings = repo::settings::get_settings(conn)?;
        settings.floating_window.auto_refresh_minutes = crate::domain::clamp_floating_refresh_interval(minutes);
        repo::settings::save_settings(conn, &settings)?;
        Ok(())
    })
}

/// 重置悬浮窗位置到默认（右下角）
#[tauri::command]
pub async fn reset_floating_window_position(
    app: AppHandle,
    state: State<'_, AppState>,
) -> AppResult<()> {
    if let Some(window) = app.get_webview_window(floating_window::FLOATING_WINDOW_LABEL) {
        if let Some(monitor) = window.current_monitor()
            .map_err(|e| crate::error::AppError::new("window_error", e.to_string()))?
        {
            let monitor_size = monitor.size();
            let window_size = window.inner_size()
                .map_err(|e| crate::error::AppError::new("window_error", e.to_string()))?;
            let x = monitor_size.width as i32 - window_size.width as i32 - 20;
            let y = monitor_size.height as i32 - window_size.height as i32 - 60;

            window.set_position(tauri::PhysicalPosition::new(x, y))
                .map_err(|e| crate::error::AppError::new("window_error", e.to_string()))?;

            state.db.with_conn(|conn| {
                let mut settings = repo::settings::get_settings(conn)?;
                settings.floating_window.position_x = Some(x);
                settings.floating_window.position_y = Some(y);
                repo::settings::save_settings(conn, &settings)?;
                Ok(())
            })?;
        }
    }

    Ok(())
}
