//! Discover Claude Code / Codex / Pi / Prime CLIs when a GUI-launched app has a stripped PATH.
//!
//! Finder / Dock / tray launches typically see `/usr/bin:/bin:/usr/sbin:/sbin`,
//! which misses Homebrew, nvm, fnm, volta, bun, and `~/.local/bin`. Detection
//! therefore searches those well-known locations, and treats a found binary as
//! installed even when `--version` fails (common for broken npm native shims).

use crate::domain::{CliToolInfo, TargetKind};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};

pub struct ProbeEnv {
    pub path_dirs: Vec<PathBuf>,
    pub extra_dirs: Vec<PathBuf>,
}

impl ProbeEnv {
    pub fn from_process() -> Self {
        let path_dirs: Vec<PathBuf> = env::var_os("PATH")
            .map(|p| env::split_paths(&p).collect())
            .unwrap_or_default();
        let extra_dirs = extra_bin_dirs(&dirs::home_dir().unwrap_or_default())
            .into_iter()
            .filter(|d| !path_dirs.contains(d))
            .collect();
        Self {
            path_dirs,
            extra_dirs,
        }
    }
}

pub fn probe_tool(kind: TargetKind, bin: &str) -> CliToolInfo {
    probe_tool_with(kind, bin, &ProbeEnv::from_process())
}

/// Prefer the public `prime-agent` binary; fall back to a `prime` shim if present.
pub fn probe_prime() -> CliToolInfo {
    probe_prime_with(&ProbeEnv::from_process())
}

pub fn probe_prime_with(probe: &ProbeEnv) -> CliToolInfo {
    let primary = probe_tool_with(TargetKind::Prime, "prime-agent", probe);
    if primary.installed {
        return primary;
    }
    probe_tool_with(TargetKind::Prime, "prime", probe)
}

pub fn probe_tool_with(kind: TargetKind, bin: &str, probe: &ProbeEnv) -> CliToolInfo {
    let Some(bin_path) = find_binary(bin, &probe.path_dirs, &probe.extra_dirs) else {
        return CliToolInfo {
            kind,
            installed: false,
            version: None,
            path: None,
        };
    };
    let version = read_version(&bin_path, probe);
    CliToolInfo {
        kind,
        installed: true,
        version,
        path: Some(bin_path.display().to_string()),
    }
}

pub fn extra_bin_dirs(home: &Path) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    let mut push = |p: PathBuf| {
        if !p.as_os_str().is_empty() && !dirs.contains(&p) {
            dirs.push(p);
        }
    };

    push(home.join(".local").join("bin"));
    push(home.join(".npm-global").join("bin"));
    push(home.join(".volta").join("bin"));
    push(home.join(".bun").join("bin"));
    push(home.join(".cargo").join("bin"));
    push(home.join(".asdf").join("shims"));
    push(home.join(".local").join("share").join("mise").join("shims"));
    push(home.join(".mise").join("shims"));
    push(
        home.join(".fnm")
            .join("aliases")
            .join("default")
            .join("bin"),
    );
    push(
        home.join(".local")
            .join("share")
            .join("fnm")
            .join("aliases")
            .join("default")
            .join("bin"),
    );
    push(home.join("n").join("bin"));

    for dir in nvm_bin_dirs(home) {
        push(dir);
    }
    for dir in fnm_version_bin_dirs(home) {
        push(dir);
    }

    #[cfg(target_os = "macos")]
    {
        push(PathBuf::from("/opt/homebrew/bin"));
        push(PathBuf::from("/opt/homebrew/sbin"));
        push(PathBuf::from("/usr/local/bin"));
        push(PathBuf::from("/usr/local/sbin"));
    }
    #[cfg(target_os = "linux")]
    {
        push(PathBuf::from("/usr/local/bin"));
        push(PathBuf::from("/home/linuxbrew/.linuxbrew/bin"));
    }
    #[cfg(windows)]
    {
        if let Ok(roaming) = env::var("APPDATA") {
            push(PathBuf::from(roaming).join("npm"));
        }
        if let Ok(local) = env::var("LOCALAPPDATA") {
            push(PathBuf::from(local).join("fnm"));
        }
        push(home.join("AppData").join("Roaming").join("npm"));
        push(home.join("scoop").join("shims"));
        push(PathBuf::from(r"C:\Program Files\nodejs"));
        push(PathBuf::from(r"C:\ProgramData\chocolatey\bin"));
    }

    dirs
}

pub fn find_binary(name: &str, path_dirs: &[PathBuf], extra_dirs: &[PathBuf]) -> Option<PathBuf> {
    find_in_dirs(name, path_dirs).or_else(|| find_in_dirs(name, extra_dirs))
}

