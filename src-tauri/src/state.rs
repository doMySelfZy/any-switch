use crate::crypto::Crypto;
use crate::db::Db;
use crate::error::AppResult;
use std::sync::atomic::{AtomicBool, AtomicI64};

pub struct AppState {
    pub db: Db,
    pub crypto: Crypto,
    pub close_to_tray: AtomicBool,
    pub start_in_tray: AtomicBool,
    pub is_quitting: AtomicBool,
    pub webdav_sync_handle: tokio::sync::Mutex<Option<tokio::task::JoinHandle<()>>>,
    pub webdav_operation: tokio::sync::Mutex<()>,
    pub next_webdav_sync_at: AtomicI64,
    /// 本地代理运行时句柄；None 表示未运行。
    pub local_proxy: tokio::sync::Mutex<Option<crate::local_proxy::server::ProxyRuntime>>,
}

impl AppState {
    pub fn init() -> AppResult<Self> {
        let app_dir = crate::paths::app_dir()?;
        let restore_outcome = crate::pending_restore::apply_pending_restore(&app_dir)?;
        if let Some(outcome) = &restore_outcome {
            match outcome.status.as_str() {
                "applied" => tracing::info!("pending application restore completed"),
                _ => {
                    tracing::error!(message = %outcome.message, "pending application restore failed safely")
                }
            }
        }
        let db = Db::open()?;
        if restore_outcome
            .as_ref()
            .is_some_and(|outcome| outcome.status == "applied")
        {
            db.with_conn(crate::repo::webdav::clear_sync_status)?;
        }
        let settings = db.with_conn(crate::repo::settings::get_settings)?;
        let has_sites = db.with_conn(|c| crate::repo::site::has_encrypted_sites(c))?;
        let crypto = Crypto::ensure_can_decrypt_db(has_sites)?;
        Ok(Self {
            db,
            crypto,
            close_to_tray: AtomicBool::new(settings.close_to_tray),
            start_in_tray: AtomicBool::new(settings.start_in_tray),
            is_quitting: AtomicBool::new(false),
            webdav_sync_handle: tokio::sync::Mutex::new(None),
            webdav_operation: tokio::sync::Mutex::new(()),
            next_webdav_sync_at: AtomicI64::new(0),
            local_proxy: tokio::sync::Mutex::new(None),
        })
    }
}
