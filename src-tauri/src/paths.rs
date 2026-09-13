use crate::error::{AppError, AppResult};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Once;

/// 当前应用数据目录名。
pub const APP_DIR_NAME: &str = ".any-switch";
/// 更名前的数据目录名，仅用于一次性迁移与回滚。
pub const LEGACY_APP_DIR_NAME: &str = ".xiaobai-switch";
/// 当前数据库文件名。
pub const DB_FILE_NAME: &str = "any-switch.db";
/// 更名前的数据库文件名，仅用于迁移识别。
pub const LEGACY_DB_FILE_NAME: &str = "xiaobai-switch.db";
const DB_SIDECAR_SUFFIXES: [&str; 3] = ["-wal", "-shm", "-journal"];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppPaths {
    pub app_dir: String,
    pub db_path: String,
    pub master_key_path: String,
    pub backups_dir: String,
    pub app_backups_dir: String,
    pub codex_env_path: String,
    pub logs_dir: String,
}

pub fn home_dir() -> AppResult<PathBuf> {
    dirs::home_dir().ok_or_else(|| AppError::new("internal", "cannot resolve home directory"))
}

/// 数据目录覆盖（测试/多实例隔离）：优先 `ANY_SWITCH_DATA_DIR`，
/// 兼容更名前的 `XIAOBAI_SWITCH_DATA_DIR`。
fn override_from(lookup: impl Fn(&str) -> Option<String>) -> Option<PathBuf> {
    for key in ["ANY_SWITCH_DATA_DIR", "XIAOBAI_SWITCH_DATA_DIR"] {
        if let Some(dir) = lookup(key) {
            let trimmed = dir.trim();
            if !trimmed.is_empty() {
                return Some(PathBuf::from(trimmed));
            }
        }
    }
    None
}

pub fn app_dir() -> AppResult<PathBuf> {
    if let Some(dir) = override_from(|key| std::env::var(key).ok()) {
        return Ok(dir);
    }
    let home = home_dir()?;
    resolve_app_dir(&home.join(APP_DIR_NAME), &home.join(LEGACY_APP_DIR_NAME))
}

/// 决定当前数据目录，并在首次启动时迁移旧目录。
///
/// 迁移采用 **复制**（绝不移动/删除旧目录）：`~/.xiaobai-switch` 会原样保留，
/// 作为用户回滚到旧版本时的数据来源。复制后校验数据库 sha256 与 `master.key`
/// 字节，任一失败则回退使用旧目录，应用仍可正常启动。
fn resolve_app_dir(new_dir: &Path, legacy_dir: &Path) -> AppResult<PathBuf> {
    let new_exists = new_dir.exists();
    let legacy_exists = legacy_dir.exists();

    if !new_exists && legacy_exists {
        match migrate_legacy_dir(legacy_dir, new_dir) {
            Ok(()) => {
                rename_legacy_database_files(new_dir);
                return Ok(new_dir.to_path_buf());
            }
            Err(error) => {
                tracing::warn!(
                    error = %error,
                    legacy = %legacy_dir.display(),
                    "failed to migrate legacy app data; falling back to the legacy directory"
                );
                return Ok(legacy_dir.to_path_buf());
            }
        }
    }

    if new_exists && legacy_exists {
        static WARN_ONCE: Once = Once::new();
        WARN_ONCE.call_once(|| {
            tracing::warn!(
                legacy = %legacy_dir.display(),
                "legacy app data directory is still present; using the new directory and leaving it untouched"
            );
        });
    }

    if new_exists {
        rename_legacy_database_files(new_dir);
    }
    Ok(new_dir.to_path_buf())
}