fn nvm_bin_dirs(home: &Path) -> Vec<PathBuf> {
    let versions_root = home.join(".nvm").join("versions").join("node");
    let mut versions = match fs::read_dir(&versions_root) {
        Ok(rd) => rd
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .collect::<Vec<_>>(),
        Err(_) => return Vec::new(),
    };
    versions.sort_by(|a, b| b.file_name().cmp(&a.file_name()));

    if let Some(default) = read_nvm_default(home) {
        if let Some(idx) = versions.iter().position(|p| {
            p.file_name()
                .and_then(|s| s.to_str())
                .is_some_and(|name| nvm_version_matches(name, &default))
        }) {
            let preferred = versions.remove(idx);
            versions.insert(0, preferred);
        }
    }

    versions.into_iter().map(|p| p.join("bin")).collect()
}

fn read_nvm_default(home: &Path) -> Option<String> {
    let raw = fs::read_to_string(home.join(".nvm").join("alias").join("default")).ok()?;
    let alias = raw.lines().next()?.trim();
    if alias.is_empty() {
        return None;
    }
    if let Some(rest) = alias.strip_prefix("lts/") {
        let lts = fs::read_to_string(home.join(".nvm").join("alias").join("lts").join(rest)).ok();
        if let Some(resolved) = lts {
            let v = resolved.lines().next()?.trim();
            if !v.is_empty() {
                return Some(v.to_string());
            }
        }
    }
    Some(alias.to_string())
}

fn nvm_version_matches(dir_name: &str, alias: &str) -> bool {
    let dir = dir_name.trim_start_matches('v');
    let alias = alias.trim().trim_start_matches('v');
    if alias.is_empty() {
        return false;
    }
    dir == alias || dir.starts_with(&format!("{alias}."))
}

fn fnm_version_bin_dirs(home: &Path) -> Vec<PathBuf> {
    let roots = [
        home.join(".local")
            .join("share")
            .join("fnm")
            .join("node-versions"),
        home.join(".fnm").join("node-versions"),
    ];
    let mut out = Vec::new();
    for root in roots {
        let Ok(rd) = fs::read_dir(&root) else {
            continue;
        };
        let mut versions: Vec<PathBuf> = rd
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .collect();
        versions.sort_by(|a, b| b.file_name().cmp(&a.file_name()));
        for ver in versions {
            out.push(ver.join("installation").join("bin"));
        }
    }
    out
}

fn candidate_names(bin: &str) -> Vec<String> {
    #[cfg(windows)]
    {
        vec![
            format!("{bin}.exe"),
            bin.to_string(),
            format!("{bin}.cmd"),
            format!("{bin}.bat"),
        ]
    }
    #[cfg(not(windows))]
    {
        vec![bin.to_string()]
    }
}

fn find_in_dirs(name: &str, dirs: &[PathBuf]) -> Option<PathBuf> {
    let names = candidate_names(name);
    for dir in dirs {
        if !dir.is_dir() {
            continue;
        }
        for file_name in &names {
            let path = dir.join(file_name);
            if is_runnable(&path) {
                return Some(path);
            }
        }
    }
    None
}

