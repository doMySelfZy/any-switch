use crate::error::{AppError, AppResult};
use crate::paths::{app_dir, ensure_app_dirs, home_dir};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

const MAX_PENDING_LEN: usize = 16 * 1024;

pub fn pending_deep_link_path() -> AppResult<PathBuf> {
    Ok(app_dir()?.join("pending-deeplink.url"))
}

pub fn parse_pending_deep_link_file(raw: &str) -> Option<String> {
    let url = raw.trim();
    if url.is_empty() || url.len() > MAX_PENDING_LEN {
        return None;
    }
    if !url.starts_with("xiaobaiswitch:") {
        return None;
    }
    Some(url.to_string())
}

pub fn take_pending_deep_link() -> AppResult<Option<String>> {
    let path = pending_deep_link_path()?;
    let raw = match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    let _ = fs::remove_file(&path);
    Ok(parse_pending_deep_link_file(&raw))
}

#[cfg(target_os = "macos")]
pub fn install_dev_url_handler() -> AppResult<PathBuf> {
    ensure_app_dirs()?;
    let apps = home_dir()?.join("Applications");
    fs::create_dir_all(&apps)?;
    let dest = apps.join("XiaoBaiSwitch Dev.app");

    let script = r#"on open location theURL
	set dest to (POSIX path of (path to home folder)) & ".xiaobai-switch/pending-deeplink.url"
	do shell script "mkdir -p \"$HOME/.xiaobai-switch\" && umask 077 && printf '%s' " & quoted form of theURL & " > " & quoted form of dest & ".tmp && mv " & quoted form of dest & ".tmp " & quoted form of dest
end open location
"#;

    let tmp = std::env::temp_dir().join("XiaoBaiSwitch-Dev-url-handler.applescript");
    fs::write(&tmp, script)?;

    if dest.exists() {
        let _ = fs::remove_dir_all(&dest);
    }

    let status = Command::new("osacompile")
        .arg("-o")
        .arg(&dest)
        .arg(&tmp)
        .status()
        .map_err(|e| AppError::new("internal", format!("osacompile failed: {e}")))?;
    if !status.success() {
        return Err(AppError::new(
            "internal",
            "osacompile failed to build URL handler app",
        ));
    }

    let info = dest.join("Contents/Info.plist");
    patch_handler_info_plist(&info)?;

    let lsregister = "/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister";
    let _ = Command::new(lsregister).arg("-f").arg(&dest).status();

    tracing::info!("registered xiaobaiswitch:// via {}", dest.display());
    Ok(dest)
}

#[cfg(target_os = "macos")]
fn patch_handler_info_plist(info: &PathBuf) -> AppResult<()> {
    let buddy = "/usr/libexec/PlistBuddy";
    let run = |cmd: &str, required: bool| -> AppResult<()> {
        let status = Command::new(buddy)
            .args(["-c", cmd])
            .arg(info)
            .status()
            .map_err(|e| AppError::new("internal", format!("PlistBuddy failed: {e}")))?;
        if required && !status.success() {
            return Err(AppError::new(
                "internal",
                format!("PlistBuddy command failed: {cmd}"),
            ));
        }
        Ok(())
    };

    let _ = run("Delete :CFBundleURLTypes", false);
    run("Add :CFBundleURLTypes array", true)?;
    run("Add :CFBundleURLTypes:0 dict", true)?;
    run(
        "Add :CFBundleURLTypes:0:CFBundleTypeRole string Editor",
        true,
    )?;
    run(
        "Add :CFBundleURLTypes:0:CFBundleURLName string com.domyselfzy.xiaobai-switch.url-handler",
        true,
    )?;
    run("Add :CFBundleURLTypes:0:CFBundleURLSchemes array", true)?;
    run(
        "Add :CFBundleURLTypes:0:CFBundleURLSchemes:0 string xiaobaiswitch",
        true,
    )?;
    let _ = run(
        "Set :CFBundleIdentifier com.domyselfzy.xiaobai-switch.url-handler",
        false,
    );
    let _ = run(
        "Add :CFBundleIdentifier string com.domyselfzy.xiaobai-switch.url-handler",
        false,
    );
    let _ = run("Set :CFBundleName XiaoBaiSwitch Dev", false);
    let _ = run("Add :CFBundleName string XiaoBaiSwitch Dev", false);
    let _ = run("Set :CFBundleDisplayName XiaoBaiSwitch Dev", false);
    let _ = run("Add :CFBundleDisplayName string XiaoBaiSwitch Dev", false);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_pending_accepts_scheme_only() {
        assert_eq!(
            parse_pending_deep_link_file(
                "  xiaobaiswitch://sites?name=A&baseurls=https://a.example.com  \n"
            )
            .as_deref(),
            Some("xiaobaiswitch://sites?name=A&baseurls=https://a.example.com")
        );
    }

    #[test]
    fn parse_pending_rejects_other_schemes() {
        assert_eq!(parse_pending_deep_link_file("https://example.com"), None);
        assert_eq!(parse_pending_deep_link_file(""), None);
        assert_eq!(parse_pending_deep_link_file("aqbot://providers"), None);
    }
}
