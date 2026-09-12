use crate::crypto::Crypto;
use crate::domain::LocalBackupInfo;
use crate::error::{AppError, AppResult};
use crate::paths::{
    app_backups_dir, master_key_path, set_private_dir_permissions, set_secret_permissions,
};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use zip::write::SimpleFileOptions;

pub const BACKUP_PREFIX: &str = "xiaobai-switch-backup-";
pub const BACKUP_SUFFIX: &str = ".zip";
const FORMAT_VERSION: u32 = 1;
const MAX_ARCHIVE_BYTES: u64 = 512 * 1024 * 1024;
const MAX_MANIFEST_BYTES: u64 = 64 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppBackupManifest {
    pub format_version: u32,
    pub app_version: String,
    pub created_at: i64,
    pub device_name: String,
    pub reason: String,
    pub database_size: u64,
    pub database_sha256: String,
    pub master_key_size: u64,
    pub master_key_sha256: String,
}

#[derive(Debug, Clone)]
pub struct CreatedAppBackup {
    pub file_name: String,
    pub path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct ValidatedAppBackup {
    pub database_path: PathBuf,
    pub master_key_path: PathBuf,
    pub manifest: AppBackupManifest,
}

pub fn create_local_backup(
    conn: &Connection,
    reason: &str,
    max_copies: u32,
) -> AppResult<CreatedAppBackup> {
    let dir = app_backups_dir()?;
    let created = create_backup_in(conn, &master_key_path()?, &dir, reason)?;
    prune_backups_in(&dir, max_copies)?;
    Ok(created)
}

pub fn create_backup_in(
    conn: &Connection,
    key_path: &Path,
    destination_dir: &Path,
    reason: &str,
) -> AppResult<CreatedAppBackup> {
    fs::create_dir_all(destination_dir)?;
    set_private_dir_permissions(destination_dir);
    let temp_dir = tempfile::Builder::new()
        .prefix(".snapshot-")
        .tempdir_in(destination_dir)
        .map_err(|e| AppError::new("backup_failed", e.to_string()))?;
    let snapshot_path = temp_dir.path().join("xiaobai-switch.db");
    let escaped = snapshot_path.to_string_lossy().replace('\'', "''");
    conn.execute_batch(&format!("VACUUM INTO '{escaped}'"))
        .map_err(|e| AppError::new("backup_failed", format!("database snapshot failed: {e}")))?;

    let key_bytes = fs::read(key_path)
        .map_err(|e| AppError::new("backup_failed", format!("cannot read master.key: {e}")))?;
    if key_bytes.len() != 32 {
        return Err(AppError::new(
            "backup_failed",
            "master.key must contain exactly 32 bytes",
        ));
    }

    let created_at = chrono::Utc::now().timestamp_millis();
    let device_name = device_name();
    let file_name = format!(
        "{BACKUP_PREFIX}{}.{}.{}{}",
        chrono::Utc::now().format("%Y%m%d_%H%M%S"),
        device_name,
        &uuid::Uuid::new_v4().simple().to_string()[..8],
        BACKUP_SUFFIX
    );
    let destination = destination_dir.join(&file_name);
    let staging = destination_dir.join(format!(".{file_name}.migrating"));
    let manifest = AppBackupManifest {
        format_version: FORMAT_VERSION,
        app_version: env!("CARGO_PKG_VERSION").into(),
        created_at,
        device_name,
        reason: reason.into(),
        database_size: fs::metadata(&snapshot_path)?.len(),
        database_sha256: sha256_file(&snapshot_path)?,
        master_key_size: key_bytes.len() as u64,
        master_key_sha256: sha256_bytes(&key_bytes),
    };
    let write_result = write_bundle(&staging, &snapshot_path, &key_bytes, &manifest);
    if let Err(error) = write_result {
        let _ = fs::remove_file(&staging);
        return Err(error);
    }
    set_secret_permissions(&staging);
    fs::rename(&staging, &destination)?;
    sync_dir(destination_dir)?;
    Ok(CreatedAppBackup {
        file_name,
        path: destination,
    })
}

fn write_bundle(
    destination: &Path,
    database_path: &Path,
    master_key: &[u8],
    manifest: &AppBackupManifest,
) -> AppResult<()> {
    let file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)
        .map_err(|e| AppError::new("backup_failed", e.to_string()))?;
    let mut zip = zip::ZipWriter::new(file);
    let compressed =
        SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let stored = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