/// 复制旧目录到新目录并校验；保留旧目录。
///
/// 先复制到同级的唯一暂存目录，校验通过后再原子 rename 到 `new_dir`。这样进程若在
/// 复制中途被杀死，只会留下带 pid/uuid 后缀的暂存目录，下次启动会清理并重新迁移，
/// 不会把半份数据当成正式数据目录使用，也不会误提并发的半成品。
fn migrate_legacy_dir(legacy_dir: &Path, new_dir: &Path) -> AppResult<()> {
    if new_dir.exists() {
        return Err(AppError::new(
            "internal",
            "new app data directory already exists",
        ));
    }
    let prefix = migration_staging_prefix(new_dir);
    sweep_stale_migration_staging(new_dir, &prefix);
    let staging = new_dir.with_file_name(format!(
        "{prefix}-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    let copied = (|| -> AppResult<()> {
        fs::create_dir_all(&staging)?;
        copy_dir_contents(legacy_dir, &staging)?;
        verify_migrated_dir(legacy_dir, &staging)
    })();
    if let Err(error) = copied {
        let _ = fs::remove_dir_all(&staging);
        return Err(error);
    }
    if let Err(error) = fs::rename(&staging, new_dir) {
        let _ = fs::remove_dir_all(&staging);
        return Err(error.into());
    }
    Ok(())
}

fn migration_staging_prefix(new_dir: &Path) -> String {
    format!(
        "{}.migrating",
        new_dir
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("app-data")
    )
}

/// 清理上次被强杀留下的暂存目录（仅限本目标目录的 `.migrating*` 同名兄弟）。
fn sweep_stale_migration_staging(new_dir: &Path, prefix: &str) {
    let Some(parent) = new_dir.parent() else {
        return;
    };
    let Ok(entries) = fs::read_dir(parent) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if name.starts_with(prefix) && entry.path().is_dir() {
            let _ = fs::remove_dir_all(entry.path());
        }
    }
}