fn is_runnable(path: &Path) -> bool {
    let meta = match fs::metadata(path) {
        Ok(m) => m,
        Err(_) => return false,
    };
    if !meta.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        meta.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

fn read_version(bin_path: &Path, probe: &ProbeEnv) -> Option<String> {
    let output = run_version_command(bin_path, probe)?;
    let line = first_line(&output.stdout).or_else(|| first_line(&output.stderr))?;
    if output.status.success() || looks_like_version(&line) {
        Some(line)
    } else {
        None
    }
}

fn run_version_command(bin_path: &Path, probe: &ProbeEnv) -> Option<std::process::Output> {
    let mut delay_ms = 5u64;
    for attempt in 0..8 {
        let mut cmd = version_command(bin_path);
        if let Some(path_value) = child_path(bin_path, probe) {
            cmd.env("PATH", path_value);
        }
        match cmd.output() {
            Ok(output) => return Some(output),
            // Linux ETXTBSY: the inode still has a writer (freshly written test shim).
            Err(err) if is_text_file_busy(&err) && attempt < 7 => {
                std::thread::sleep(std::time::Duration::from_millis(delay_ms));
                delay_ms = delay_ms.saturating_mul(2);
            }
            Err(_) => return None,
        }
    }
    None
}

fn is_text_file_busy(err: &std::io::Error) -> bool {
    if err.kind() == std::io::ErrorKind::ExecutableFileBusy {
        return true;
    }
    #[cfg(unix)]
    {
        err.raw_os_error() == Some(26)
    }
    #[cfg(not(unix))]
    {
        false
    }
}

fn looks_like_version(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    if lower.contains("error")
        || lower.contains("enoent")
        || lower.contains("not found")
        || lower.contains("errno")
    {
        return false;
    }
    has_version_token(line)
}

fn has_version_token(s: &str) -> bool {
    let bytes = s.as_bytes();
    let len = bytes.len();
    for i in 0..len.saturating_sub(2) {
        if bytes[i].is_ascii_digit() && bytes[i + 1] == b'.' && bytes[i + 2].is_ascii_digit() {
            return true;
        }
    }
    false
}

fn version_command(bin_path: &Path) -> Command {
    #[cfg(windows)]
    {
        let ext = bin_path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if ext == "cmd" || ext == "bat" {
            let mut cmd = Command::new("cmd");
            cmd.arg("/C").arg(bin_path).arg("--version");
            return cmd;
        }
    }
    let mut cmd = Command::new(bin_path);
    cmd.arg("--version");
    cmd
}

fn child_path(bin_path: &Path, probe: &ProbeEnv) -> Option<OsString> {
    let mut dirs = Vec::new();
    if let Some(parent) = bin_path.parent() {
        dirs.push(parent.to_path_buf());
    }
    dirs.extend(probe.path_dirs.iter().cloned());
    dirs.extend(probe.extra_dirs.iter().cloned());
    env::join_paths(dirs.iter().filter(|p| !p.as_os_str().is_empty())).ok()
}

fn first_line(bytes: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(bytes);
    text.lines()
        .map(str::trim)
        .find(|s| !s.is_empty())
        .map(ToString::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;

    fn write_fake_cli(dir: &Path, name: &str, unix_body: &str, win_body: &str) -> PathBuf {
        fs::create_dir_all(dir).unwrap();
        #[cfg(windows)]
        {
            let path = dir.join(format!("{name}.cmd"));
            let mut f = fs::File::create(&path).unwrap();
            writeln!(f, "@echo off").unwrap();
            for line in win_body.lines() {
                writeln!(f, "{line}").unwrap();
            }
            f.sync_all().unwrap();
            path
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            // Close + chmod a sibling, then rename. Exec'ing an inode that is
            // still open for write fails with ETXTBSY on Linux (flaky CI).
            let path = dir.join(name);
            let tmp = dir.join(format!(".{name}.writing"));
            {
                let mut f = fs::File::create(&tmp).unwrap();
                writeln!(f, "#!/bin/sh").unwrap();
                for line in unix_body.lines() {
                    writeln!(f, "{line}").unwrap();
                }
                f.sync_all().unwrap();
            }
            let mut perms = fs::metadata(&tmp).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&tmp, perms).unwrap();
            fs::rename(&tmp, &path).unwrap();
            let _ = win_body;
            path
        }
    }

    #[test]
    fn extra_bin_dirs_include_local_and_nvm_newest_first() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path();
        fs::create_dir_all(home.join(".nvm/versions/node/v20.19.5/bin")).unwrap();
        fs::create_dir_all(home.join(".nvm/versions/node/v22.22.0/bin")).unwrap();

        let dirs = extra_bin_dirs(home);
        assert!(dirs.contains(&home.join(".local/bin")));

        let nvm: Vec<_> = dirs
            .iter()
            .filter(|d| d.starts_with(home.join(".nvm")))
            .cloned()
            .collect();
        assert_eq!(
            nvm,
            vec![
                home.join(".nvm/versions/node/v22.22.0/bin"),
                home.join(".nvm/versions/node/v20.19.5/bin"),
            ]
        );
    }

    #[test]
    fn extra_bin_dirs_prefer_nvm_default_alias() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path();
        fs::create_dir_all(home.join(".nvm/versions/node/v20.19.5/bin")).unwrap();
        fs::create_dir_all(home.join(".nvm/versions/node/v22.22.0/bin")).unwrap();
        fs::create_dir_all(home.join(".nvm/alias")).unwrap();
        fs::write(home.join(".nvm/alias/default"), "20.19.5\n").unwrap();

        let nvm: Vec<_> = extra_bin_dirs(home)
            .into_iter()
            .filter(|d| d.starts_with(home.join(".nvm")))
            .collect();
        assert_eq!(nvm[0], home.join(".nvm/versions/node/v20.19.5/bin"));
        assert_eq!(nvm[1], home.join(".nvm/versions/node/v22.22.0/bin"));
    }

    #[test]
    fn extra_bin_dirs_include_fnm_installations() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path();
        fs::create_dir_all(home.join(".local/share/fnm/node-versions/v20.11.0/installation/bin"))
            .unwrap();

        let dirs = extra_bin_dirs(home);
        assert!(
            dirs.contains(&home.join(".local/share/fnm/node-versions/v20.11.0/installation/bin"))
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn extra_bin_dirs_include_homebrew_on_macos() {
        let dirs = extra_bin_dirs(Path::new("/tmp/unused-home"));
        assert!(dirs.contains(&PathBuf::from("/opt/homebrew/bin")));
        assert!(dirs.contains(&PathBuf::from("/usr/local/bin")));
    }

    #[test]
    fn find_binary_prefers_path_over_extra() {
        let path_dir = tempfile::tempdir().unwrap();
        let extra_dir = tempfile::tempdir().unwrap();
        let on_path = write_fake_cli(path_dir.path(), "claude", "echo path", "echo path");
        write_fake_cli(extra_dir.path(), "claude", "echo extra", "echo extra");

        let found = find_binary(
            "claude",
            &[path_dir.path().to_path_buf()],
            &[extra_dir.path().to_path_buf()],
        );
        assert_eq!(found.as_deref(), Some(on_path.as_path()));
    }

    #[test]
    fn find_binary_uses_extra_dirs_when_path_is_empty() {
        let extra_dir = tempfile::tempdir().unwrap();
        let extra = write_fake_cli(extra_dir.path(), "codex", "echo extra", "echo extra");

        let found = find_binary("codex", &[], &[extra_dir.path().to_path_buf()]);
        assert_eq!(found.as_deref(), Some(extra.as_path()));
    }

    #[cfg(unix)]
    #[test]
    fn find_binary_skips_non_executable_files() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("claude");
        fs::write(&path, "#!/bin/sh\necho hi\n").unwrap();
        assert_eq!(
            find_binary("claude", &[dir.path().to_path_buf()], &[]),
            None
        );
    }

    #[test]
    fn probe_treats_found_binary_as_installed_when_version_fails() {
        let dir = tempfile::tempdir().unwrap();
        let bin = write_fake_cli(dir.path(), "codex", "exit 1", "exit /B 1");
        let info = probe_tool_with(
            TargetKind::Codex,
            "codex",
            &ProbeEnv {
                path_dirs: vec![],
                extra_dirs: vec![dir.path().to_path_buf()],
            },
        );
        assert!(info.installed);
        assert_eq!(info.kind, TargetKind::Codex);
        assert_eq!(info.path.as_deref(), Some(bin.to_str().unwrap()));
        assert_eq!(info.version, None);
    }

    #[test]
    fn probe_keeps_version_token_even_when_exit_fails() {
        let dir = tempfile::tempdir().unwrap();
        write_fake_cli(
            dir.path(),
            "codex",
            "echo 'codex-cli 0.42.0'\nexit 1",
            "echo codex-cli 0.42.0\r\nexit /B 1",
        );
        let info = probe_tool_with(
            TargetKind::Codex,
            "codex",
            &ProbeEnv {
                path_dirs: vec![dir.path().to_path_buf()],
                extra_dirs: vec![],
            },
        );
        assert!(info.installed);
        assert_eq!(info.version.as_deref(), Some("codex-cli 0.42.0"));
    }

    #[test]
    fn probe_ignores_error_output_as_version() {
        let dir = tempfile::tempdir().unwrap();
        write_fake_cli(
            dir.path(),
            "codex",
            "echo 'Error: spawn /Users/x/.nvm/versions/node/v20.19.5/lib/codex ENOENT' >&2\nexit 1",
            "echo Error: spawn C:\\Users\\x\\nvm\\v20.19.5\\codex ENOENT 1>&2\r\nexit /B 1",
        );
        let info = probe_tool_with(
            TargetKind::Codex,
            "codex",
            &ProbeEnv {
                path_dirs: vec![dir.path().to_path_buf()],
                extra_dirs: vec![],
            },
        );
        assert!(info.installed);
        assert_eq!(info.version, None);
    }

    #[test]
    fn probe_reads_version_from_stderr() {
        let dir = tempfile::tempdir().unwrap();
        write_fake_cli(
            dir.path(),
            "claude",
            "echo '2.1.197 (Claude Code)' >&2\nexit 0",
            "echo 2.1.197 (Claude Code) 1>&2",
        );
        let info = probe_tool_with(
            TargetKind::ClaudeCode,
            "claude",
            &ProbeEnv {
                path_dirs: vec![dir.path().to_path_buf()],
                extra_dirs: vec![],
            },
        );
        assert!(info.installed);
        assert_eq!(info.version.as_deref(), Some("2.1.197 (Claude Code)"));
    }

    #[cfg(unix)]
    #[test]
    fn text_file_busy_detects_etxtbsy() {
        let err = std::io::Error::from_raw_os_error(26);
        assert!(is_text_file_busy(&err));
        assert!(!is_text_file_busy(&std::io::Error::from(
            std::io::ErrorKind::NotFound
        )));
    }

    #[test]
    fn probe_reads_version_from_stdout() {
        let dir = tempfile::tempdir().unwrap();
        write_fake_cli(
            dir.path(),
            "claude",
            "echo '2.1.212 (Claude Code)'",
            "echo 2.1.212 (Claude Code)",
        );
        let info = probe_tool_with(
            TargetKind::ClaudeCode,
            "claude",
            &ProbeEnv {
                path_dirs: vec![],
                extra_dirs: vec![dir.path().to_path_buf()],
            },
        );
        assert!(info.installed);
        assert_eq!(
            info.version.as_deref(),
            Some("2.1.212 (Claude Code)"),
            "path={:?} installed={}",
            info.path,
            info.installed
        );
    }

    #[test]
    fn empty_path_finds_well_known_binaries_when_present() {
        let home = dirs::home_dir().unwrap_or_default();
        let extras = extra_bin_dirs(&home);
        for (kind, name) in [
            (TargetKind::ClaudeCode, "claude"),
            (TargetKind::Codex, "codex"),
        ] {
            if find_binary(name, &[], &extras).is_none() {
                continue;
            }
            let info = probe_tool_with(
                kind,
                name,
                &ProbeEnv {
                    path_dirs: vec![],
                    extra_dirs: extras.clone(),
                },
            );
            assert!(
                info.installed,
                "{name} exists in extra dirs but a GUI-like empty PATH did not detect it"
            );
            assert!(info.path.is_some());
        }
    }

    #[test]
    fn probe_missing_binary_is_not_installed() {
        let info = probe_tool_with(
            TargetKind::ClaudeCode,
            "claude",
            &ProbeEnv {
                path_dirs: vec![],
                extra_dirs: vec![],
            },
        );
        assert!(!info.installed);
        assert_eq!(info.path, None);
        assert_eq!(info.version, None);
    }

    #[cfg(unix)]
    #[test]
    fn probe_puts_binary_dir_on_child_path_for_node_shims() {
        let dir = tempfile::tempdir().unwrap();
        write_fake_cli(dir.path(), "node", "echo node-ok", "echo node-ok");
        write_fake_cli(
            dir.path(),
            "codex",
            r#"command -v node >/dev/null && echo "codex-cli 0.42.0" || exit 1"#,
            "echo codex-cli 0.42.0",
        );
        let info = probe_tool_with(
            TargetKind::Codex,
            "codex",
            &ProbeEnv {
                path_dirs: vec![],
                extra_dirs: vec![dir.path().to_path_buf()],
            },
        );
        assert!(info.installed);
        assert_eq!(info.version.as_deref(), Some("codex-cli 0.42.0"));
    }

    #[test]
    fn probe_prime_prefers_prime_agent_then_falls_back_to_prime() {
        let missing = probe_prime_with(&ProbeEnv {
            path_dirs: vec![],
            extra_dirs: vec![],
        });
        assert!(!missing.installed);
        assert_eq!(missing.kind, TargetKind::Prime);

        let fallback = tempfile::tempdir().unwrap();
        write_fake_cli(
            fallback.path(),
            "prime",
            "echo prime 1.0.0",
            "echo prime 1.0.0",
        );
        let info = probe_prime_with(&ProbeEnv {
            path_dirs: vec![],
            extra_dirs: vec![fallback.path().to_path_buf()],
        });
        assert!(info.installed);
        assert_eq!(info.kind, TargetKind::Prime);
        assert!(info
            .path
            .as_deref()
            .is_some_and(|path| path.contains("prime")),);

        let preferred = tempfile::tempdir().unwrap();
        write_fake_cli(
            preferred.path(),
            "prime-agent",
            "echo prime-agent 2.0.0",
            "echo prime-agent 2.0.0",
        );
        write_fake_cli(
            preferred.path(),
            "prime",
            "echo prime 1.0.0",
            "echo prime 1.0.0",
        );
        let info = probe_prime_with(&ProbeEnv {
            path_dirs: vec![],
            extra_dirs: vec![preferred.path().to_path_buf()],
        });
        assert!(info.installed);
        assert_eq!(info.kind, TargetKind::Prime);
        assert!(info
            .path
            .as_deref()
            .is_some_and(|path| path.contains("prime-agent")),);
    }
}
