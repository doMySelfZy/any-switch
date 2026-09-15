use crate::app_backup;
use crate::domain::RestoreStartupResult;
use crate::error::{AppError, AppResult};
use crate::paths::{set_private_dir_permissions, set_secret_permissions};
use crate::sync::DataFingerprint;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const PENDING_NAME: &str = ".pending-restore";
const JOURNAL_NAME: &str = "apply-journal.json";
const COMMITTED_NAME: &str = "committed";
const PAYLOAD_NAME: &str = "payload";
/// 本次下载对应的远端指纹：随 pending 目录落盘，恢复真正应用成功后才由启动流程
/// 提交为同步记账（换库要等下次启动，排队时提交会留下"远端即共同祖先"的假账）。
const EXPECTED_SYNC_NAME: &str = "expected-sync.json";

#[derive(Debug, Serialize, Deserialize)]
struct ApplyJournal {
    target_existed: Vec<bool>,
}

#[derive(Debug)]
struct RestoreOperation {
    target: PathBuf,
    staging: Option<PathBuf>,
    rollback: PathBuf,
    target_existed: bool,
}

struct DirectoryCleanup {
    path: PathBuf,
    armed: bool,
}

impl Drop for DirectoryCleanup {
    fn drop(&mut self) {
        if self.armed {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestoreRelaunch {
    RestartInPlace,
    ExitForDevCli,
}

pub fn restore_relaunch_kind(debug_build: bool) -> RestoreRelaunch {
    if debug_build {
        RestoreRelaunch::ExitForDevCli
    } else {
        RestoreRelaunch::RestartInPlace
    }
}

pub fn cleanup_restore_staging_dirs(app_dir: &Path) -> AppResult<()> {
    let entries = match fs::read_dir(app_dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    for entry in entries {
        let entry = entry?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if entry.path().is_dir()
            && (name.starts_with(".webdav-restore-") || name.starts_with(".local-restore-"))
        {
            fs::remove_dir_all(entry.path())?;
        }
    }
    Ok(())
}

pub fn queue_pending_restore(
    archive_path: &Path,
    app_dir: &Path,
    expected: Option<DataFingerprint>,
) -> AppResult<()> {
    fs::create_dir_all(app_dir)?;
    let pending = app_dir.join(PENDING_NAME);
    if pending.exists() {
        return Err(AppError::new(
            "restore_pending",
            "another restore is already pending",
        ));
    }
    let staging = app_dir.join(format!(
        "{PENDING_NAME}.{}.migrating",
        uuid::Uuid::new_v4().simple()
    ));
    fs::create_dir(&staging)?;
    set_private_dir_permissions(&staging);
    let mut cleanup = DirectoryCleanup {
        path: staging.clone(),
        armed: true,
    };
    let payload = staging.join(PAYLOAD_NAME);
    let validated = app_backup::validate_and_extract_bundle(archive_path, &payload)?;
    if !validated.database_path.is_file() || !validated.master_key_path.is_file() {
        return Err(AppError::new(
            "backup_invalid",
            "validated restore payload is incomplete",
        ));
    }
    if let Some(fingerprint) = &expected {
        write_synced_json(&staging.join(EXPECTED_SYNC_NAME), fingerprint)?;
    }
    tracing::info!(
        source_device = %validated.manifest.device_name,
        source_created_at = validated.manifest.created_at,
        "validated application restore payload"
    );
    fs::rename(&staging, &pending)?;
    cleanup.armed = false;
    sync_dir(app_dir)?;
    Ok(())
}

/// 启动恢复结果 + 本次下载的同步目标指纹（仅进程内使用，不写入 restore-result.json）。
#[derive(Debug)]
pub struct RestoreStartupOutcome {
    pub result: RestoreStartupResult,
    /// 仅当恢复真正应用成功且排队时记录了预期指纹才为 Some；失败/回滚一律 None。
    pub synced_fingerprint: Option<DataFingerprint>,
}

pub fn apply_pending_restore(app_dir: &Path) -> AppResult<Option<RestoreStartupOutcome>> {
    if let Err(error) = cleanup_restore_staging_dirs(app_dir) {
        tracing::warn!(%error, "failed to clean leftover restore download directories");
    }
    let pending = app_dir.join(PENDING_NAME);
    if !pending.exists() {
        return Ok(None);
    }
    let result = apply_pending_restore_inner(app_dir, &pending);
    match result {
        Ok(applied) => {
            let outcome = RestoreStartupResult {
                status: "applied".into(),
                message: "Application data was restored successfully.".into(),
            };
            write_restore_result(app_dir, &outcome)?;
            Ok(Some(RestoreStartupOutcome {
                result: outcome,
                synced_fingerprint: applied,
            }))
        }
        Err(error) => {
            recover_uncommitted(app_dir, &pending)?;
            let quarantine = app_dir.join(format!(
                ".failed-restore-{}",
                chrono::Utc::now().timestamp_millis()
            ));
            if pending.exists() {
                fs::rename(&pending, &quarantine)?;
            }
            let outcome = RestoreStartupResult {
                status: "failed".into(),
                message: format!(
                    "Restore failed and the previous application data was kept: {error}. Quarantined payload: {}",
                    quarantine.display()
                ),
            };
            // 恢复失败：本地仍是旧数据，绝不能提交记账，否则下一轮会判定"只有本地变了"反向上传。
            write_restore_result(app_dir, &outcome)?;
            Ok(Some(RestoreStartupOutcome {
                result: outcome,
                synced_fingerprint: None,
            }))
        }
    }
}

/// 返回本次下载的目标指纹；无待恢复记录（手动恢复）时为 None。
fn apply_pending_restore_inner(
    app_dir: &Path,
    pending: &Path,
) -> AppResult<Option<DataFingerprint>> {
    let expected = read_expected_sync(pending);
    if pending.join(JOURNAL_NAME).exists() {
        // 上次启动已换库成功、只剩清理：`committed` 只在发布成功后写入，它的存在即代表
        // 数据已经落地，缺的只是记账。这里必须把预期指纹交出去提交——若返回 None，
        // last_synced 会停留在下载前的旧值，一旦恢复后的 schema 迁移改变了本地数据
        // （本地重算值 != 远端声明值），判定就会永远落入 Download 反复重下。
        if pending.join(COMMITTED_NAME).exists() {
            let committed = expected.clone();
            if let Err(error) = cleanup_committed(app_dir, pending) {
                tracing::warn!(%error, "restore was committed but cleanup is still pending");
            }
            return Ok(committed);
        }
        recover_uncommitted(app_dir, pending)?;
    }

    let payload = pending.join(PAYLOAD_NAME);
    let desired = desired_operations(app_dir, &payload);
    let mut operations = prepare_operations(desired)?;
    let journal = ApplyJournal {
        target_existed: operations.iter().map(|op| op.target_existed).collect(),
    };
    write_synced_json(&pending.join(JOURNAL_NAME), &journal)?;

    if let Err(error) = publish_operations(&mut operations) {
        rollback_operations(&operations)?;
        return Err(error);
    }
    write_synced_file(&pending.join(COMMITTED_NAME), b"")?;
    if let Err(error) = cleanup_committed(app_dir, pending) {
        tracing::warn!(%error, "restore was committed but cleanup is still pending");
    }
    Ok(expected)
}

/// 读取排队时记录的预期指纹。解析失败只当作"没有目标指纹"：
/// 记账文件损坏不能反过来阻断数据恢复，届时下一轮同步自愈。
fn read_expected_sync(pending: &Path) -> Option<DataFingerprint> {
    let path = pending.join(EXPECTED_SYNC_NAME);
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return None,
        Err(error) => {
            tracing::warn!(%error, "ignoring unreadable expected sync record");
            return None;
        }
    };
    match serde_json::from_slice(&bytes) {
        Ok(fingerprint) => Some(fingerprint),
        Err(error) => {
            tracing::warn!(%error, "ignoring unreadable expected sync record");
            None
        }
    }
}

fn desired_operations(app_dir: &Path, payload: &Path) -> Vec<(Option<PathBuf>, PathBuf)> {
    // 迁移复制失败回退旧目录时，活动库名仍是 `xiaobai-switch.db`：必须用同一解析结果
    // 处理主库与 `-wal/-shm/-journal` 旁文件，否则会留下孤儿 WAL。
    let db = crate::paths::resolve_db_file_name(app_dir);
    vec![
        (Some(payload.join("master.key")), app_dir.join("master.key")),
        (None, app_dir.join(format!("{db}-wal"))),
        (None, app_dir.join(format!("{db}-shm"))),
        (None, app_dir.join(format!("{db}-journal"))),
        (
            Some(payload.join(app_backup::BUNDLE_DATABASE_FILE_NAME)),
            app_dir.join(db),
        ),
    ]
}

fn prepare_operations(
    desired: Vec<(Option<PathBuf>, PathBuf)>,
) -> AppResult<Vec<RestoreOperation>> {
    let mut operations = Vec::new();
    for (index, (source, target)) in desired.into_iter().enumerate() {
        if source.as_ref().is_some_and(|path| !path.is_file()) {
            return Err(AppError::new(
                "backup_invalid",
                format!("restore payload is missing: {}", source.unwrap().display()),
            ));
        }
        let staging = source.as_ref().map(|_| {
            target.with_file_name(format!(
                ".{}.restore-{index}.new",
                target.file_name().unwrap_or_default().to_string_lossy()
            ))
        });
        let rollback = target.with_file_name(format!(
            ".{}.restore-{index}.rollback",
            target.file_name().unwrap_or_default().to_string_lossy()
        ));
        remove_file_if_exists(&rollback)?;
        if let (Some(source), Some(staging)) = (&source, &staging) {
            remove_file_if_exists(staging)?;
            copy_file_synced(source, staging)?;
            if target.file_name().and_then(|name| name.to_str()) == Some("master.key") {
                set_secret_permissions(staging);
            }
        }
        operations.push(RestoreOperation {
            target: target.clone(),
            staging,
            rollback,
            target_existed: target.is_file(),
        });
    }
    Ok(operations)
}

fn publish_operations(operations: &mut [RestoreOperation]) -> AppResult<()> {
    publish_operations_inner(operations, None)
}

fn publish_operations_inner(
    operations: &mut [RestoreOperation],
    fail_before_index: Option<usize>,
) -> AppResult<()> {
    for (index, operation) in operations.iter_mut().enumerate() {
        if fail_before_index == Some(index) {
            return Err(AppError::new(
                "internal",
                "injected restore publish failure",
            ));
        }
        if operation.target_existed {
            if !operation.target.is_file() {
                return Err(AppError::new(
                    "backup_invalid",
                    format!("restore target changed: {}", operation.target.display()),
                ));
            }
            fs::rename(&operation.target, &operation.rollback)?;
        } else if operation.target.exists() {
            return Err(AppError::new(
                "backup_invalid",
                format!(
                    "restore target is not a file: {}",
                    operation.target.display()
                ),
            ));
        }
        if let Some(staging) = &operation.staging {
            fs::rename(staging, &operation.target)?;
        }
        sync_dir(operation.target.parent().unwrap_or_else(|| Path::new(".")))?;
    }
    Ok(())
}

fn recover_uncommitted(app_dir: &Path, pending: &Path) -> AppResult<()> {
    let journal_path = pending.join(JOURNAL_NAME);
    if !journal_path.exists() {
        return Ok(());
    }
    let journal: ApplyJournal = serde_json::from_slice(&fs::read(&journal_path)?)
        .map_err(|e| AppError::new("backup_invalid", format!("restore journal is invalid: {e}")))?;
    let desired = desired_operations(app_dir, &pending.join(PAYLOAD_NAME));
    if journal.target_existed.len() != desired.len() {
        return Err(AppError::new(
            "backup_invalid",
            "restore journal does not match the payload",
        ));
    }
    let operations = desired
        .into_iter()
        .enumerate()
        .map(|(index, (source, target))| RestoreOperation {
            staging: source.as_ref().map(|_| {
                target.with_file_name(format!(
                    ".{}.restore-{index}.new",
                    target.file_name().unwrap_or_default().to_string_lossy()
                ))
            }),
            rollback: target.with_file_name(format!(
                ".{}.restore-{index}.rollback",
                target.file_name().unwrap_or_default().to_string_lossy()
            )),
            target_existed: journal.target_existed[index],
            target,
        })
        .collect::<Vec<_>>();
    rollback_operations(&operations)?;
    remove_file_if_exists(&journal_path)?;
    remove_file_if_exists(&pending.join(COMMITTED_NAME))?;
    sync_dir(pending)?;
    Ok(())
}

fn rollback_operations(operations: &[RestoreOperation]) -> AppResult<()> {
    for operation in operations.iter().rev() {
        if operation.rollback.exists() {
            remove_file_if_exists(&operation.target)?;
            fs::rename(&operation.rollback, &operation.target)?;
        } else if operation.target_existed {
            if !operation.target.exists() {
                return Err(AppError::new(
                    "internal",
                    format!(
                        "restore rollback is missing for {}",
                        operation.target.display()
                    ),
                ));
            }
        } else {
            remove_file_if_exists(&operation.target)?;
        }
        if let Some(staging) = &operation.staging {
            remove_file_if_exists(staging)?;
        }
    }
    Ok(())
}

fn cleanup_committed(app_dir: &Path, pending: &Path) -> AppResult<()> {
    for (index, (_, target)) in desired_operations(app_dir, &pending.join(PAYLOAD_NAME))
        .into_iter()
        .enumerate()
    {
        let rollback = target.with_file_name(format!(
            ".{}.restore-{index}.rollback",
            target.file_name().unwrap_or_default().to_string_lossy()
        ));
        let staging = target.with_file_name(format!(
            ".{}.restore-{index}.new",
            target.file_name().unwrap_or_default().to_string_lossy()
        ));
        remove_file_if_exists(&rollback)?;
        remove_file_if_exists(&staging)?;
    }
    if pending.exists() {
        fs::remove_dir_all(pending)?;
    }
    sync_dir(app_dir)
}

pub fn take_restore_result(app_dir: &Path) -> AppResult<Option<RestoreStartupResult>> {
    let path = app_dir.join("restore-result.json");
    if !path.exists() {
        return Ok(None);
    }
    let result = serde_json::from_slice(&fs::read(&path)?)?;
    fs::remove_file(path)?;
    Ok(Some(result))
}

fn write_restore_result(app_dir: &Path, result: &RestoreStartupResult) -> AppResult<()> {
    write_synced_json(&app_dir.join("restore-result.json"), result)
}

fn write_synced_json(path: &Path, value: &impl Serialize) -> AppResult<()> {
    write_synced_file(path, &serde_json::to_vec_pretty(value)?)
}

fn write_synced_file(path: &Path, bytes: &[u8]) -> AppResult<()> {
    let temp = path.with_extension("migrating");
    remove_file_if_exists(&temp)?;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)?;
    use std::io::Write;
    file.write_all(bytes)?;
    file.sync_all()?;
    fs::rename(temp, path)?;
    sync_dir(path.parent().unwrap_or_else(|| Path::new(".")))
}

fn copy_file_synced(source: &Path, target: &Path) -> AppResult<()> {
    fs::copy(source, target)?;
    fs::OpenOptions::new()
        .write(true)
        .open(target)?
        .sync_all()?;
    Ok(())
}

fn remove_file_if_exists(path: &Path) -> AppResult<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

#[cfg(unix)]
fn sync_dir(path: &Path) -> AppResult<()> {
    fs::File::open(path)?.sync_all()?;
    Ok(())
}

#[cfg(not(unix))]
fn sync_dir(_path: &Path) -> AppResult<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn queued_restore_is_applied_before_database_open() {
        let temp = tempfile::tempdir().unwrap();
        let source_dir = temp.path().join("source");
        let live_dir = temp.path().join("live");
        fs::create_dir_all(&source_dir).unwrap();
        fs::create_dir_all(&live_dir).unwrap();

        let source_db = source_dir.join("source.db");
        let source_conn = Connection::open(&source_db).unwrap();
        crate::db::apply_schema(&source_conn).unwrap();
        fs::write(source_dir.join("master.key"), [3_u8; 32]).unwrap();
        let bundle = app_backup::create_backup_in(
            &source_conn,
            &source_dir.join("master.key"),
            &source_dir,
            "manual",
        )
        .unwrap();

        fs::write(live_dir.join("xiaobai-switch.db"), b"old-db").unwrap();
        fs::write(live_dir.join("master.key"), [9_u8; 32]).unwrap();
        fs::write(live_dir.join("xiaobai-switch.db-wal"), b"old-wal").unwrap();
        fs::write(live_dir.join("xiaobai-switch.db-shm"), b"old-shm").unwrap();
        queue_pending_restore(&bundle.path, &live_dir, None).unwrap();
        let outcome = apply_pending_restore(&live_dir).unwrap().unwrap();
        assert_eq!(outcome.result.status, "applied");
        assert_eq!(outcome.synced_fingerprint, None);
        assert_eq!(
            fs::read(live_dir.join("master.key")).unwrap(),
            vec![3_u8; 32]
        );
        assert!(!live_dir.join("xiaobai-switch.db-wal").exists());
        assert!(!live_dir.join("xiaobai-switch.db-shm").exists());
        let restored = Connection::open(live_dir.join("xiaobai-switch.db")).unwrap();
        let quick_check: String = restored
            .query_row("PRAGMA quick_check", [], |row| row.get(0))
            .unwrap();
        assert_eq!(quick_check, "ok");
        assert_eq!(
            take_restore_result(&live_dir).unwrap().unwrap().status,
            "applied"
        );
        assert!(take_restore_result(&live_dir).unwrap().is_none());
    }

    #[test]
    fn injected_publish_failure_rolls_back_database_key_and_sidecars() {
        let temp = tempfile::tempdir().unwrap();
        let app_dir = temp.path().join("live");
        let payload = temp.path().join("payload");
        fs::create_dir_all(&app_dir).unwrap();
        fs::create_dir_all(&payload).unwrap();
        fs::write(app_dir.join("master.key"), b"old-key").unwrap();
        fs::write(app_dir.join("xiaobai-switch.db"), b"old-db").unwrap();
        fs::write(app_dir.join("xiaobai-switch.db-wal"), b"old-wal").unwrap();
        fs::write(payload.join("master.key"), b"new-key").unwrap();
        fs::write(
            payload.join(app_backup::BUNDLE_DATABASE_FILE_NAME),
            b"new-db",
        )
        .unwrap();

        let mut operations = prepare_operations(desired_operations(&app_dir, &payload)).unwrap();
        let error = publish_operations_inner(&mut operations, Some(2)).unwrap_err();
        assert!(error.to_string().contains("injected"));
        rollback_operations(&operations).unwrap();

        assert_eq!(fs::read(app_dir.join("master.key")).unwrap(), b"old-key");
        assert_eq!(
            fs::read(app_dir.join("xiaobai-switch.db")).unwrap(),
            b"old-db"
        );
        assert_eq!(
            fs::read(app_dir.join("xiaobai-switch.db-wal")).unwrap(),
            b"old-wal"
        );
        assert!(!app_dir.join("xiaobai-switch.db-shm").exists());
    }

    #[test]
    fn restore_targets_the_legacy_database_name_when_migration_fell_back() {
        let temp = tempfile::tempdir().unwrap();
        let app_dir = temp.path().join("live");
        let payload = temp.path().join("payload");
        fs::create_dir_all(&app_dir).unwrap();
        fs::create_dir_all(&payload).unwrap();
        // 回退到 AnySwitch 时期目录（库名仍是 any-switch.db）时，恢复必须命中它。
        fs::write(app_dir.join("any-switch.db"), b"old-db").unwrap();
        fs::write(app_dir.join("any-switch.db-wal"), b"old-wal").unwrap();
        fs::write(payload.join("master.key"), b"new-key").unwrap();
        fs::write(
            payload.join(app_backup::BUNDLE_DATABASE_FILE_NAME),
            b"new-db",
        )
        .unwrap();

        let operations = desired_operations(&app_dir, &payload);

        assert!(operations
            .iter()
            .any(|(_, target)| target == &app_dir.join("any-switch.db")));
        assert!(operations
            .iter()
            .any(|(_, target)| target == &app_dir.join("any-switch.db-wal")));
        assert!(!operations
            .iter()
            .any(|(_, target)| target == &app_dir.join("xiaobai-switch.db")));
    }

    #[test]
    fn debug_restore_exits_instead_of_reexecing_the_dev_binary() {
        assert_eq!(restore_relaunch_kind(true), RestoreRelaunch::ExitForDevCli);
        assert_eq!(
            restore_relaunch_kind(false),
            RestoreRelaunch::RestartInPlace
        );
    }

    #[test]
    fn queued_restore_does_not_need_the_source_archive_after_queue() {
        let temp = tempfile::tempdir().unwrap();
        let source_dir = temp.path().join("source");
        let live_dir = temp.path().join("live");
        fs::create_dir_all(&source_dir).unwrap();
        fs::create_dir_all(&live_dir).unwrap();

        let source_db = source_dir.join("source.db");
        let source_conn = Connection::open(&source_db).unwrap();
        crate::db::apply_schema(&source_conn).unwrap();
        fs::write(source_dir.join("master.key"), [3_u8; 32]).unwrap();
        let bundle = app_backup::create_backup_in(
            &source_conn,
            &source_dir.join("master.key"),
            &source_dir,
            "manual",
        )
        .unwrap();

        fs::write(live_dir.join("xiaobai-switch.db"), b"old-db").unwrap();
        fs::write(live_dir.join("master.key"), [9_u8; 32]).unwrap();
        queue_pending_restore(&bundle.path, &live_dir, None).unwrap();
        fs::remove_file(&bundle.path).unwrap();
        let outcome = apply_pending_restore(&live_dir).unwrap().unwrap();
        assert_eq!(outcome.result.status, "applied");
        assert_eq!(
            fs::read(live_dir.join("master.key")).unwrap(),
            vec![3_u8; 32]
        );
    }

    #[test]
    fn queued_restore_persists_the_expected_sync_fingerprint() {
        // 排队时写入的指纹必须与恢复落地一一对应：没有它，成功恢复后无法提交记账。
        let temp = tempfile::tempdir().unwrap();
        let source_dir = temp.path().join("source");
        let live_dir = temp.path().join("live");
        fs::create_dir_all(&source_dir).unwrap();
        fs::create_dir_all(&live_dir).unwrap();

        let source_db = source_dir.join("source.db");
        let source_conn = Connection::open(&source_db).unwrap();
        crate::db::apply_schema(&source_conn).unwrap();
        fs::write(source_dir.join("master.key"), [3_u8; 32]).unwrap();
        let bundle = app_backup::create_backup_in(
            &source_conn,
            &source_dir.join("master.key"),
            &source_dir,
            "manual",
        )
        .unwrap();

        fs::write(live_dir.join("xiaobai-switch.db"), b"old-db").unwrap();
        fs::write(live_dir.join("master.key"), [9_u8; 32]).unwrap();
        let expected = DataFingerprint {
            database_sha256: "e".repeat(64),
            master_key_sha256: "f".repeat(64),
        };
        queue_pending_restore(&bundle.path, &live_dir, Some(expected.clone())).unwrap();

        let outcome = apply_pending_restore(&live_dir).unwrap().unwrap();
        assert_eq!(outcome.result.status, "applied");
        assert_eq!(outcome.synced_fingerprint, Some(expected));
    }

    /// 构造一个最小可恢复的 pending 目录（成功应用一份已存在的库文件）。
    fn stage_pending(live_dir: &Path, expected: Option<DataFingerprint>) {
        let pending = live_dir.join(PENDING_NAME);
        let payload = pending.join(PAYLOAD_NAME);
        fs::create_dir_all(&payload).unwrap();
        fs::write(payload.join("master.key"), [3_u8; 32]).unwrap();
        fs::write(
            payload.join(app_backup::BUNDLE_DATABASE_FILE_NAME),
            b"new-db",
        )
        .unwrap();
        if let Some(fingerprint) = expected {
            write_synced_json(&pending.join(EXPECTED_SYNC_NAME), &fingerprint).unwrap();
        }
    }

    #[test]
    fn applied_restore_reports_the_expected_sync_fingerprint() {
        // 记账提交依赖这里回传的指纹：必须是排队时记录的远端声明值，
        // 而不是恢复后本地重算的结果（schema 迁移会改变后者）。
        let temp = tempfile::tempdir().unwrap();
        let live_dir = temp.path().join("live");
        fs::create_dir_all(&live_dir).unwrap();
        fs::write(live_dir.join("xiaobai-switch.db"), b"old-db").unwrap();
        fs::write(live_dir.join("master.key"), [9_u8; 32]).unwrap();
        let expected = DataFingerprint {
            database_sha256: "a".repeat(64),
            master_key_sha256: "b".repeat(64),
        };
        stage_pending(&live_dir, Some(expected.clone()));

        let outcome = apply_pending_restore(&live_dir).unwrap().unwrap();

        assert_eq!(outcome.result.status, "applied");
        assert_eq!(outcome.synced_fingerprint, Some(expected));
        // 成功后 pending 目录连同预期指纹文件一并清理，不会重复提交。
        assert!(!live_dir.join(PENDING_NAME).exists());
    }

    #[test]
    fn committed_restore_still_reports_the_expected_sync_fingerprint() {
        // 上次启动已换库、但没来得及提交记账（进程在写 committed 之后、提交之前退出）。
        // 数据已经落地，必须继续把指纹交出去提交：否则 last_synced 停在下载前的旧值，
        // 恢复后的 schema 迁移一旦改变本地数据，判定就会永远落入 Download 反复重下。
        let temp = tempfile::tempdir().unwrap();
        let live_dir = temp.path().join("live");
        fs::create_dir_all(&live_dir).unwrap();
        fs::write(live_dir.join("xiaobai-switch.db"), b"new-db").unwrap();
        fs::write(live_dir.join("master.key"), [3_u8; 32]).unwrap();
        let expected = DataFingerprint {
            database_sha256: "1".repeat(64),
            master_key_sha256: "2".repeat(64),
        };
        stage_pending(&live_dir, Some(expected.clone()));
        let pending = live_dir.join(PENDING_NAME);
        // journal 的长度按当前 `desired_operations` 派生，避免写死 5 之后被后续
        // 新增目标悄悄变成过期夹具（committed 分支不校验它，只有回滚路径才校验）。
        let target_count = desired_operations(&live_dir, &pending.join(PAYLOAD_NAME)).len();
        write_synced_json(
            &pending.join(JOURNAL_NAME),
            &ApplyJournal {
                target_existed: vec![true; target_count],
            },
        )
        .unwrap();
        write_synced_file(&pending.join(COMMITTED_NAME), b"").unwrap();

        let outcome = apply_pending_restore(&live_dir).unwrap().unwrap();

        assert_eq!(outcome.result.status, "applied");
        assert_eq!(outcome.synced_fingerprint, Some(expected));
        // 换库在上一轮已完成：这里只做清理与记账，不得再动数据。
        assert_eq!(
            fs::read(live_dir.join("xiaobai-switch.db")).unwrap(),
            b"new-db"
        );
        assert!(!pending.exists());
    }

    #[test]
    fn failed_restore_reports_no_sync_fingerprint() {
        // 恢复失败：本地仍是旧数据，绝不能回传指纹去提交记账，
        // 否则下一轮会判定"只有本地变了"并用旧数据覆盖云端较新的数据。
        let temp = tempfile::tempdir().unwrap();
        let live_dir = temp.path().join("live");
        fs::create_dir_all(&live_dir).unwrap();
        fs::write(live_dir.join("xiaobai-switch.db"), b"old-db").unwrap();
        let expected = DataFingerprint {
            database_sha256: "c".repeat(64),
            master_key_sha256: "d".repeat(64),
        };
        let pending = live_dir.join(PENDING_NAME);
        let payload = pending.join(PAYLOAD_NAME);
        fs::create_dir_all(&payload).unwrap();
        fs::write(payload.join("master.key"), [3_u8; 32]).unwrap();
        write_synced_json(&pending.join(EXPECTED_SYNC_NAME), &expected).unwrap();

        let outcome = apply_pending_restore(&live_dir).unwrap().unwrap();

        assert_eq!(outcome.result.status, "failed");
        assert_eq!(outcome.synced_fingerprint, None);
        assert_eq!(fs::read(live_dir.join("xiaobai-switch.db")).unwrap(), b"old-db");
        assert!(!pending.exists(), "failed payload must be quarantined");
    }

    #[test]
    fn leftover_restore_download_dirs_are_removed_on_startup() {
        let temp = tempfile::tempdir().unwrap();
        let app_dir = temp.path().join("live");
        fs::create_dir_all(&app_dir).unwrap();
        let webdav = app_dir.join(".webdav-restore-abc");
        let local = app_dir.join(".local-restore-xyz");
        fs::create_dir_all(&webdav).unwrap();
        fs::create_dir_all(&local).unwrap();
        fs::write(webdav.join("backup.zip"), b"zip").unwrap();
        fs::write(app_dir.join("keep-me"), b"ok").unwrap();

        cleanup_restore_staging_dirs(&app_dir).unwrap();

        assert!(!webdav.exists());
        assert!(!local.exists());
        assert!(app_dir.join("keep-me").exists());
    }
}