fn copy_dir_contents(from: &Path, to: &Path) -> AppResult<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let target = to.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_contents(&entry.path(), &target)?;
        } else if file_type.is_file() {
            fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

fn verify_migrated_dir(legacy_dir: &Path, new_dir: &Path) -> AppResult<()> {
    let mut migrated_database: Option<&str> = None;
    for name in [DB_FILE_NAME, LEGACY_DB_FILE_NAME] {
        let source = legacy_dir.join(name);
        if !source.is_file() {
            continue;
        }
        let target = new_dir.join(name);
        if !target.is_file() || sha256_file(&target)? != sha256_file(&source)? {
            return Err(AppError::new(
                "internal",
                format!("migrated database {name} failed checksum verification"),
            ));
        }
        // WAL 里可能有未 checkpoint 的已提交数据：旁文件也必须逐字节一致。
        for suffix in DB_SIDECAR_SUFFIXES {
            let source_sidecar = legacy_dir.join(format!("{name}{suffix}"));
            if !source_sidecar.is_file() {
                continue;
            }
            let target_sidecar = new_dir.join(format!("{name}{suffix}"));
            if !target_sidecar.is_file()
                || sha256_file(&target_sidecar)? != sha256_file(&source_sidecar)?
            {
                return Err(AppError::new(
                    "internal",
                    format!("migrated database sidecar {name}{suffix} failed checksum verification"),
                ));
            }
        }
        if migrated_database.is_none() {
            migrated_database = Some(name);
        }
    }
    // 字节一致不足以保证可读（例如源库在复制期间被 checkpoint）：真实打开一次。
    if let Some(name) = migrated_database {
        verify_database_opens(&new_dir.join(name))?;
    }
    let source_key = legacy_dir.join("master.key");
    if source_key.is_file() {
        let target_key = new_dir.join("master.key");
        let source_bytes = fs::read(&source_key)?;
        let target_bytes = fs::read(&target_key)?;
        if source_bytes.len() != 32 || target_bytes != source_bytes {
            return Err(AppError::new(
                "internal",
                "migrated master.key failed verification",
            ));
        }
    }
    Ok(())
}

/// 打开迁移后的数据库并做 `PRAGMA quick_check`，确保不是半份/损坏的副本。
fn verify_database_opens(path: &Path) -> AppResult<()> {
    let conn = rusqlite::Connection::open(path).map_err(|error| {
        AppError::new(
            "internal",
            format!("migrated database cannot be opened: {error}"),
        )
    })?;
    let check: String = conn
        .query_row("PRAGMA quick_check", [], |row| row.get(0))
        .map_err(|error| {
            AppError::new(
                "internal",
                format!("migrated database integrity check failed: {error}"),
            )
        })?;
    if check != "ok" {
        return Err(AppError::new(
            "internal",
            format!("migrated database integrity check failed: {check}"),
        ));
    }
    Ok(())
}

/// 新目录内仅有旧数据库文件名时重命名为新名字（含 WAL/SHM/JOURNAL 旁文件）。
fn rename_legacy_database_files(dir: &Path) {
    let new_db = dir.join(DB_FILE_NAME);
    if new_db.exists() {
        return;
    }
    let legacy_db = dir.join(LEGACY_DB_FILE_NAME);
    if !legacy_db.is_file() {
        return;
    }
    if let Err(error) = fs::rename(&legacy_db, &new_db) {
        tracing::warn!(
            error = %error,
            "failed to rename the legacy database file; keeping the legacy name"
        );
        return;
    }
    for suffix in DB_SIDECAR_SUFFIXES {
        let from = dir.join(format!("{LEGACY_DB_FILE_NAME}{suffix}"));
        if from.is_file() {
            let _ = fs::rename(from, dir.join(format!("{DB_FILE_NAME}{suffix}")));
        }
    }
}

pub fn ensure_app_dirs() -> AppResult<PathBuf> {
    let dir = app_dir()?;
    fs::create_dir_all(&dir)?;
    fs::create_dir_all(dir.join("backups"))?;
    fs::create_dir_all(dir.join("backups").join("app"))?;
    fs::create_dir_all(dir.join("env"))?;
    fs::create_dir_all(dir.join("locks"))?;
    fs::create_dir_all(dir.join("logs"))?;
    Ok(dir)
}

/// 解析目录内实际使用的数据库文件名：优先新名字，迁移回退时可能是旧名字。
pub(crate) fn resolve_db_file_name(dir: &Path) -> &'static str {
    if dir.join(DB_FILE_NAME).exists() {
        DB_FILE_NAME
    } else if dir.join(LEGACY_DB_FILE_NAME).exists() {
        // 复制迁移失败回退旧目录时，数据库仍是旧文件名。
        LEGACY_DB_FILE_NAME
    } else {
        DB_FILE_NAME
    }
}

pub fn db_path() -> AppResult<PathBuf> {
    let dir = app_dir()?;
    Ok(dir.join(resolve_db_file_name(&dir)))
}

pub fn master_key_path() -> AppResult<PathBuf> {
    Ok(app_dir()?.join("master.key"))
}

pub fn codex_env_path() -> AppResult<PathBuf> {
    Ok(app_dir()?.join("env").join("codex.env"))
}

pub fn backups_dir() -> AppResult<PathBuf> {
    Ok(app_dir()?.join("backups"))
}

pub fn app_backups_dir() -> AppResult<PathBuf> {
    Ok(backups_dir()?.join("app"))
}

pub fn locks_dir() -> AppResult<PathBuf> {
    Ok(app_dir()?.join("locks"))
}

pub fn default_claude_home() -> AppResult<PathBuf> {
    Ok(home_dir()?.join(".claude"))
}

pub fn default_codex_home() -> AppResult<PathBuf> {
    if let Ok(v) = std::env::var("CODEX_HOME") {
        if !v.trim().is_empty() {
            return Ok(PathBuf::from(v));
        }
    }
    Ok(home_dir()?.join(".codex"))
}

