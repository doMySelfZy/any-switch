use crate::domain::{LocalProxyRequestLogEntry, LocalProxyStatus, ProxyHeader, TargetKind};
use crate::error::{AppError, AppResult};
use crate::local_proxy;
use crate::repo;
use crate::state::AppState;
use tauri::{Manager, State};

#[tauri::command]
pub fn local_proxy_status(app: tauri::AppHandle) -> AppResult<LocalProxyStatus> {
    local_proxy::status(&app)
}

/// 启动代理。运行中重复调用为无操作；端口被占用时返回 `proxy_bind_failed`。
#[tauri::command]
pub async fn start_local_proxy(app: tauri::AppHandle) -> AppResult<LocalProxyStatus> {
    local_proxy::server::start(&app).await?;
    local_proxy::status(&app)
}

#[tauri::command]
pub async fn stop_local_proxy(app: tauri::AppHandle) -> AppResult<LocalProxyStatus> {
    local_proxy::server::stop(&app).await?;
    local_proxy::status(&app)
}

/// 打开/关闭某个目标的接管：改设置里的接管集合，并立即重写该目标配置。
///
/// 打开接管要求代理**此刻在监听**：只检查"意图"会允许客户端指向一个 bind 失败
/// 而没人监听的端口。
#[tauri::command]
pub fn set_local_proxy_takeover(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    target: TargetKind,
    enabled: bool,
) -> AppResult<LocalProxyStatus> {
    if enabled {
        let running = state
            .local_proxy
            .try_lock()
            .map(|guard| guard.is_some())
            .unwrap_or(true);
        if !running {
            return Err(AppError::new(
                "proxy_not_running",
                "start the local proxy before taking over a target",
            ));
        }
    }
    let current = state.db.with_conn(repo::settings::get_settings)?;
    let mut targets: Vec<TargetKind> = current
        .local_proxy_targets
        .iter()
        .copied()
        .filter(|t| *t != target)
        .collect();
    if enabled {
        targets.push(target);
    }
    let next = state.db.with_conn(|c| {
        repo::settings::merge_settings(
            c,
            serde_json::json!({ "localProxyTargets": targets }),
        )
    })?;

    // 立即让客户端配置反映新地址：接管开 → 写代理地址；接管关 → 写回真实上游。
    if let Err(error) = crate::route_switch::sync_applied_target(&state, target, &next) {
        tracing::warn!(
            target = target.as_str(),
            error = %error,
            "local proxy takeover rewrite failed"
        );
        return Err(error);
    }
    local_proxy::status(&app)
}

/// 保存监听端口，并在需要时把接管目标按新端口重写。
///
/// 顺序很重要：先落库、再重启监听（新端口）、最后重写客户端配置。若端口被占用，
/// 监听失败会返回错误且**不改**客户端配置——否则四个 CLI 会指向一个不存在的端口。
#[tauri::command]
pub async fn set_local_proxy_port(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    port: u16,
) -> AppResult<LocalProxyStatus> {
    let next = state.db.with_conn(|c| {
        repo::settings::merge_settings(c, serde_json::json!({ "localProxyPort": port }))
    })?;
    if next.local_proxy_port == port {
        // merge 会夹取端口；这里读回落库后的值，保证报错信息与最终状态一致。
    }

    let was_running = state
        .local_proxy
        .try_lock()
        .map(|guard| guard.is_some())
        .unwrap_or(false);
    if was_running {
        local_proxy::server::stop(&app).await?;
        local_proxy::server::start(&app).await?;
        for target in next.local_proxy_targets.iter().copied() {
            if let Err(error) = crate::route_switch::sync_applied_target(&state, target, &next) {
                tracing::warn!(
                    target = target.as_str(),
                    error = %error,
                    "local proxy port change: target rewrite failed"
                );
            }
        }
    }
    local_proxy::status(&app)
}

#[tauri::command]
pub fn list_local_proxy_requests(
    app: tauri::AppHandle,
    limit: Option<usize>,
) -> AppResult<Vec<LocalProxyRequestLogEntry>> {
    let state = app.state::<AppState>();
    let guard = state
        .local_proxy
        .try_lock()
        .map_err(|_| AppError::new("internal", "local proxy state busy"))?;
    Ok(match guard.as_ref() {
        Some(runtime) => runtime.log.list(limit.unwrap_or(100)),
        None => Vec::new(),
    })
}

/// 清空请求日志（UI 的"清空"按钮）。
#[tauri::command]
pub fn clear_local_proxy_requests(app: tauri::AppHandle) -> AppResult<()> {
    let state = app.state::<AppState>();
    let guard = state
        .local_proxy
        .try_lock()
        .map_err(|_| AppError::new("internal", "local proxy state busy"))?;
    if let Some(runtime) = guard.as_ref() {
        runtime.log.clear();
    }
    Ok(())
}

/// 读取某站点已配置的代理请求头（按需解密，仅编辑时调用）。
#[tauri::command]
pub fn get_site_proxy_headers(
    state: State<'_, AppState>,
    site_id: String,
) -> AppResult<Vec<ProxyHeader>> {
    state
        .db
        .with_conn(|c| repo::site::get_site_proxy_headers(c, &state.crypto, &site_id))
}
