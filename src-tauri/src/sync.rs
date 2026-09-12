use crate::app_backup;
use crate::domain::SyncOutcome;
use crate::error::{AppError, AppResult};
use crate::repo;
use crate::state::AppState;
use crate::webdav::WebDavClient;
use serde::{Deserialize, Serialize};
use tauri::AppHandle;

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
    fn from_bundle_manifest(manifest: &app_backup::AppBackupManifest) -> Self {
        Self {
            database_sha256: manifest.database_sha256.clone(),
            master_key_sha256: manifest.master_key_sha256.clone(),
        }
    }
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
    let conflict = match last_synced {
        None => false,
        Some(last) => *last != *local && *last != remote_fp,
    };
    (SyncAction::Download, conflict)
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
    let local_manifest = app_backup::read_bundle_manifest(&local_bundle.path)?;
    let local_fp = DataFingerprint::from_bundle_manifest(&local_manifest);

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
                device_name: local_manifest.device_name.clone(),
                updated_at: chrono::Utc::now().timestamp_millis(),
                bundle_file_name: local_bundle.file_name.clone(),
                database_sha256: local_manifest.database_sha256.clone(),
                master_key_sha256: local_manifest.master_key_sha256.clone(),
                app_version: env!("CARGO_PKG_VERSION").into(),
            };
            client.upload_sync_manifest(&manifest).await?;
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
                warning: None,
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
            if validated.manifest.database_sha256 != remote.database_sha256
                || validated.manifest.master_key_sha256 != remote.master_key_sha256
            {
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
            database_sha256: db.into(),
            master_key_sha256: key.into(),
        }
    }

    fn manifest(db: &str, key: &str, revision: u64) -> SyncManifest {
        SyncManifest {
            format_version: SYNC_FORMAT_VERSION,
            revision,
            device_name: "work-pc".into(),
            updated_at: 1_000,
            bundle_file_name: "xiaobai-switch-backup-20260912_000000.work-pc.abcdef01.zip".into(),
            database_sha256: db.into(),
            master_key_sha256: key.into(),
            app_version: "0.0.0".into(),
        }
    }

    #[test]
    fn uploads_when_remote_is_empty() {
        let local = fingerprint("a", "k");
        let (action, conflict) = decide_action(&local, None, None);
        assert_eq!(action, SyncAction::Upload);
        assert!(!conflict);
    }

    #[test]
    fn in_sync_when_fingerprints_match() {
        let local = fingerprint("a", "k");
        let remote = manifest("a", "k", 3);
        let (action, conflict) = decide_action(&local, Some(&remote), Some(&local));
        assert_eq!(action, SyncAction::InSync);
        assert!(!conflict);
    }

    #[test]
    fn downloads_when_only_remote_changed() {
        let local = fingerprint("a", "k");
        let remote = manifest("b", "k", 4);
        let (action, conflict) = decide_action(&local, Some(&remote), Some(&local));
        assert_eq!(action, SyncAction::Download);
        assert!(!conflict);
    }

    #[test]
    fn uploads_when_only_local_changed() {
        let local = fingerprint("b", "k");
        let remote = manifest("a", "k", 4);
        let synced = fingerprint("a", "k");
        let (action, conflict) = decide_action(&local, Some(&remote), Some(&synced));
        assert_eq!(action, SyncAction::Upload);
        assert!(!conflict);
    }

    #[test]
    fn flags_conflict_when_both_sides_changed() {
        let local = fingerprint("c", "k");
        let remote = manifest("b", "k", 4);
        let synced = fingerprint("a", "k");
        let (action, conflict) = decide_action(&local, Some(&remote), Some(&synced));
        assert_eq!(action, SyncAction::Download);
        assert!(conflict);
    }

    #[test]
    fn downloads_on_first_contact_without_conflict() {
        let local = fingerprint("a", "k");
        let remote = manifest("b", "k", 7);
        let (action, conflict) = decide_action(&local, Some(&remote), None);
        assert_eq!(action, SyncAction::Download);
        assert!(!conflict);
    }

    #[test]
    fn manifest_round_trips_and_validates() {
        let remote = manifest("a", "k", 2);
        let bytes = serde_json::to_vec(&remote).unwrap();
        assert_eq!(parse_remote_manifest_bytes(&bytes).unwrap(), remote);
    }

    #[test]
    fn rejects_tampered_manifests() {
        let mut remote = manifest("a", "k", 2);
        remote.format_version = 99;
        let bytes = serde_json::to_vec(&remote).unwrap();
        assert!(parse_remote_manifest_bytes(&bytes).is_err());

        let mut remote = manifest("a", "k", 0);
        let bytes = serde_json::to_vec(&remote).unwrap();
        assert!(parse_remote_manifest_bytes(&bytes).is_err());

        let mut remote = manifest("short", "k", 2);
        let bytes = serde_json::to_vec(&remote).unwrap();
        assert!(parse_remote_manifest_bytes(&bytes).is_err());

        let mut remote = manifest("a", "k", 2);
        remote.bundle_file_name = "../escape.zip".into();
        let bytes = serde_json::to_vec(&remote).unwrap();
        assert!(parse_remote_manifest_bytes(&bytes).is_err());
    }

    #[test]
    fn fingerprint_equality_uses_both_hashes() {
        assert_eq!(fingerprint("a", "k"), fingerprint("a", "k"));
        assert_ne!(fingerprint("a", "k"), fingerprint("a", "k2"));
    }
}
