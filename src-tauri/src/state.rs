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
            match outcome.result.status.as_str() {
                "applied" => tracing::info!("pending application restore completed"),
                _ => tracing::error!(
                    message = %outcome.result.message,
                    "pending application restore failed safely"
                ),
            }
        }
        let db = Db::open()?;
        if let Some(applied) = restore_outcome
            .as_ref()
            .filter(|outcome| outcome.result.status == "applied")
        {
            db.with_conn(crate::repo::webdav::clear_sync_status)?;
            // 下载记账在此提交：此时远端数据才真正落地（换库发生在 Db::open 之前），
            // 且后续 schema 迁移可能已改动本地数据。提交值必须是远端 manifest 声明的指纹：
            // 迁移补表后本地重算值不等于它，判定会落入 remote == last → 重新发布升级后的数据；
            // 若误记本地重算值，则会因 last == local 而反复下载。
            if let Some(fingerprint) = &applied.synced_fingerprint {
                db.with_conn(|conn| crate::sync::save_last_synced(conn, fingerprint))?;
            }
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
