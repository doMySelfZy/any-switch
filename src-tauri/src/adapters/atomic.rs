use crate::error::{AppError, AppResult};
use crate::paths::set_secret_permissions;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Write content to path atomically (same-dir temp + rename).
pub fn atomic_write(path: &Path, content: &[u8], secret: bool) -> AppResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("file");
    let tmp_name = format!(
        "{}.xiaobai-{}-{}.tmp",
        file_name,
        std::process::id(),
        rand::random::<u32>()
    );
    let tmp_path = path
        .parent()
        .map(|p| p.join(&tmp_name))
        .unwrap_or_else(|| PathBuf::from(&tmp_name));

    {
        let mut f = fs::File::create(&tmp_path)?;
        f.write_all(content)?;
        f.sync_all()?;
    }

    let mut last_err = None;
    for _ in 0..3 {
        match try_replace(&tmp_path, path) {
            Ok(()) => {
                if secret {
                    set_secret_permissions(path);
                }
                return Ok(());
            }
            Err(e) => {
                last_err = Some(e);
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    }

    let _ = fs::remove_file(&tmp_path);
    Err(AppError::new(
        "atomic_write_failed",
        format!(
            "failed to write {}: {}",
            path.display(),
            last_err
                .map(|e| e.to_string())
                .unwrap_or_else(|| "unknown".into())
        ),
    ))
}

fn try_replace(tmp: &Path, dest: &Path) -> std::io::Result<()> {
    if !dest.exists() {
        return fs::rename(tmp, dest);
    }
    // Unix rename replaces; Windows may need remove-then-rename
    #[cfg(unix)]
    {
        fs::rename(tmp, dest)
    }
    #[cfg(not(unix))]
    {
        let _ = fs::remove_file(dest);
        fs::rename(tmp, dest)
    }
}

pub const BACKUP_ORIGINS_FILE: &str = ".origins.json";

/// 目标 CLI 配置文件的互斥锁：`<file>.lock` 目录。与 Pi/Prime 适配器用的是同一套约定，
/// 所以针对同一个文件（例如 Prime 的 settings.json）的读写会互相排他。
/// 客户端可能在任意时刻改写自己的配置，读-改-写期间必须持锁，否则会丢掉它的并发更新。
pub struct FileLock {
    path: PathBuf,
}

impl FileLock {
    pub fn acquire(file: &Path) -> AppResult<Self> {
        let file_name = file
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| AppError::new("internal", "invalid config path"))?;
        let path = file.with_file_name(format!("{file_name}.lock"));
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        for attempt in 0..10 {
            match fs::create_dir(&path) {
                Ok(()) => return Ok(Self { path }),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    if attempt < 9 {
                        std::thread::sleep(Duration::from_millis(20));
                    }
                }
                Err(error) => return Err(error.into()),
            }
        }
        Err(AppError::new(
            "lock_busy",
            format!("the target CLI is using {}", file.display()),
        ))
    }
}

impl Drop for FileLock {
    fn drop(&mut self) {
        let _ = fs::remove_dir(&self.path);
    }
}

pub fn backup_file(src: &Path, backup_dir: &Path) -> AppResult<PathBuf> {
    if !src.exists() {
        return Err(AppError::new(
            "backup_failed",
            format!("source missing: {}", src.display()),
        ));
    }
    fs::create_dir_all(backup_dir)?;
    let name = unique_backup_name(backup_dir, src);
    let dest = backup_dir.join(&name);
    if dest.exists() {
        remember_origin(backup_dir, &name, src);
        return Ok(dest);
    }
    fs::copy(src, &dest)?;
    remember_origin(backup_dir, &name, src);
    Ok(dest)
}

fn unique_backup_name(backup_dir: &Path, src: &Path) -> String {
    let base = src
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("file")
        .to_string();
    let dest = backup_dir.join(&base);
    if !dest.exists() {
        return base;
    }
    let origins = read_origins(backup_dir);
    if origins.get(&base).map(PathBuf::from).as_deref() == Some(src) {
        return base;
    }
    if let Some(parent) = src
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str())
    {
        let prefixed = format!("{parent}_{base}");
        if !backup_dir.join(&prefixed).exists() {
            return prefixed;
        }
    }
    for i in 2..100 {
        let candidate = format!("{base}.{i}");
        if !backup_dir.join(&candidate).exists() {
            return candidate;
        }
    }
    base
}

fn remember_origin(backup_dir: &Path, name: &str, src: &Path) {
    let mut origins = read_origins(backup_dir);
    origins.insert(name.to_string(), src.display().to_string());
    if let Ok(text) = serde_json::to_string(&origins) {
        let _ = fs::write(backup_dir.join(BACKUP_ORIGINS_FILE), text);
    }
}

pub fn read_origins(backup_dir: &Path) -> std::collections::HashMap<String, String> {
    let path = backup_dir.join(BACKUP_ORIGINS_FILE);
    let Ok(text) = fs::read_to_string(path) else {
        return std::collections::HashMap::new();
    };
    serde_json::from_str(&text).unwrap_or_default()
}

pub fn restore_file(backup: &Path, dest: &Path) -> AppResult<()> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(backup, dest)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn write_and_replace() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("settings.json");
        atomic_write(&path, b"{\"a\":1}", false).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "{\"a\":1}");
        atomic_write(&path, b"{\"a\":2}", false).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "{\"a\":2}");
    }
}