    zip.start_file("xiaobai-switch.db", compressed)
        .map_err(zip_error)?;
    let mut database = fs::File::open(database_path)?;
    std::io::copy(&mut database, &mut zip).map_err(|e| {
        AppError::new(
            "backup_failed",
            format!("database archive write failed: {e}"),
        )
    })?;

    zip.start_file("master.key", stored).map_err(zip_error)?;
    zip.write_all(master_key)?;

    zip.start_file("manifest.json", stored).map_err(zip_error)?;
    let manifest_json = serde_json::to_vec_pretty(manifest)?;
    zip.write_all(&manifest_json)?;
    zip.finish().map_err(zip_error)?.sync_all()?;
    Ok(())
}

pub fn validate_and_extract_bundle(
    archive_path: &Path,
    destination_dir: &Path,
) -> AppResult<ValidatedAppBackup> {
    let archive_size = fs::metadata(archive_path)?.len();
    if archive_size > MAX_ARCHIVE_BYTES {
        return Err(AppError::new(
            "backup_invalid",
            "backup archive is too large",
        ));
    }
    fs::create_dir_all(destination_dir)?;
    set_private_dir_permissions(destination_dir);
    let file = fs::File::open(archive_path)?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| AppError::new("backup_invalid", format!("invalid ZIP archive: {e}")))?;
    let allowed = ["xiaobai-switch.db", "master.key", "manifest.json"];
    let mut seen = HashSet::new();

    for index in 0..archive.len() {
        let entry = archive.by_index(index).map_err(zip_invalid)?;
        let name = entry.name().to_string();
        if entry.is_dir() || !allowed.contains(&name.as_str()) || !seen.insert(name.clone()) {
            return Err(AppError::new(
                "backup_invalid",
                format!("unsupported or duplicate ZIP entry: {name}"),
            ));
        }
        let max_size = if name == "manifest.json" {
            MAX_MANIFEST_BYTES
        } else if name == "master.key" {
            32
        } else {
            MAX_ARCHIVE_BYTES
        };
        if entry.size() > max_size {
            return Err(AppError::new(
                "backup_invalid",
                format!("backup entry is too large: {name}"),
            ));
        }
        let destination = destination_dir.join(&name);
        let mut output = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&destination)?;
        let copied = std::io::copy(&mut entry.take(max_size + 1), &mut output)?;
        if copied > max_size {
            return Err(AppError::new(
                "backup_invalid",
                format!("backup entry is too large: {name}"),
            ));
        }
        output.sync_all()?;
        if name == "master.key" {
            set_secret_permissions(&destination);
        }
    }
    if seen.len() != allowed.len() {
        return Err(AppError::new(
            "backup_invalid",
            "backup must contain database, master.key, and manifest.json",
        ));
    }

    let database_path = destination_dir.join("xiaobai-switch.db");
    let key_path = destination_dir.join("master.key");
    let manifest: AppBackupManifest =
        serde_json::from_slice(&fs::read(destination_dir.join("manifest.json"))?)
            .map_err(|e| AppError::new("backup_invalid", format!("invalid manifest: {e}")))?;
    if manifest.format_version != FORMAT_VERSION {
        return Err(AppError::new(
            "backup_invalid",
            format!("unsupported backup format: {}", manifest.format_version),
        ));
    }
    if fs::metadata(&database_path)?.len() != manifest.database_size
        || sha256_file(&database_path)? != manifest.database_sha256
    {
        return Err(AppError::new(
            "backup_invalid",
            "database checksum does not match the manifest",
        ));
    }
    let key_bytes = fs::read(&key_path)?;
    if key_bytes.len() != 32
        || key_bytes.len() as u64 != manifest.master_key_size
        || sha256_bytes(&key_bytes) != manifest.master_key_sha256
    {
        return Err(AppError::new(
            "backup_invalid",
            "master.key checksum does not match the manifest",
        ));
    }
    validate_restored_database(&database_path, &key_bytes)?;
    Ok(ValidatedAppBackup {
        database_path,
        master_key_path: key_path,
        manifest,
    })
}