pub fn default_pi_agent_dir() -> AppResult<PathBuf> {
    if let Ok(v) = std::env::var("PI_CODING_AGENT_DIR") {
        if !v.trim().is_empty() {
            return Ok(PathBuf::from(v));
        }
    }
    Ok(home_dir()?.join(".pi").join("agent"))
}

pub fn default_prime_agent_dir() -> AppResult<PathBuf> {
    if let Ok(v) = std::env::var("PRIME_AGENT_CODING_AGENT_DIR") {
        if !v.trim().is_empty() {
            return Ok(PathBuf::from(v));
        }
    }
    Ok(home_dir()?.join(".prime").join("agent"))
}

pub fn resolve_claude_home(override_path: Option<&str>) -> AppResult<PathBuf> {
    if let Some(p) = override_path {
        if !p.trim().is_empty() {
            return Ok(PathBuf::from(p));
        }
    }
    default_claude_home()
}

pub fn resolve_codex_home(override_path: Option<&str>) -> AppResult<PathBuf> {
    if let Some(p) = override_path {
        if !p.trim().is_empty() {
            return Ok(PathBuf::from(p));
        }
    }
    default_codex_home()
}

pub fn resolve_pi_agent_dir(override_path: Option<&str>) -> AppResult<PathBuf> {
    if let Some(p) = override_path {
        if !p.trim().is_empty() {
            return Ok(PathBuf::from(p));
        }
    }
    default_pi_agent_dir()
}

pub fn resolve_prime_agent_dir(override_path: Option<&str>) -> AppResult<PathBuf> {
    if let Some(p) = override_path {
        if !p.trim().is_empty() {
            return Ok(PathBuf::from(p));
        }
    }
    default_prime_agent_dir()
}

pub fn app_paths_dto() -> AppResult<AppPaths> {
    let dir = app_dir()?;
    Ok(AppPaths {
        app_dir: dir.display().to_string(),
        db_path: db_path()?.display().to_string(),
        master_key_path: master_key_path()?.display().to_string(),
        backups_dir: backups_dir()?.display().to_string(),
        app_backups_dir: app_backups_dir()?.display().to_string(),
        codex_env_path: codex_env_path()?.display().to_string(),
        logs_dir: dir.join("logs").display().to_string(),
    })
}

fn sha256_file(path: &Path) -> AppResult<String> {
    use std::io::Read;
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}

#[cfg(unix)]
pub fn set_secret_permissions(path: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(meta) = fs::metadata(path) {
        let mut perms = meta.permissions();
        perms.set_mode(0o600);
        let _ = fs::set_permissions(path, perms);
    }
}

#[cfg(unix)]
pub fn set_private_dir_permissions(path: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(meta) = fs::metadata(path) {
        let mut perms = meta.permissions();
        perms.set_mode(0o700);
        let _ = fs::set_permissions(path, perms);
    }
}

#[cfg(not(unix))]
pub fn set_secret_permissions(_path: &std::path::Path) {}

