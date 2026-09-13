use crate::app_backup;
use crate::domain::SyncOutcome;
use crate::error::{AppError, AppResult};
use crate::repo;
use crate::state::AppState;
use crate::webdav::WebDavClient;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::{AppHandle, Manager};

/// WebDAV 同步版本指针文件名。属于跨机内部协议标识：更名前后必须保持一致，
/// 否则新旧版本机器互相读不到版本指针，会误判并相互覆盖。
pub const SYNC_MANIFEST_FILE_NAME: &str = "xiaobai-switch-sync.json";
pub const SYNC_FORMAT_VERSION: u32 = 1;
pub const MAX_MANIFEST_BYTES: u64 = 64 * 1024;

const META_LAST_SYNCED_DATABASE_SHA256: &str = "last_synced_database_sha256";
const META_LAST_SYNCED_MASTER_KEY_SHA256: &str = "last_synced_master_key_sha256";
const META_LAST_PUBLISHED_REVISION: &str = "last_published_revision";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataFingerprint {
    pub database_sha256: String,
    pub master_key_sha256: String,
}

impl DataFingerprint {
    fn from_parts(database_sha256: String, master_key_sha256: String) -> Self {
        Self {
            database_sha256,
            master_key_sha256,
        }
    }
}

/// 参与内容指纹的业务表（引擎记账表与 WebDAV 配置不参与）。
const FINGERPRINT_TABLES: [&str; 7] = [
    "settings",
    "sites",
    "site_api_keys",
    "site_models",
    "site_thinking_presets",
    "target_bindings",
    "apply_records",
];

/// 逻辑内容指纹：按表遍历全部业务行做稳定哈希。
/// 不使用数据库文件字节 hash——SQLite 文件头部（change counter 等）每次
/// VACUUM 都会变化，会让"内容没变指纹却变了"，导致同步误判反复应用。
pub fn compute_logical_fingerprint(
    conn: &Connection,
    master_key: &[u8],
) -> AppResult<DataFingerprint> {
    use rusqlite::types::Value;
    let mut hasher = Sha256::new();
    for table in FINGERPRINT_TABLES {
        hasher.update(table.as_bytes());
        hasher.update([0]);
        let mut statement = conn.prepare(&format!("SELECT * FROM {table} ORDER BY rowid"))?;
        let column_count = statement.column_count();
        let mut rows = statement.query([])?;
        while let Some(row) = rows.next()? {
            for index in 0..column_count {
                let value: Value = row.get(index)?;
                match value {
                    Value::Null => hasher.update([0_u8]),
                    Value::Integer(number) => {
                        hasher.update([1_u8]);
                        hasher.update(number.to_le_bytes());
                    }
                    Value::Real(number) => {
                        hasher.update([2_u8]);
                        hasher.update(number.to_le_bytes());
                    }
                    Value::Text(ref text) => {
                        hasher.update([3_u8]);
                        hasher.update((text.len() as u64).to_le_bytes());
                        hasher.update(text.as_bytes());
                    }
                    Value::Blob(ref bytes) => {
                        hasher.update([4_u8]);
                        hasher.update((bytes.len() as u64).to_le_bytes());
                        hasher.update(bytes);
                    }
                }
            }
            hasher.update([255_u8]);
        }
        hasher.update([254_u8]);
    }
    Ok(DataFingerprint::from_parts(
        hex::encode(hasher.finalize()),
        hex::encode(Sha256::digest(master_key)),
    ))
}

/// 对一个数据库文件计算逻辑指纹（校验下载 bundle 的内容时使用）。
pub fn fingerprint_database_file(database_path: &std::path::Path, master_key: &[u8]) -> AppResult<DataFingerprint> {
    let conn = Connection::open(database_path)
        .map_err(|e| AppError::new("sync_manifest_invalid", format!("cannot open extracted database: {e}")))?;
    compute_logical_fingerprint(&conn, master_key)
}