fn validate_restored_database(database_path: &Path, key: &[u8]) -> AppResult<()> {
    let conn = Connection::open(database_path)
        .map_err(|e| AppError::new("backup_invalid", format!("cannot open database: {e}")))?;
    let quick_check: String = conn
        .query_row("PRAGMA quick_check", [], |row| row.get(0))
        .map_err(|e| AppError::new("backup_invalid", format!("database check failed: {e}")))?;
    if quick_check != "ok" {
        return Err(AppError::new(
            "backup_invalid",
            format!("database integrity check failed: {quick_check}"),
        ));
    }
    let crypto = Crypto::from_restore_key(key)?;
    crate::db::apply_schema_with_crypto(&conn, Some(&crypto), crate::db::migrate::BackupMode::Skip)
        .map_err(|e| AppError::new("backup_invalid", e.to_string()))?;
    crate::db::verify_encrypted_payloads(&conn, &crypto).map_err(|_| {
        AppError::new(
            "backup_invalid",
            "backup database and master.key do not belong together",
        )
    })?;
    Ok(())
}

pub fn latest_local_backup_at() -> AppResult<Option<i64>> {
    let dir = app_backups_dir()?;
    let mut latest: Option<i64> = None;
    if !dir.exists() {
        return Ok(None);
    }
    for entry in fs::read_dir(dir)?.flatten() {
        let path = entry.path();
        if !is_backup_file(&path) {
            continue;
        }
        let modified = entry
            .metadata()
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as i64);
        latest = match (latest, modified) {
            (Some(current), Some(candidate)) => Some(current.max(candidate)),
            (None, candidate) => candidate,
            (current, None) => current,
        };
    }
    Ok(latest)
}

pub fn list_local_backups() -> AppResult<Vec<LocalBackupInfo>> {
    list_local_backups_in(&app_backups_dir()?)
}

fn list_local_backups_in(dir: &Path) -> AppResult<Vec<LocalBackupInfo>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut backups = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        if !file_type.is_file() || !is_backup_file(&path) {
            continue;
        }
        let metadata = entry.metadata()?;
        let file_name = entry.file_name().to_string_lossy().into_owned();
        let modified_at = metadata
            .modified()?
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| AppError::new("backup_invalid", "backup timestamp predates Unix epoch"))?
            .as_millis() as i64;
        let (created_at, device_name, reason, app_version, error) =
            match read_manifest_from_bundle(&path) {
                Ok(manifest) => (
                    manifest.created_at,
                    manifest.device_name,
                    Some(manifest.reason),
                    Some(manifest.app_version),
                    None,
                ),
                Err(error) => (
                    modified_at,
                    parse_device_from_filename(&file_name),
                    None,
                    None,
                    Some(error.to_string()),
                ),
            };
        backups.push(LocalBackupInfo {
            file_name,
            size: metadata.len(),
            created_at,
            device_name,
            reason,
            app_version,
            error,
        });
    }
    backups.sort_by(|left, right| {
        right
            .created_at
            .cmp(&left.created_at)
            .then_with(|| right.file_name.cmp(&left.file_name))
    });
    Ok(backups)
}

pub fn delete_local_backup(file_name: &str) -> AppResult<()> {
    delete_local_backup_in(&app_backups_dir()?, file_name)
}

fn delete_local_backup_in(dir: &Path, file_name: &str) -> AppResult<()> {
    let path = resolve_local_backup_in(dir, file_name)?;
    fs::remove_file(path)?;
    Ok(())
}

pub fn stage_local_backup(file_name: &str, destination: &Path) -> AppResult<()> {
    stage_local_backup_in(&app_backups_dir()?, file_name, destination)
}

fn stage_local_backup_in(dir: &Path, file_name: &str, destination: &Path) -> AppResult<()> {
    let source = resolve_local_backup_in(dir, file_name)?;
    if fs::metadata(&source)?.len() > MAX_ARCHIVE_BYTES {
        return Err(AppError::new(
            "backup_invalid",
            "backup archive is too large",
        ));
    }
    let mut input = fs::File::open(source)?;
    let mut output = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)?;
    std::io::copy(&mut input, &mut output)?;
    output.sync_all()?;
    set_secret_permissions(destination);
    Ok(())
}

fn resolve_local_backup_in(dir: &Path, file_name: &str) -> AppResult<PathBuf> {
    let name = Path::new(file_name);
    if name.file_name().and_then(|value| value.to_str()) != Some(file_name)
        || !file_name.starts_with(BACKUP_PREFIX)
        || !file_name.ends_with(BACKUP_SUFFIX)
    {
        return Err(AppError::new(
            "backup_invalid",
            "invalid local backup file name",
        ));
    }
    let path = dir.join(file_name);
    let metadata = fs::symlink_metadata(&path)
        .map_err(|_| AppError::new("not_found", "local backup not found"))?;
    if !metadata.file_type().is_file() {
        return Err(AppError::new(
            "backup_invalid",
            "local backup must be a regular file",
        ));
    }
    Ok(path)
}