#[cfg(not(unix))]
pub fn set_private_dir_permissions(_path: &std::path::Path) {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn write_file(path: &Path, bytes: &[u8]) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, bytes).unwrap();
    }

    /// 迁移校验会真实打开数据库做 `PRAGMA quick_check`，测试必须用真正的 SQLite 文件。
    fn write_sqlite_file(path: &Path) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let conn = rusqlite::Connection::open(path).unwrap();
        conn.execute_batch(
            "CREATE TABLE probe (id INTEGER PRIMARY KEY, note TEXT); \
             INSERT INTO probe (note) VALUES ('ok');",
        )
        .unwrap();
    }

    fn write_sidecar(path: &Path) {
        write_file(path, b"sidecar-bytes");
    }

    #[test]
    fn pi_override_beats_environment() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("PI_CODING_AGENT_DIR", "/tmp/pi-env");
        assert_eq!(
            resolve_pi_agent_dir(Some("/tmp/pi-setting")).unwrap(),
            PathBuf::from("/tmp/pi-setting")
        );
        assert_eq!(
            resolve_pi_agent_dir(None).unwrap(),
            PathBuf::from("/tmp/pi-env")
        );
        std::env::remove_var("PI_CODING_AGENT_DIR");
        assert!(default_pi_agent_dir().unwrap().ends_with(".pi/agent"));
    }

    #[test]
    fn prime_override_beats_environment() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("PRIME_AGENT_CODING_AGENT_DIR", "/tmp/prime-env");
        assert_eq!(
            resolve_prime_agent_dir(Some("/tmp/prime-setting")).unwrap(),
            PathBuf::from("/tmp/prime-setting")
        );
        assert_eq!(
            resolve_prime_agent_dir(None).unwrap(),
            PathBuf::from("/tmp/prime-env")
        );
        std::env::remove_var("PRIME_AGENT_CODING_AGENT_DIR");
        assert!(default_prime_agent_dir().unwrap().ends_with(".prime/agent"));
    }

    #[test]
    fn data_dir_override_prefers_the_new_variable() {
        let map = |key: &str| match key {
            "ANY_SWITCH_DATA_DIR" => Some("  /tmp/any  ".to_string()),
            "XIAOBAI_SWITCH_DATA_DIR" => Some("/tmp/legacy".to_string()),
            _ => None,
        };
        assert_eq!(
            override_from(map).unwrap(),
            PathBuf::from("/tmp/any"),
            "new variable must win and be trimmed"
        );

        let legacy_only = |key: &str| match key {
            "XIAOBAI_SWITCH_DATA_DIR" => Some("/tmp/legacy".to_string()),
            _ => None,
        };
        assert_eq!(
            override_from(legacy_only).unwrap(),
            PathBuf::from("/tmp/legacy")
        );

        assert!(override_from(|_| None).is_none());
        assert!(override_from(|_| Some("   ".to_string())).is_none());
    }

    #[test]
    fn migrates_legacy_directory_by_copy_and_keeps_legacy() {
        let temp = tempfile::tempdir().unwrap();
        let legacy = temp.path().join(LEGACY_APP_DIR_NAME);
        let new_dir = temp.path().join(APP_DIR_NAME);
        write_sqlite_file(&legacy.join(LEGACY_DB_FILE_NAME));
        write_file(&legacy.join("master.key"), &[7_u8; 32]);
        write_file(&legacy.join("backups/app/old.zip"), b"zip");
        let legacy_db_before = fs::read(legacy.join(LEGACY_DB_FILE_NAME)).unwrap();

        let resolved = resolve_app_dir(&new_dir, &legacy).unwrap();

        assert_eq!(resolved, new_dir);
        assert!(
            new_dir.join(DB_FILE_NAME).is_file(),
            "database must be migrated and renamed in the new directory"
        );
        assert!(!new_dir.join(LEGACY_DB_FILE_NAME).exists());
        assert_eq!(
            fs::read(new_dir.join("master.key")).unwrap(),
            vec![7_u8; 32]
        );
        assert_eq!(
            fs::read(new_dir.join("backups/app/old.zip")).unwrap(),
            b"zip"
        );

        // 旧目录作为回滚点原样保留。
        assert!(legacy.is_dir());
        assert_eq!(
            fs::read(legacy.join(LEGACY_DB_FILE_NAME)).unwrap(),
            legacy_db_before
        );
        assert_eq!(fs::read(legacy.join("master.key")).unwrap(), vec![7_u8; 32]);
        assert!(!new_dir.with_file_name(".any-switch.migrating").exists());
    }

    #[test]
    fn recovers_from_an_interrupted_migration_without_using_partial_data() {
        let temp = tempfile::tempdir().unwrap();
        let legacy = temp.path().join(LEGACY_APP_DIR_NAME);
        let new_dir = temp.path().join(APP_DIR_NAME);
        write_sqlite_file(&legacy.join(LEGACY_DB_FILE_NAME));
        write_file(&legacy.join("master.key"), &[7_u8; 32]);
        // 模拟上次迁移被强杀：残留一个只有半个文件的暂存目录（含带 pid/uuid 后缀的）。
        write_file(
            &temp
                .path()
                .join(format!("{APP_DIR_NAME}.migrating"))
                .join("half.tmp"),
            b"half",
        );
        write_file(
            &temp
                .path()
                .join(format!("{APP_DIR_NAME}.migrating-1234-deadbeef"))
                .join("half.tmp"),
            b"half",
        );

        let resolved = resolve_app_dir(&new_dir, &legacy).unwrap();

        assert_eq!(resolved, new_dir);
        assert!(new_dir.join(DB_FILE_NAME).is_file());
        assert!(!temp
            .path()
            .join(format!("{APP_DIR_NAME}.migrating"))
            .exists());
        assert!(!temp
            .path()
            .join(format!("{APP_DIR_NAME}.migrating-1234-deadbeef"))
            .exists());
        assert!(legacy.join(LEGACY_DB_FILE_NAME).exists());
    }

    #[test]
    fn copy_and_verify_migrated_dir_includes_wal_sidecars() {
        let temp = tempfile::tempdir().unwrap();
        let legacy = temp.path().join("legacy");
        let new_dir = temp.path().join("new");
        write_sqlite_file(&legacy.join(LEGACY_DB_FILE_NAME));
        write_sidecar(&legacy.join(format!("{LEGACY_DB_FILE_NAME}-journal")));
        write_sidecar(&legacy.join(format!("{LEGACY_DB_FILE_NAME}-shm")));
        write_file(&legacy.join("master.key"), &[7_u8; 32]);

        fs::create_dir_all(&new_dir).unwrap();
        copy_dir_contents(&legacy, &new_dir).unwrap();

        assert!(
            new_dir
                .join(format!("{LEGACY_DB_FILE_NAME}-journal"))
                .is_file(),
            "sidecar files must be copied, not only the main database"
        );
        assert!(new_dir
            .join(format!("{LEGACY_DB_FILE_NAME}-shm"))
            .is_file());
        assert!(verify_migrated_dir(&legacy, &new_dir).is_ok());
    }

    #[test]
    fn verify_migrated_dir_rejects_a_tampered_database() {
        let temp = tempfile::tempdir().unwrap();
        let legacy = temp.path().join("legacy");
        let new_dir = temp.path().join("new");
        write_file(&legacy.join(LEGACY_DB_FILE_NAME), b"legacy-bytes");
        write_file(&new_dir.join(LEGACY_DB_FILE_NAME), b"tampered-bytes");

        assert!(verify_migrated_dir(&legacy, &new_dir).is_err());
    }

    #[test]
    fn verify_migrated_dir_rejects_a_tampered_wal_sidecar() {
        let temp = tempfile::tempdir().unwrap();
        let legacy = temp.path().join("legacy");
        let new_dir = temp.path().join("new");
        write_sqlite_file(&legacy.join(LEGACY_DB_FILE_NAME));
        write_sidecar(&legacy.join(format!("{LEGACY_DB_FILE_NAME}-wal")));
        // 主库字节必须一致，才能确保失败来自 wal 旁文件校验。
        fs::create_dir_all(&new_dir).unwrap();
        fs::copy(
            legacy.join(LEGACY_DB_FILE_NAME),
            new_dir.join(LEGACY_DB_FILE_NAME),
        )
        .unwrap();
        write_file(
            &new_dir.join(format!("{LEGACY_DB_FILE_NAME}-wal")),
            b"tampered-sidecar",
        );

        assert!(verify_migrated_dir(&legacy, &new_dir).is_err());
    }

    #[test]
    fn verify_migrated_dir_rejects_a_database_that_cannot_be_opened() {
        let temp = tempfile::tempdir().unwrap();
        let legacy = temp.path().join("legacy");
        let new_dir = temp.path().join("new");
        // 字节一致（绕过 checksum），但不是合法 SQLite：必须被真实打开校验拦下。
        write_file(&legacy.join(LEGACY_DB_FILE_NAME), b"not a sqlite database");
        write_file(
            &new_dir.join(LEGACY_DB_FILE_NAME),
            b"not a sqlite database",
        );

        assert!(verify_migrated_dir(&legacy, &new_dir).is_err());
    }

    #[test]
    fn prefers_new_directory_and_leaves_both_sides_untouched() {
        let temp = tempfile::tempdir().unwrap();
        let legacy = temp.path().join(LEGACY_APP_DIR_NAME);
        let new_dir = temp.path().join(APP_DIR_NAME);
        write_file(&legacy.join(LEGACY_DB_FILE_NAME), b"legacy-db");
        write_file(&new_dir.join(DB_FILE_NAME), b"new-db");

        let resolved = resolve_app_dir(&new_dir, &legacy).unwrap();

        assert_eq!(resolved, new_dir);
        assert_eq!(fs::read(new_dir.join(DB_FILE_NAME)).unwrap(), b"new-db");
        assert_eq!(
            fs::read(legacy.join(LEGACY_DB_FILE_NAME)).unwrap(),
            b"legacy-db"
        );
    }

    #[test]
    fn renames_legacy_database_inside_existing_new_directory() {
        let temp = tempfile::tempdir().unwrap();
        let legacy = temp.path().join(LEGACY_APP_DIR_NAME);
        let new_dir = temp.path().join(APP_DIR_NAME);
        write_file(&new_dir.join(LEGACY_DB_FILE_NAME), b"db");
        write_file(&new_dir.join(format!("{LEGACY_DB_FILE_NAME}-wal")), b"wal");

        let resolved = resolve_app_dir(&new_dir, &legacy).unwrap();

        assert_eq!(resolved, new_dir);
        assert_eq!(fs::read(new_dir.join(DB_FILE_NAME)).unwrap(), b"db");
        assert_eq!(
            fs::read(new_dir.join(format!("{DB_FILE_NAME}-wal"))).unwrap(),
            b"wal"
        );
        assert!(!new_dir.join(LEGACY_DB_FILE_NAME).exists());
        assert!(!new_dir.join(format!("{LEGACY_DB_FILE_NAME}-wal")).exists());
    }

    #[test]
    fn falls_back_to_the_legacy_directory_when_copy_fails() {
        let temp = tempfile::tempdir().unwrap();
        let legacy = temp.path().join(LEGACY_APP_DIR_NAME);
        write_file(&legacy.join(LEGACY_DB_FILE_NAME), b"legacy-db");
        // `blocker` 是文件而不是目录：新目录位于其下层，create_dir_all 必然失败。
        let blocker = temp.path().join("blocker");
        fs::write(&blocker, b"x").unwrap();
        let new_dir = blocker.join(APP_DIR_NAME);

        let resolved = resolve_app_dir(&new_dir, &legacy).unwrap();

        assert_eq!(resolved, legacy);
        assert!(legacy.join(LEGACY_DB_FILE_NAME).exists());
    }

    #[test]
    fn db_path_uses_the_legacy_file_name_until_it_is_renamed() {
        let temp = tempfile::tempdir().unwrap();
        let dir = temp.path().join("data");
        write_file(&dir.join(LEGACY_DB_FILE_NAME), b"db");
        assert_eq!(resolve_db_file_name(&dir), LEGACY_DB_FILE_NAME);
        write_file(&dir.join(DB_FILE_NAME), b"db2");
        assert_eq!(resolve_db_file_name(&dir), DB_FILE_NAME);
        let empty = temp.path().join("empty");
        fs::create_dir_all(&empty).unwrap();
        assert_eq!(resolve_db_file_name(&empty), DB_FILE_NAME);
    }
}