/// 远端"版本指针"：谁在何时上传了哪个数据包。
/// 判断新旧唯一依据是数据指纹；revision 只用于展示与遥测。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncManifest {
    pub format_version: u32,
    pub revision: u64,
    pub device_name: String,
    pub updated_at: i64,
    pub bundle_file_name: String,
    pub database_sha256: String,
    pub master_key_sha256: String,
    pub app_version: String,
}

impl SyncManifest {
    fn fingerprint(&self) -> DataFingerprint {
        DataFingerprint {
            database_sha256: self.database_sha256.clone(),
            master_key_sha256: self.master_key_sha256.clone(),
        }
    }

    fn validate(&self) -> AppResult<()> {
        if self.format_version != SYNC_FORMAT_VERSION {
            return Err(AppError::new(
                "sync_manifest_invalid",
                format!("unsupported sync manifest format: {}", self.format_version),
            ));
        }
        if self.revision == 0 || self.device_name.trim().is_empty() {
            return Err(AppError::new(
                "sync_manifest_invalid",
                "sync manifest revision and device are required",
            ));
        }
        crate::webdav::validate_backup_file_name(&self.bundle_file_name)?;
        for hash in [&self.database_sha256, &self.master_key_sha256] {
            if hash.len() != 64 || !hash.chars().all(|ch| ch.is_ascii_hexdigit()) {
                return Err(AppError::new(
                    "sync_manifest_invalid",
                    "sync manifest checksums must be sha256 hex digests",
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncAction {
    Upload,
    Download,
    InSync,
}

/// 纯决策：比较本地指纹、远端指纹与上次同步指纹。
/// 冲突（两侧都变过）在 Phase A 策略下以远端为准，本地先快照留底。
fn decide_action(
    local: &DataFingerprint,
    remote: Option<&SyncManifest>,
    last_synced: Option<&DataFingerprint>,
) -> (SyncAction, bool) {
    let Some(remote) = remote else {
        return (SyncAction::Upload, false);
    };
    let remote_fp = remote.fingerprint();
    if remote_fp == *local {
        return (SyncAction::InSync, false);
    }
    match last_synced {
        // 首次接触：以云端为准（本地未同步的改动已被快照兜底）。
        None => (SyncAction::Download, false),
        Some(last) if remote_fp == *last => (SyncAction::Upload, false),
        Some(last) if *last == *local => (SyncAction::Download, false),
        Some(_) => (SyncAction::Download, true),
    }
}

pub async fn run_sync(
    app: &AppHandle,
    state: &AppState,
    reason: &str,
) -> AppResult<SyncOutcome> {
    let _guard = state.webdav_operation.lock().await;
    state
        .db
        .with_conn(|conn| repo::webdav::record_sync_status(conn, "running", None, false))?;

    let outcome = run_sync_inner(app, state, reason).await;

    match &outcome {
        Ok(outcome) => {
            let status = if outcome.conflict || outcome.warning.is_some() {
                "warning"
            } else {
                "success"
            };
            let note = outcome.warning.clone().or_else(|| {
                outcome.conflict.then(|| {
                    "sync conflict: both sides changed since the last sync; applied the remote data after taking a local snapshot".to_string()
                })
            });
            state.db.with_conn(|conn| {
                repo::webdav::record_sync_status(conn, status, note.as_deref(), true)
            })?;
        }
        Err(error) => {
            let message = error.to_string();
            state.db.with_conn(|conn| {
                repo::webdav::record_sync_status(conn, "failed", Some(&message), false)
            })?;
        }
    }
    outcome
}

async fn run_sync_inner(
    app: &AppHandle,
    state: &AppState,
    reason: &str,
) -> AppResult<SyncOutcome> {
    let stored = state
        .db
        .with_conn(repo::webdav::get_config)?
        .ok_or_else(|| AppError::new("webdav_not_configured", "WebDAV is not configured"))?;
    let settings = state.db.with_conn(repo::settings::get_settings)?;
    let client = WebDavClient::new(
        crate::commands::webdav::runtime_config(&stored, &state.crypto)?,
        &settings,
    )?;

    let app_dir = crate::paths::app_dir()?;
    let key_bytes = std::fs::read(&crate::paths::master_key_path()?)
        .map_err(|e| AppError::new("sync_failed", format!("cannot read master.key: {e}")))?;
    if key_bytes.len() != 32 {
        return Err(AppError::new("sync_failed", "master.key must contain exactly 32 bytes"));
    }
    let local_fp = state
        .db
        .with_conn(|conn| compute_logical_fingerprint(conn, &key_bytes))?;
    let temp_dir = tempfile::Builder::new()
        .prefix(".sync-")
        .tempdir_in(&app_dir)?;
    let local_bundle = state.db.with_conn(|conn| {
        app_backup::create_backup_in(
            conn,
            &crate::paths::master_key_path()?,
            temp_dir.path(),
            reason,
        )
    })?;
    let device_name = app_backup::parse_device_from_filename(&local_bundle.file_name);

    let remote = client.download_sync_manifest().await?;
    let last_synced = load_last_synced(state)?;
    let (action, conflict) = decide_action(&local_fp, remote.as_ref(), last_synced.as_ref());

    match action {
        SyncAction::InSync => {
            save_last_synced(state, &local_fp)?;
            Ok(SyncOutcome {
                action: "in_sync".into(),
                revision: remote.map(|manifest| manifest.revision).unwrap_or(0),
                bundle_file_name: None,
                conflict: false,
                pending_restart: false,
                warning: None,
            })
        }
        SyncAction::Upload => {
            let next_revision = remote.as_ref().map(|m| m.revision + 1).unwrap_or(1);
            client
                .upload_file(&local_bundle.file_name, &local_bundle.path)
                .await?;
            let manifest = SyncManifest {
                format_version: SYNC_FORMAT_VERSION,
                revision: next_revision,
                device_name: device_name.clone(),
                updated_at: chrono::Utc::now().timestamp_millis(),
                bundle_file_name: local_bundle.file_name.clone(),
                database_sha256: local_fp.database_sha256.clone(),
                master_key_sha256: local_fp.master_key_sha256.clone(),
                app_version: env!("CARGO_PKG_VERSION").into(),
            };
            client.upload_sync_manifest(&manifest).await?;
            // 按保留数清理本设备在云端的旧数据包，防止无限堆积。
            let device = app_backup::parse_device_from_filename(&local_bundle.file_name);
            let warning = client
                .cleanup_device_backups(&device, stored.max_remote_backups)
                .await
                .err()
                .map(|error| {
                    format!("synced, but old remote bundles could not be pruned: {error}")
                });
            save_last_synced(state, &local_fp)?;
            state.db.with_conn(|conn| {
                repo::sync_meta::set_meta(
                    conn,
                    META_LAST_PUBLISHED_REVISION,
                    &next_revision.to_string(),
                )
            })?;
            Ok(SyncOutcome {
                action: "upload".into(),
                revision: next_revision,
                bundle_file_name: Some(local_bundle.file_name),
                conflict: false,
                pending_restart: false,
                warning,
            })
        }
        SyncAction::Download => {
            let remote = remote.expect("download decision requires a remote manifest");
            let archive = temp_dir.path().join(&remote.bundle_file_name);
            client
                .download_file(&remote.bundle_file_name, &archive)
                .await?;
            let extract_dir = tempfile::Builder::new()
                .prefix(".sync-apply-")
                .tempdir_in(&app_dir)?;
            let validated =
                app_backup::validate_and_extract_bundle(&archive, extract_dir.path())?;
            // 内容校验：解包后的数据库逻辑指纹必须与远端版本指针一致
            // （zip 内部的文件级 sha256 校验已在 validate_and_extract_bundle 完成）。
            let extracted_key = std::fs::read(&validated.master_key_path)?;
            let extracted_fp =
                fingerprint_database_file(&validated.database_path, &extracted_key)?;
            if extracted_fp != remote.fingerprint() {
                return Err(AppError::new(
                    "sync_manifest_invalid",
                    "downloaded bundle does not match the remote sync manifest",
                ));
            }
            // 应用前强制本地快照：任何远端数据替换都可回滚。
            state.db.with_conn(|conn| {
                app_backup::create_local_backup(conn, "pre_sync_apply", settings.max_backup_copies)
            })?;
            save_last_synced(state, &remote.fingerprint())?;
            crate::pending_restore::queue_pending_restore(&archive, &app_dir)?;
            Ok(SyncOutcome {
                action: "download".into(),
                revision: remote.revision,
                bundle_file_name: Some(remote.bundle_file_name),
                conflict,
                pending_restart: true,
                warning: None,
            })
        }
    }
}

fn load_last_synced(state: &AppState) -> AppResult<Option<DataFingerprint>> {
    state.db.with_conn(|conn| {
        let database = repo::sync_meta::get_meta(conn, META_LAST_SYNCED_DATABASE_SHA256)?;
        let master_key = repo::sync_meta::get_meta(conn, META_LAST_SYNCED_MASTER_KEY_SHA256)?;
        Ok(match (database, master_key) {
            (Some(database_sha256), Some(master_key_sha256)) => Some(DataFingerprint {
                database_sha256,
                master_key_sha256,
            }),
            _ => None,
        })
    })
}

fn save_last_synced(state: &AppState, fingerprint: &DataFingerprint) -> AppResult<()> {
    state.db.with_conn(|conn| {
        repo::sync_meta::set_meta(
            conn,
            META_LAST_SYNCED_DATABASE_SHA256,
            &fingerprint.database_sha256,
        )?;
        repo::sync_meta::set_meta(
            conn,
            META_LAST_SYNCED_MASTER_KEY_SHA256,
            &fingerprint.master_key_sha256,
        )
    })
}

// ---------------------------------------------------------------------------
// Phase B：变更驱动触发
// ---------------------------------------------------------------------------

static SYNC_DIRTY: AtomicBool = AtomicBool::new(false);
static SYNC_POLL_REQUESTED: AtomicBool = AtomicBool::new(false);

/// 引擎自身的记账表，写入它们不算"数据变更"。
const NOISY_SYNC_TABLES: [&str; 2] = ["sync_meta", "webdav_sync_state"];

pub(crate) fn is_noisy_sync_table(table: &str) -> bool {
    NOISY_SYNC_TABLES.contains(&table)
}

/// SQLite update_hook 回调：任何业务数据写入都会标记脏。
pub fn note_db_write(table: &str) {
    if !is_noisy_sync_table(table) {
        SYNC_DIRTY.store(true, Ordering::Relaxed);
    }
}

/// 请求做一次完整同步决策（启动、窗口聚焦时调用）。
pub fn request_sync_poll() {
    SYNC_POLL_REQUESTED.store(true, Ordering::Relaxed);
}

/// 给应用数据库挂上变更钩子（Db::open 时调用一次）。
pub fn install_db_hook(conn: &rusqlite::Connection) {
    // 用具名函数而非闭包：闭包在这里会触发 HRTB 生命周期推断失败。
    conn.update_hook(Some(sync_update_hook));
}

fn sync_update_hook(_action: rusqlite::hooks::Action, _db: &str, table: &str, _rowid: i64) {
    note_db_write(table);
}

/// 同步守护任务：5 秒一轮，处理三类触发——
/// 1) 启动后首轮决策（换机打开即拉取）；2) 数据变更（防抖后上传）；
/// 3) 窗口聚焦（切回窗口时决策一次）。定时器只作为兜底留在调度器里。
pub fn spawn_sync_daemon(app: AppHandle) {
    request_sync_poll();
    tauri::async_runtime::spawn(async move {
        let mut first_poll_done = false;
        loop {
            tokio::time::sleep(Duration::from_secs(5)).await;
            let dirty = SYNC_DIRTY.swap(false, Ordering::Relaxed);
            let poll = SYNC_POLL_REQUESTED.swap(false, Ordering::Relaxed);
            if !dirty && !poll {
                continue;
            }
            // 防抖：等连续写突发平息后再打包。
            tokio::time::sleep(Duration::from_secs(3)).await;
            let state = app.state::<AppState>();
            let enabled = state.db.with_conn(|conn| {
                Ok(repo::webdav::get_config(conn)?.is_some_and(|config| config.auto_sync_enabled))
            });
            let enabled = match enabled {
                Ok(enabled) => enabled,
                Err(error) => {
                    tracing::warn!(error = %error, "sync daemon config check failed");
                    continue;
                }
            };
            if !enabled {
                continue;
            }
            let reason = if dirty && !poll {
                "auto"
            } else if !first_poll_done {
                first_poll_done = true;
                "startup"
            } else {
                "poll"
            };
            match run_sync(&app, &state, reason).await {
                Ok(outcome) => {
                    if outcome.pending_restart {
                        drop(state);
                        crate::commands::webdav::relaunch_after_restore(app.clone());
                    }
                }
                Err(error) => {
                    tracing::warn!(error = %error, reason, "background sync failed");
                }
            }
        }
    });
}

/// 把远端 bundle 复制到 staging 并解析出它的指纹（供引擎与测试复用）。
pub fn parse_remote_manifest_bytes(bytes: &[u8]) -> AppResult<SyncManifest> {
    if bytes.len() as u64 > MAX_MANIFEST_BYTES {
        return Err(AppError::new(
            "sync_manifest_invalid",
            "sync manifest is too large",
        ));
    }
    let manifest: SyncManifest = serde_json::from_slice(bytes)
        .map_err(|error| AppError::new("sync_manifest_invalid", format!("invalid sync manifest: {error}")))?;
    manifest.validate()?;
    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fingerprint(db: &str, key: &str) -> DataFingerprint {
        DataFingerprint {
            database_sha256: sha64(db),
            master_key_sha256: sha64(key),
        }
    }

    fn sha64(seed: &str) -> String {
        format!("{:0>64}", seed)
    }

    fn manifest(db: &str, key: &str, revision: u64) -> SyncManifest {
        SyncManifest {
            format_version: SYNC_FORMAT_VERSION,
            revision,
            device_name: "work-pc".into(),
            updated_at: 1_000,
            bundle_file_name: "any-switch-backup-20260912_000000.work-pc.abcdef01.zip".into(),
            database_sha256: sha64(db),
            master_key_sha256: sha64(key),
            app_version: "0.0.0".into(),
        }
    }

    #[test]
    fn uploads_when_remote_is_empty() {
        let local = fingerprint("a", "f");
        let (action, conflict) = decide_action(&local, None, None);
        assert_eq!(action, SyncAction::Upload);
        assert!(!conflict);
    }

    #[test]
    fn in_sync_when_fingerprints_match() {
        let local = fingerprint("a", "f");
        let remote = manifest("a", "f", 3);
        let (action, conflict) = decide_action(&local, Some(&remote), Some(&local));
        assert_eq!(action, SyncAction::InSync);
        assert!(!conflict);
    }

    #[test]
    fn downloads_when_only_remote_changed() {
        let local = fingerprint("a", "f");
        let remote = manifest("b", "f", 4);
        let (action, conflict) = decide_action(&local, Some(&remote), Some(&local));
        assert_eq!(action, SyncAction::Download);
        assert!(!conflict);
    }

    #[test]
    fn uploads_when_only_local_changed() {
        let local = fingerprint("b", "f");
        let remote = manifest("a", "f", 4);
        let synced = fingerprint("a", "f");
        let (action, conflict) = decide_action(&local, Some(&remote), Some(&synced));
        assert_eq!(action, SyncAction::Upload);
        assert!(!conflict);
    }

    #[test]
    fn flags_conflict_when_both_sides_changed() {
        let local = fingerprint("c", "f");
        let remote = manifest("b", "f", 4);
        let synced = fingerprint("a", "f");
        let (action, conflict) = decide_action(&local, Some(&remote), Some(&synced));
        assert_eq!(action, SyncAction::Download);
        assert!(conflict);
    }

    #[test]
    fn downloads_on_first_contact_without_conflict() {
        let local = fingerprint("a", "f");
        let remote = manifest("b", "f", 7);
        let (action, conflict) = decide_action(&local, Some(&remote), None);
        assert_eq!(action, SyncAction::Download);
        assert!(!conflict);
    }

    #[test]
    fn manifest_round_trips_and_validates() {
        let remote = manifest("a", "f", 2);
        let bytes = serde_json::to_vec(&remote).unwrap();
        assert_eq!(parse_remote_manifest_bytes(&bytes).unwrap(), remote);
    }

    #[test]
    fn rejects_tampered_manifests() {
        let mut remote = manifest("a", "f", 2);
        remote.format_version = 99;
        let bytes = serde_json::to_vec(&remote).unwrap();
        assert!(parse_remote_manifest_bytes(&bytes).is_err());

        let remote = manifest("a", "f", 0);
        let bytes = serde_json::to_vec(&remote).unwrap();
        assert!(parse_remote_manifest_bytes(&bytes).is_err());

        let mut remote = manifest("short", "k", 2);
        let bytes = serde_json::to_vec(&remote).unwrap();
        assert!(parse_remote_manifest_bytes(&bytes).is_err());

        let mut remote = manifest("a", "f", 2);
        remote.bundle_file_name = "../escape.zip".into();
        let bytes = serde_json::to_vec(&remote).unwrap();
        assert!(parse_remote_manifest_bytes(&bytes).is_err());
    }

    #[test]
    fn fingerprint_equality_uses_both_hashes() {
        assert_eq!(fingerprint("a", "f"), fingerprint("a", "f"));
        assert_ne!(fingerprint("a", "f"), fingerprint("a", "f2"));
    }

    #[test]
    fn logical_fingerprint_is_stable_across_vacuum_and_detects_changes() {
        let source = tempfile::tempdir().unwrap();
        let db_path = source.path().join("data.db");
        let conn = Connection::open(&db_path).unwrap();
        crate::db::apply_schema(&conn).unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO settings (id, json) VALUES (1, '{\"a\":1}')",
            [],
        )
        .unwrap();
        let key = [7_u8; 32];
        let fingerprint = compute_logical_fingerprint(&conn, &key).unwrap();
        conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);").unwrap();

        // VACUUM 出字节布局不同的副本，逻辑指纹必须保持一致。
        let copy_path = source.path().join("copy.db");
        let copy_arg = copy_path.to_string_lossy().to_string();
        conn.execute("VACUUM INTO ?", [&copy_arg]).unwrap();
        let copy_fp = fingerprint_database_file(&copy_path, &key).unwrap();
        assert_eq!(fingerprint, copy_fp);

        // 逻辑内容变化必须改变指纹。
        conn.execute(
            "INSERT OR REPLACE INTO settings (id, json) VALUES (1, '{\"a\":2}')",
            [],
        )
        .unwrap();
        let changed = compute_logical_fingerprint(&conn, &key).unwrap();
        assert_ne!(fingerprint, changed);
    }

    #[test]
    fn only_business_tables_mark_dirty() {
        assert!(is_noisy_sync_table("sync_meta"));
        assert!(is_noisy_sync_table("webdav_sync_state"));
        assert!(!is_noisy_sync_table("sites"));
        assert!(!is_noisy_sync_table("settings"));
    }

    #[test]
    fn db_writes_mark_dirty_except_sync_tables() {
        SYNC_DIRTY.store(false, Ordering::Relaxed);
        SYNC_POLL_REQUESTED.store(false, Ordering::Relaxed);

        let conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::apply_schema(&conn).unwrap();
        install_db_hook(&conn);

        // 引擎记账表写入：不标记脏。
        repo::sync_meta::set_meta(&conn, "probe", "1").unwrap();
        assert!(!SYNC_DIRTY.load(Ordering::Relaxed));

        // 业务数据写入：标记脏。
        conn.execute(
            "INSERT OR REPLACE INTO settings (id, json) VALUES (1, '{}')",
            [],
        )
        .unwrap();
        assert!(SYNC_DIRTY.swap(false, Ordering::Relaxed));
        assert!(SYNC_POLL_REQUESTED.swap(false, Ordering::Relaxed) == false);
    }
}