/// 读取 bundle 内嵌 manifest（供同步引擎计算数据指纹）。
pub fn read_bundle_manifest(path: &Path) -> AppResult<AppBackupManifest> {
    read_manifest_from_bundle(path)
}

fn read_manifest_from_bundle(path: &Path) -> AppResult<AppBackupManifest> {
    let file = fs::File::open(path)?;
    let mut archive = zip::ZipArchive::new(file).map_err(|error| {
        AppError::new("backup_invalid", format!("invalid ZIP archive: {error}"))
    })?;
    let entry = archive.by_name("manifest.json").map_err(|error| {
        AppError::new("backup_invalid", format!("manifest is missing: {error}"))
    })?;
    if entry.size() > MAX_MANIFEST_BYTES {
        return Err(AppError::new(
            "backup_invalid",
            "backup manifest is too large",
        ));
    }
    let mut bytes = Vec::with_capacity(entry.size() as usize);
    let copied = entry.take(MAX_MANIFEST_BYTES + 1).read_to_end(&mut bytes)?;
    if copied as u64 > MAX_MANIFEST_BYTES {
        return Err(AppError::new(
            "backup_invalid",
            "backup manifest is too large",
        ));
    }
    let manifest: AppBackupManifest = serde_json::from_slice(&bytes)
        .map_err(|error| AppError::new("backup_invalid", format!("invalid manifest: {error}")))?;
    if manifest.format_version != FORMAT_VERSION {
        return Err(AppError::new(
            "backup_invalid",
            format!("unsupported backup format: {}", manifest.format_version),
        ));
    }
    Ok(manifest)
}

pub fn prune_backups_in(dir: &Path, max_copies: u32) -> AppResult<usize> {
    if !dir.exists() {
        return Ok(0);
    }
    let max_copies = crate::domain::clamp_max_backup_copies(max_copies) as usize;
    let mut backups = fs::read_dir(dir)?
        .flatten()
        .filter(|entry| is_backup_file(&entry.path()))
        .collect::<Vec<_>>();
    backups.sort_by_key(|entry| std::cmp::Reverse(entry.file_name()));
    let mut removed = 0;
    for entry in backups.into_iter().skip(max_copies) {
        fs::remove_file(entry.path())?;
        removed += 1;
    }
    Ok(removed)
}

pub fn is_backup_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with(BACKUP_PREFIX) && name.ends_with(BACKUP_SUFFIX))
}

pub fn parse_device_from_filename(file_name: &str) -> String {
    let Some(rest) = file_name
        .strip_prefix(BACKUP_PREFIX)
        .and_then(|name| name.strip_suffix(BACKUP_SUFFIX))
    else {
        return "unknown".into();
    };
    let mut parts = rest.rsplitn(3, '.');
    let _random = parts.next();
    parts.next().unwrap_or("unknown").to_string()
}

fn device_name() -> String {
    let raw = std::env::var("HOSTNAME")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            std::process::Command::new("hostname")
                .output()
                .ok()
                .filter(|output| output.status.success())
                .and_then(|output| String::from_utf8(output.stdout).ok())
        })
        .unwrap_or_else(|| "unknown".into());
    let sanitized = raw
        .trim()
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'))
        .take(48)
        .collect::<String>();
    if sanitized.is_empty() {
        "unknown".into()
    } else {
        sanitized
    }
}

