use crate::app_backup;
use crate::domain::RestoreStartupResult;
use crate::error::{AppError, AppResult};
use crate::paths::{set_private_dir_permissions, set_secret_permissions};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const PENDING_NAME: &str = ".pending-restore";
const JOURNAL_NAME: &str = "apply-journal.json";
const COMMITTED_NAME: &str = "committed";
const PAYLOAD_NAME: &str = "payload";

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

pub fn queue_pending_restore(archive_path: &Path, app_dir: &Path) -> AppResult<()> {
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

pub fn apply_pending_restore(app_dir: &Path) -> AppResult<Option<RestoreStartupResult>> {
    if let Err(error) = cleanup_restore_staging_dirs(app_dir) {
        tracing::warn!(%error, "failed to clean leftover restore download directories");
    }
    let pending = app_dir.join(PENDING_NAME);
    if !pending.exists() {
        return Ok(None);
    }
    let result = apply_pending_restore_inner(app_dir, &pending);
    match result {
        Ok(()) => {
            let outcome = RestoreStartupResult {
                status: "applied".into(),
                message: "Application data was restored successfully.".into(),
            };
            write_restore_result(app_dir, &outcome)?;
            Ok(Some(outcome))
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
            write_restore_result(app_dir, &outcome)?;
            Ok(Some(outcome))
        }
    }
}

fn apply_pending_restore_inner(app_dir: &Path, pending: &Path) -> AppResult<()> {
    if pending.join(JOURNAL_NAME).exists() {
        if pending.join(COMMITTED_NAME).exists() {
            if let Err(error) = cleanup_committed(app_dir, pending) {
                tracing::warn!(%error, "restore was committed but cleanup is still pending");
            }
            return Ok(());
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
    Ok(())
}

fn desired_operations(app_dir: &Path, payload: &Path) -> Vec<(Option<PathBuf>, PathBuf)> {
    vec![
        (Some(payload.join("master.key")), app_dir.join("master.key")),
        (None, app_dir.join("xiaobai-switch.db-wal")),
        (None, app_dir.join("xiaobai-switch.db-shm")),
        (None, app_dir.join("xiaobai-switch.db-journal")),
        (
            Some(payload.join("xiaobai-switch.db")),
            app_dir.join("xiaobai-switch.db"),
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
        queue_pending_restore(&bundle.path, &live_dir).unwrap();
        let result = apply_pending_restore(&live_dir).unwrap().unwrap();
        assert_eq!(result.status, "applied");
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
        fs::write(payload.join("xiaobai-switch.db"), b"new-db").unwrap();

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
        queue_pending_restore(&bundle.path, &live_dir).unwrap();
        fs::remove_file(&bundle.path).unwrap();
        let result = apply_pending_restore(&live_dir).unwrap().unwrap();
        assert_eq!(result.status, "applied");
        assert_eq!(
            fs::read(live_dir.join("master.key")).unwrap(),
            vec![3_u8; 32]
        );
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
