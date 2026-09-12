use crate::error::{AppError, AppResult};
use serde::Serialize;
use std::fs;
use std::path::PathBuf;

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

pub fn app_dir() -> AppResult<PathBuf> {
    // 测试/多实例隔离用：显式设置 XIAOBAI_SWITCH_DATA_DIR 时优先。
    if let Ok(dir) = std::env::var("XIAOBAI_SWITCH_DATA_DIR") {
        let trimmed = dir.trim();
        if !trimmed.is_empty() {
            return Ok(PathBuf::from(trimmed));
        }
    }
    Ok(home_dir()?.join(".xiaobai-switch"))
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

pub fn db_path() -> AppResult<PathBuf> {
    Ok(app_dir()?.join("xiaobai-switch.db"))
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
}