fn sha256_file(path: &Path) -> AppResult<String> {
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

fn sha256_bytes(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn zip_error(error: zip::result::ZipError) -> AppError {
    AppError::new("backup_failed", format!("ZIP error: {error}"))
}

fn zip_invalid(error: zip::result::ZipError) -> AppError {
    AppError::new("backup_invalid", format!("ZIP error: {error}"))
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
    use std::collections::HashMap;

    fn fixture() -> (tempfile::TempDir, Connection, PathBuf) {
        let temp = tempfile::tempdir().unwrap();
        let db_path = temp.path().join("source.db");
        let conn = Connection::open(&db_path).unwrap();
        crate::db::apply_schema(&conn).unwrap();
        let key_path = temp.path().join("master.key");
        fs::write(&key_path, [7_u8; 32]).unwrap();
        (temp, conn, key_path)
    }

    fn rewrite_bundle(
        source: &Path,
        destination: &Path,
        replacement_database: Option<&[u8]>,
        replacement_key: Option<&[u8]>,
        update_manifest: bool,
    ) {
        let mut source_zip = zip::ZipArchive::new(fs::File::open(source).unwrap()).unwrap();
        let mut entries = HashMap::new();
        for index in 0..source_zip.len() {
            let mut entry = source_zip.by_index(index).unwrap();
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes).unwrap();
            entries.insert(entry.name().to_string(), bytes);
        }
        if let Some(database) = replacement_database {
            entries.insert("xiaobai-switch.db".into(), database.to_vec());
        }
        if let Some(key) = replacement_key {
            entries.insert("master.key".into(), key.to_vec());
        }
        if update_manifest {
            let mut manifest: AppBackupManifest =
                serde_json::from_slice(&entries["manifest.json"]).unwrap();
            manifest.database_size = entries["xiaobai-switch.db"].len() as u64;
            manifest.database_sha256 = sha256_bytes(&entries["xiaobai-switch.db"]);
            manifest.master_key_size = entries["master.key"].len() as u64;
            manifest.master_key_sha256 = sha256_bytes(&entries["master.key"]);
            entries.insert(
                "manifest.json".into(),
                serde_json::to_vec(&manifest).unwrap(),
            );
        }
        let mut output = zip::ZipWriter::new(fs::File::create(destination).unwrap());
        for name in ["xiaobai-switch.db", "master.key", "manifest.json"] {
            output
                .start_file(name, SimpleFileOptions::default())
                .unwrap();
            output.write_all(&entries[name]).unwrap();
        }
        output.finish().unwrap();
    }

    #[test]
    fn creates_and_validates_database_key_bundle() {
        let (temp, conn, key_path) = fixture();
        conn.execute(
            "INSERT OR REPLACE INTO settings (id, json) VALUES (1, ?1)",
            [r#"{"language":"zh-CN"}"#],
        )
        .unwrap();
        let output = temp.path().join("out");
        let created = create_backup_in(&conn, &key_path, &output, "manual").unwrap();
        let extracted = temp.path().join("extracted");
        let validated = validate_and_extract_bundle(&created.path, &extracted).unwrap();
        assert_eq!(validated.manifest.reason, "manual");
        assert_eq!(fs::read(validated.master_key_path).unwrap(), vec![7_u8; 32]);
        let restored = Connection::open(validated.database_path).unwrap();
        let quick_check: String = restored
            .query_row("PRAGMA quick_check", [], |row| row.get(0))
            .unwrap();
        assert_eq!(quick_check, "ok");
        let settings_json: String = restored
            .query_row("SELECT json FROM settings WHERE id = 1", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(settings_json, r#"{"language":"zh-CN"}"#);
    }

    #[test]
    fn rejects_master_key_tampering() {
        let (temp, conn, key_path) = fixture();
        let output = temp.path().join("out");
        let created = create_backup_in(&conn, &key_path, &output, "manual").unwrap();
        let tampered = temp.path().join("tampered.zip");
        rewrite_bundle(&created.path, &tampered, None, Some(&[8_u8; 32]), false);
        assert!(validate_and_extract_bundle(&tampered, &temp.path().join("rejected")).is_err());
    }

    #[test]
    fn rejects_database_and_master_key_from_different_backups() {
        let (temp, conn, key_path) = fixture();
        let crypto = Crypto::from_restore_key(&[7_u8; 32]).unwrap();
        conn.execute(
            "INSERT INTO sites (
               id, name, base_url, protocol, claude_auth_key_style, notes, enabled, sort_order, created_at, updated_at
             ) VALUES ('site-a', 'Site A', 'https://example.com', 'openai_compatible', 'anthropic_auth_token', NULL, 1, 0, 1, 1)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO site_api_keys (
               id, site_id, label, api_key_encrypted, key_prefix, is_active, created_at, updated_at
             ) VALUES ('key-a', 'site-a', 'K 1', ?1, 'sk-…', 1, 1, 1)",
            [crypto.encrypt("sk-secret").unwrap()],
        )
        .unwrap();
        let output = temp.path().join("out");
        let created = create_backup_in(&conn, &key_path, &output, "manual").unwrap();
        let mismatched = temp.path().join("mismatched.zip");
        rewrite_bundle(&created.path, &mismatched, None, Some(&[8_u8; 32]), true);
        assert!(validate_and_extract_bundle(&mismatched, &temp.path().join("rejected")).is_err());
    }

    #[test]
    fn rejects_path_traversal_and_corrupt_database() {
        let temp = tempfile::tempdir().unwrap();
        let traversal = temp.path().join("traversal.zip");
        let mut zip = zip::ZipWriter::new(fs::File::create(&traversal).unwrap());
        zip.start_file("../master.key", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(&[1_u8; 32]).unwrap();
        zip.finish().unwrap();
        assert!(
            validate_and_extract_bundle(&traversal, &temp.path().join("traversal-out")).is_err()
        );

        let (source, conn, key_path) = fixture();
        let output = source.path().join("out");
        let created = create_backup_in(&conn, &key_path, &output, "manual").unwrap();
        let corrupt = source.path().join("corrupt.zip");
        rewrite_bundle(
            &created.path,
            &corrupt,
            Some(b"not a sqlite database"),
            None,
            true,
        );
        assert!(validate_and_extract_bundle(&corrupt, &source.path().join("corrupt-out")).is_err());
    }

    #[test]
    fn rejects_an_archive_over_the_download_limit() {
        let temp = tempfile::tempdir().unwrap();
        let oversized = temp.path().join("oversized.zip");
        let file = fs::File::create(&oversized).unwrap();
        file.set_len(MAX_ARCHIVE_BYTES + 1).unwrap();
        assert!(validate_and_extract_bundle(&oversized, &temp.path().join("out")).is_err());
    }

    #[test]
    fn prunes_old_backups_and_uses_private_permissions() {
        let (temp, conn, key_path) = fixture();
        let output = temp.path().join("out");
        let created = create_backup_in(&conn, &key_path, &output, "manual").unwrap();
        fs::write(
            output.join("xiaobai-switch-backup-20260101_000000.host.00000001.zip"),
            b"old",
        )
        .unwrap();
        fs::write(
            output.join("xiaobai-switch-backup-20260102_000000.host.00000002.zip"),
            b"older",
        )
        .unwrap();
        assert_eq!(prune_backups_in(&output, 2).unwrap(), 1);
        assert_eq!(fs::read_dir(&output).unwrap().count(), 2);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&output).unwrap().permissions().mode() & 0o777,
                0o700
            );
            assert_eq!(
                fs::metadata(created.path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
    }

    #[test]
    fn parses_device_name_from_generated_shape() {
        assert_eq!(
            parse_device_from_filename(
                "xiaobai-switch-backup-20260827_120000.mac-mini.12345678.zip"
            ),
            "mac-mini"
        );
    }

    #[test]
    fn lists_local_backups_with_visible_manifest_errors() {
        let (temp, conn, key_path) = fixture();
        let output = temp.path().join("out");
        let created = create_backup_in(&conn, &key_path, &output, "manual").unwrap();
        let broken_name = "xiaobai-switch-backup-20260827_120000.test-device.broken01.zip";
        fs::write(output.join(broken_name), b"not a zip archive").unwrap();

        let backups = list_local_backups_in(&output).unwrap();
        assert_eq!(backups.len(), 2);
        let valid = backups
            .iter()
            .find(|backup| backup.file_name == created.file_name)
            .unwrap();
        assert_eq!(valid.reason.as_deref(), Some("manual"));
        assert_eq!(
            valid.app_version.as_deref(),
            Some(env!("CARGO_PKG_VERSION"))
        );
        assert!(valid.error.is_none());

        let broken = backups
            .iter()
            .find(|backup| backup.file_name == broken_name)
            .unwrap();
        assert_eq!(broken.device_name, "test-device");
        assert!(broken
            .error
            .as_deref()
            .is_some_and(|error| error.contains("ZIP")));
    }

    #[test]
    fn local_backup_operations_reject_paths_and_non_files() {
        let temp = tempfile::tempdir().unwrap();
        let dir = temp.path().join("backups");
        fs::create_dir(&dir).unwrap();
        let name = "xiaobai-switch-backup-20260827_120000.host.12345678.zip";
        fs::write(dir.join(name), b"archive").unwrap();

        assert!(resolve_local_backup_in(&dir, "../master.key").is_err());
        assert!(resolve_local_backup_in(&dir, "unrelated.zip").is_err());
        let staged = temp.path().join("staged.zip");
        stage_local_backup_in(&dir, name, &staged).unwrap();
        assert_eq!(fs::read(&staged).unwrap(), b"archive");
        delete_local_backup_in(&dir, name).unwrap();
        assert!(!dir.join(name).exists());

        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            let outside = temp.path().join("outside.zip");
            fs::write(&outside, b"outside").unwrap();
            symlink(&outside, dir.join(name)).unwrap();
            assert!(resolve_local_backup_in(&dir, name).is_err());
        }
    }
}
