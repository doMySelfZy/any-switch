use crate::adapters::atomic::atomic_write;
use crate::domain::AppSettings;
use crate::error::{AppError, AppResult};
use crate::paths::{
    home_dir, resolve_claude_home, resolve_codex_home, resolve_pi_agent_dir,
    resolve_prime_agent_dir,
};
use crate::repo;
use crate::state::AppState;
use chrono::Utc;
use futures_util::StreamExt;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_yaml::Value as YamlValue;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{Cursor, Read};
use std::path::{Component, Path, PathBuf};
use std::time::Duration;
use tauri::State;

const ENABLED_FILE: &str = "SKILL.md";
const DISABLED_FILE: &str = "SKILL.md.disabled";
const INSTALL_MANIFEST: &str = ".xiaobai-skill.json";
const MAX_SKILL_FILE_BYTES: u64 = 1024 * 1024;
const MAX_ARCHIVE_BYTES: usize = 32 * 1024 * 1024;
const MAX_ARCHIVE_ENTRY_BYTES: u64 = 32 * 1024 * 1024;
const MAX_EXTRACTED_BYTES: u64 = 128 * 1024 * 1024;
const MAX_ARCHIVE_ENTRIES: usize = 4096;
const MAX_SCAN_DEPTH: usize = 12;
const MAX_DETAIL_FILES: usize = 4096;
const SKILLS_SH_SEARCH_URL: &str = "https://skills.sh/api/search";
const SKILLS_SH_BROWSE_QUERY: &str = "skill";
const SKILLS_SH_MIN_QUERY_CHARS: usize = 2;
const MARKETPLACE_LIMIT: &str = "20";

static SKILL_FS_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum SkillTarget {
    Agents,
    ClaudeCode,
    Codex,
    Pi,
    Prime,
}

impl SkillTarget {
    const ALL: [SkillTarget; 5] = [
        Self::Agents,
        Self::ClaudeCode,
        Self::Codex,
        Self::Pi,
        Self::Prime,
    ];

    fn sort_key(self) -> usize {
        Self::ALL
            .iter()
            .position(|item| *item == self)
            .unwrap_or(usize::MAX)
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SkillInfo {
    pub name: String,
    pub description: String,
    pub author: Option<String>,
    pub version: Option<String>,
    pub target: SkillTarget,
    pub source_path: String,
    pub skills_path: String,
    pub directory_path: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillDetail {
    pub info: SkillInfo,
    pub content: String,
    pub files: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketplaceSkill {
    pub name: String,
    pub description: String,
    pub repo: String,
    pub stars: i64,
    pub installs: i64,
    pub installed_targets: Vec<SkillTarget>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct SkillFrontmatter {
    name: Option<String>,
    description: Option<String>,
    author: Option<YamlValue>,
    version: Option<YamlValue>,
    #[serde(default)]
    metadata: HashMap<String, YamlValue>,
}

#[derive(Debug)]
struct ParsedSkill {
    name: String,
    description: String,
    author: Option<String>,
    version: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct InstallManifest {
    source_ref: String,
    installed_at: String,
}

fn validation_error(message: impl Into<String>) -> AppError {
    AppError::new("validation_failed", message)
}

fn invalid_config(message: impl Into<String>) -> AppError {
    AppError::new("invalid_config", message)
}

fn target_skills_path(settings: &AppSettings, target: SkillTarget) -> AppResult<PathBuf> {
    let root = match target {
        SkillTarget::Agents => home_dir()?.join(".agents"),
        SkillTarget::ClaudeCode => resolve_claude_home(settings.claude_home_override.as_deref())?,
        SkillTarget::Codex => resolve_codex_home(settings.codex_home_override.as_deref())?,
        SkillTarget::Pi => resolve_pi_agent_dir(settings.pi_agent_dir_override.as_deref())?,
        SkillTarget::Prime => resolve_prime_agent_dir(settings.prime_agent_dir_override.as_deref())?,
    };
    Ok(root.join("skills"))
}

fn all_target_roots(settings: &AppSettings) -> AppResult<Vec<(SkillTarget, PathBuf)>> {
    SkillTarget::ALL
        .into_iter()
        .map(|target| Ok((target, target_skills_path(settings, target)?)))
        .collect()
}

fn load_settings(state: &AppState) -> AppResult<AppSettings> {
    state.db.with_conn(repo::settings::get_settings)
}

#[tauri::command]
pub fn list_skills(state: State<'_, AppState>) -> AppResult<Vec<SkillInfo>> {
    let settings = load_settings(&state)?;
    let mut skills = Vec::new();
    for (target, root) in all_target_roots(&settings)? {
        skills.extend(scan_target(target, &root)?);
    }
    skills.sort_by(|left, right| {
        left.target
            .sort_key()
            .cmp(&right.target.sort_key())
            .then_with(|| left.name.cmp(&right.name))
            .then_with(|| left.source_path.cmp(&right.source_path))
    });
    Ok(skills)
}

#[tauri::command]
pub fn get_skill(
    state: State<'_, AppState>,
    target: SkillTarget,
    source_path: String,
) -> AppResult<SkillDetail> {
    let settings = load_settings(&state)?;
    let root = target_skills_path(&settings, target)?;
    let source = validate_skill_path(&root, Path::new(&source_path))?;
    let info = build_skill_info(target, &root, &source)?;
    let content = read_skill_text(&source)?;
    let directory = source
        .parent()
        .ok_or_else(|| validation_error("skill file has no parent directory"))?;
    let files = list_detail_files(directory)?;
    Ok(SkillDetail {
        info,
        content,
        files,
    })
}

#[tauri::command]
pub fn set_skill_enabled(
    state: State<'_, AppState>,
    target: SkillTarget,
    source_path: String,
    enabled: bool,
) -> AppResult<()> {
    let settings = load_settings(&state)?;
    let root = target_skills_path(&settings, target)?;
    let _guard = SKILL_FS_LOCK.lock();
    set_skill_enabled_at(&root, Path::new(&source_path), enabled)
}

#[tauri::command]
pub async fn install_skill(
    state: State<'_, AppState>,
    source: String,
    target: SkillTarget,
) -> AppResult<String> {
    let (owner, repo_name) = parse_github_source(&source)?;
    let settings = load_settings(&state)?;
    let skills_root = target_skills_path(&settings, target)?;
    let archive = download_github_archive(&settings, &owner, &repo_name).await?;
    let source_ref = format!("{owner}/{repo_name}");
    let _guard = SKILL_FS_LOCK.lock();
    install_archive(&archive, &skills_root, &repo_name, &source_ref)?;
    Ok(repo_name)
}

#[tauri::command]
pub fn uninstall_skill(
    state: State<'_, AppState>,
    target: SkillTarget,
    source_path: String,
) -> AppResult<()> {
    let settings = load_settings(&state)?;
    let root = target_skills_path(&settings, target)?;
    let _guard = SKILL_FS_LOCK.lock();
    let source = validate_skill_path(&root, Path::new(&source_path))?;
    let skill_directory = source
        .parent()
        .ok_or_else(|| validation_error("skill file has no parent directory"))?;
    let canonical_root = fs::canonicalize(&root)?;
    let canonical_dir = fs::canonicalize(skill_directory)?;
    if canonical_dir == canonical_root {
        return Err(validation_error(
            "refusing to remove a skills root directory",
        ));
    }
    let uninstall_dir = resolve_uninstall_directory(&canonical_root, &canonical_dir)?;
    fs::remove_dir_all(uninstall_dir)?;
    Ok(())
}

#[tauri::command]
pub async fn search_skill_marketplace(
    state: State<'_, AppState>,
    query: String,
    source: Option<String>,
) -> AppResult<Vec<MarketplaceSkill>> {
    let settings = load_settings(&state)?;
    let roots = all_target_roots(&settings)?;
    let installed = installed_source_targets(&roots)?;
    let client = crate::http_client::build_client(&settings, Duration::from_secs(20))?;
    match source.as_deref().unwrap_or("skills.sh") {
        "skills.sh" => search_skills_sh(&client, query.trim(), &installed).await,
        "github" => search_github(&client, query.trim(), &installed).await,
        _ => Err(validation_error("unsupported skill marketplace source")),
    }
}

fn scan_target(target: SkillTarget, root: &Path) -> AppResult<Vec<SkillInfo>> {
    if !root.exists() {
        return Ok(Vec::new());
    }
    if !root.is_dir() {
        return Err(invalid_config(format!(
            "skills path is not a directory: {}",
            root.display()
        )));
    }
    let mut skills = Vec::new();
    scan_directory(target, root, root, 0, &mut skills)?;
    Ok(skills)
}

fn scan_directory(
    target: SkillTarget,
    root: &Path,
    directory: &Path,
    depth: usize,
    skills: &mut Vec<SkillInfo>,
) -> AppResult<()> {
    if depth > MAX_SCAN_DEPTH {
        return Err(validation_error("skill directory nesting is too deep"));
    }
    let enabled = directory.join(ENABLED_FILE);
    let disabled = directory.join(DISABLED_FILE);
    if enabled.exists() && disabled.exists() {
        return Err(invalid_config(format!(
            "both {ENABLED_FILE} and {DISABLED_FILE} exist in {}",
            directory.display()
        )));
    }
    if enabled.is_file() || disabled.is_file() {
        let source = if enabled.is_file() { enabled } else { disabled };
        skills.push(build_skill_info(target, root, &source)?);
        return Ok(());
    }

    let mut entries = fs::read_dir(directory)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        if entry.file_name().to_string_lossy().starts_with('.') {
            continue;
        }
        let file_type = entry.file_type()?;
        if file_type.is_dir() && !file_type.is_symlink() {
            scan_directory(target, root, &entry.path(), depth + 1, skills)?;
        }
    }
    Ok(())
}

fn build_skill_info(target: SkillTarget, root: &Path, source: &Path) -> AppResult<SkillInfo> {
    let content = read_skill_text(source)?;
    let parsed = parse_skill(&content)?;
    let directory = source
        .parent()
        .ok_or_else(|| validation_error("skill file has no parent directory"))?;
    Ok(SkillInfo {
        name: parsed.name,
        description: parsed.description,
        author: parsed.author,
        version: parsed.version,
        target,
        source_path: source.display().to_string(),
        skills_path: root.display().to_string(),
        directory_path: directory.display().to_string(),
        enabled: source.file_name().and_then(|name| name.to_str()) == Some(ENABLED_FILE),
    })
}

fn read_skill_text(path: &Path) -> AppResult<String> {
    let metadata = fs::metadata(path)?;
    if metadata.len() > MAX_SKILL_FILE_BYTES {
        return Err(validation_error(format!(
            "skill file exceeds {} bytes: {}",
            MAX_SKILL_FILE_BYTES,
            path.display()
        )));
    }
    Ok(fs::read_to_string(path)?)
}

fn parse_skill(content: &str) -> AppResult<ParsedSkill> {
    let (yaml, _) = split_frontmatter(content)?;
    let frontmatter: SkillFrontmatter = serde_yaml::from_str(yaml)
        .map_err(|error| invalid_config(format!("invalid skill frontmatter: {error}")))?;
    let name = required_frontmatter(frontmatter.name, "name")?;
    let description = required_frontmatter(frontmatter.description, "description")?;
    let author = frontmatter
        .author
        .as_ref()
        .and_then(yaml_scalar)
        .or_else(|| frontmatter.metadata.get("author").and_then(yaml_scalar));
    let version = frontmatter
        .version
        .as_ref()
        .and_then(yaml_scalar)
        .or_else(|| frontmatter.metadata.get("version").and_then(yaml_scalar));
    Ok(ParsedSkill {
        name,
        description,
        author,
        version,
    })
}

fn split_frontmatter(content: &str) -> AppResult<(&str, &str)> {
    let mut offset = 0;
    let mut lines = content.split_inclusive('\n');
    let first = lines
        .next()
        .ok_or_else(|| invalid_config("skill file is empty"))?;
    if first.trim_end_matches(['\r', '\n']) != "---" {
        return Err(invalid_config(
            "skill file must begin with YAML frontmatter",
        ));
    }
    offset += first.len();
    let yaml_start = offset;
    for line in lines {
        let line_start = offset;
        offset += line.len();
        if line.trim_end_matches(['\r', '\n']) == "---" {
            return Ok((&content[yaml_start..line_start], &content[offset..]));
        }
    }
    Err(invalid_config("skill frontmatter is not closed"))
}

fn required_frontmatter(value: Option<String>, field: &str) -> AppResult<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| invalid_config(format!("skill frontmatter requires {field}")))
}

fn yaml_scalar(value: &YamlValue) -> Option<String> {
    match value {
        YamlValue::String(value) => Some(value.clone()),
        YamlValue::Number(value) => Some(value.to_string()),
        YamlValue::Bool(value) => Some(value.to_string()),
        _ => None,
    }
}

fn validate_skill_path(root: &Path, source: &Path) -> AppResult<PathBuf> {
    let file_name = source.file_name().and_then(|name| name.to_str());
    if !matches!(file_name, Some(ENABLED_FILE | DISABLED_FILE)) {
        return Err(validation_error("sourcePath must point to a skill file"));
    }
    let source_metadata = fs::symlink_metadata(source)?;
    if source_metadata.file_type().is_symlink() || !source_metadata.is_file() {
        return Err(validation_error("sourcePath must be a regular skill file"));
    }
    let canonical_root = fs::canonicalize(root)?;
    let canonical_source = fs::canonicalize(source)?;
    let relative = canonical_source
        .strip_prefix(&canonical_root)
        .map_err(|_| validation_error("skill is outside the target skills root"))?;
    if relative.components().any(is_hidden_component) {
        return Err(validation_error(
            "hidden skill directories cannot be managed",
        ));
    }
    Ok(canonical_source)
}

fn is_hidden_component(component: Component<'_>) -> bool {
    matches!(component, Component::Normal(name) if name.to_string_lossy().starts_with('.'))
}

fn set_skill_enabled_at(root: &Path, source: &Path, enabled: bool) -> AppResult<()> {
    let source = validate_skill_path(root, source)?;
    let current_enabled = source.file_name().and_then(|name| name.to_str()) == Some(ENABLED_FILE);
    if current_enabled == enabled {
        return Ok(());
    }
    let destination = source.with_file_name(if enabled { ENABLED_FILE } else { DISABLED_FILE });
    if destination.exists() {
        return Err(invalid_config(format!(
            "cannot toggle skill because {} already exists",
            destination.display()
        )));
    }
    fs::rename(source, destination)?;
    Ok(())
}

fn list_detail_files(directory: &Path) -> AppResult<Vec<String>> {
    let mut files = Vec::new();
    collect_detail_files(directory, directory, 0, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_detail_files(
    root: &Path,
    directory: &Path,
    depth: usize,
    files: &mut Vec<String>,
) -> AppResult<()> {
    if depth > MAX_SCAN_DEPTH || files.len() > MAX_DETAIL_FILES {
        return Err(validation_error("skill contains too many nested files"));
    }
    let mut entries = fs::read_dir(directory)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        if entry.file_name().to_string_lossy().starts_with('.') {
            continue;
        }
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            collect_detail_files(root, &entry.path(), depth + 1, files)?;
        } else if file_type.is_file() {
            let relative = entry
                .path()
                .strip_prefix(root)
                .map_err(|_| validation_error("skill file escaped its directory"))?
                .to_string_lossy()
                .to_string();
            files.push(relative);
            if files.len() > MAX_DETAIL_FILES {
                return Err(validation_error("skill contains too many files"));
            }
        }
    }
    Ok(())
}

fn parse_github_source(source: &str) -> AppResult<(String, String)> {
    let trimmed = source.trim().trim_end_matches('/');
    let parts = if trimmed.contains("://") {
        let url = url::Url::parse(trimmed)
            .map_err(|_| validation_error("invalid GitHub skill source URL"))?;
        if url.scheme() != "https" || url.host_str() != Some("github.com") {
            return Err(validation_error("skill source must use https://github.com"));
        }
        if url.query().is_some() || url.fragment().is_some() {
            return Err(validation_error(
                "GitHub skill source cannot contain query or fragment",
            ));
        }
        url.path_segments()
            .ok_or_else(|| validation_error("invalid GitHub skill source"))?
            .filter(|part| !part.is_empty())
            .map(str::to_string)
            .collect::<Vec<_>>()
    } else {
        trimmed.split('/').map(str::to_string).collect::<Vec<_>>()
    };
    if parts.len() != 2 {
        return Err(validation_error(
            "skill source must be owner/repo or a GitHub repository URL",
        ));
    }
    let owner = parts[0].trim().to_string();
    let repo = parts[1].trim().trim_end_matches(".git").to_string();
    validate_github_component(&owner, "owner")?;
    validate_github_component(&repo, "repository")?;
    Ok((owner, repo))
}

fn validate_github_component(value: &str, label: &str) -> AppResult<()> {
    let valid = !value.is_empty()
        && value != "."
        && value != ".."
        && !value.starts_with('.')
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "-_.".contains(character));
    if valid {
        Ok(())
    } else {
        Err(validation_error(format!("invalid GitHub {label}")))
    }
}

fn github_archive_candidates(owner: &str, repo: &str) -> Vec<(String, bool)> {
    vec![
        (
            format!("https://github.com/{owner}/{repo}/archive/HEAD.zip"),
            false,
        ),
        (
            format!("https://codeload.github.com/{owner}/{repo}/zip/refs/heads/main"),
            false,
        ),
        (
            format!("https://codeload.github.com/{owner}/{repo}/zip/refs/heads/master"),
            false,
        ),
        (
            format!("https://api.github.com/repos/{owner}/{repo}/zipball"),
            true,
        ),
    ]
}

async fn download_limited_bytes(
    client: &reqwest::Client,
    url: &str,
    github_api: bool,
) -> AppResult<Vec<u8>> {
    let mut request = client.get(url);
    if github_api {
        request = request.header("Accept", "application/vnd.github+json");
    }
    let response = request.send().await?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(http_status_error(status, &body));
    }
    if response
        .content_length()
        .is_some_and(|length| length > MAX_ARCHIVE_BYTES as u64)
    {
        return Err(validation_error("skill archive is too large"));
    }
    let mut bytes = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        if bytes.len().saturating_add(chunk.len()) > MAX_ARCHIVE_BYTES {
            return Err(validation_error("skill archive is too large"));
        }
        bytes.extend_from_slice(&chunk);
    }
    if bytes.is_empty() {
        return Err(validation_error("skill archive is empty"));
    }
    Ok(bytes)
}

async fn download_github_archive(
    settings: &AppSettings,
    owner: &str,
    repo: &str,
) -> AppResult<Vec<u8>> {
    let client = crate::http_client::build_client(settings, Duration::from_secs(45))?;
    let mut last_error = None;
    for (url, github_api) in github_archive_candidates(owner, repo) {
        match download_limited_bytes(&client, &url, github_api).await {
            Ok(bytes) => return Ok(bytes),
            Err(error) => last_error = Some(error),
        }
    }
    Err(last_error.unwrap_or_else(|| AppError::new("network", "failed to download skill archive")))
}

fn install_archive(
    archive: &[u8],
    skills_root: &Path,
    repo_name: &str,
    source_ref: &str,
) -> AppResult<()> {
    fs::create_dir_all(skills_root)?;
    if !skills_root.is_dir() {
        return Err(invalid_config("target skills path is not a directory"));
    }
    let stage = tempfile::Builder::new()
        .prefix(".xiaobai-skill-")
        .tempdir_in(skills_root)?;
    let extract_root = stage.path().join("extract");
    fs::create_dir(&extract_root)?;
    let top_level = extract_archive(archive, &extract_root)?;
    let payload = extract_root.join(top_level);
    let skill_files = find_valid_skill_files(&payload)?;
    if skill_files.is_empty() {
        return Err(validation_error(
            "downloaded repository does not contain a valid SKILL.md",
        ));
    }
    write_install_manifest(&payload, source_ref)?;
    let target = safe_child_directory(skills_root, repo_name)?;
    replace_directory(&payload, &target, stage.path(), source_ref)
}

fn extract_archive(archive: &[u8], extract_root: &Path) -> AppResult<PathBuf> {
    let cursor = Cursor::new(archive);
    let mut zip = zip::ZipArchive::new(cursor)
        .map_err(|error| invalid_config(format!("invalid skill archive: {error}")))?;
    if zip.len() == 0 || zip.len() > MAX_ARCHIVE_ENTRIES {
        return Err(validation_error("skill archive has an invalid entry count"));
    }
    let mut top_level = None;
    let mut total_size = 0_u64;
    for index in 0..zip.len() {
        let mut entry = zip
            .by_index(index)
            .map_err(|error| invalid_config(format!("invalid ZIP entry: {error}")))?;
        let relative = entry
            .enclosed_name()
            .ok_or_else(|| validation_error("skill archive contains an unsafe path"))?
            .to_path_buf();
        validate_archive_path(&relative, &mut top_level)?;
        validate_archive_entry(&entry)?;
        total_size = total_size
            .checked_add(entry.size())
            .ok_or_else(|| validation_error("skill archive size overflow"))?;
        if total_size > MAX_EXTRACTED_BYTES {
            return Err(validation_error(
                "skill archive expands beyond the size limit",
            ));
        }
        let destination = extract_root.join(&relative);
        if entry.is_dir() {
            fs::create_dir_all(&destination)?;
            continue;
        }
        let parent = destination
            .parent()
            .ok_or_else(|| validation_error("ZIP entry has no parent directory"))?;
        fs::create_dir_all(parent)?;
        let mut output = fs::File::create(&destination)?;
        let copied = std::io::copy(
            &mut entry.by_ref().take(MAX_ARCHIVE_ENTRY_BYTES + 1),
            &mut output,
        )?;
        if copied > MAX_ARCHIVE_ENTRY_BYTES {
            return Err(validation_error("skill archive entry is too large"));
        }
        preserve_executable_permissions(&destination, entry.unix_mode())?;
    }
    top_level.ok_or_else(|| validation_error("skill archive is empty"))
}

fn validate_archive_path(relative: &Path, top_level: &mut Option<PathBuf>) -> AppResult<()> {
    if !relative
        .components()
        .all(|component| matches!(component, Component::Normal(_)))
    {
        return Err(validation_error(
            "skill archive contains a non-normalized path",
        ));
    }
    let first = relative
        .components()
        .next()
        .and_then(|component| match component {
            Component::Normal(value) => Some(PathBuf::from(value)),
            _ => None,
        })
        .ok_or_else(|| validation_error("skill archive contains an unsafe path"))?;
    if let Some(existing) = top_level {
        if existing != &first {
            return Err(validation_error(
                "skill archive must contain one top-level directory",
            ));
        }
    } else {
        *top_level = Some(first);
    }
    Ok(())
}

fn validate_archive_entry<R: Read>(entry: &zip::read::ZipFile<'_, R>) -> AppResult<()> {
    if entry.size() > MAX_ARCHIVE_ENTRY_BYTES {
        return Err(validation_error("skill archive entry is too large"));
    }
    if let Some(mode) = entry.unix_mode() {
        let kind = mode & 0o170000;
        if kind != 0 && kind != 0o040000 && kind != 0o100000 {
            return Err(validation_error(
                "skill archive contains a link or special file",
            ));
        }
    }
    Ok(())
}

#[cfg(unix)]
fn preserve_executable_permissions(path: &Path, mode: Option<u32>) -> AppResult<()> {
    use std::os::unix::fs::PermissionsExt;
    if let Some(mode) = mode {
        let permissions = mode & 0o777;
        if permissions != 0 {
            fs::set_permissions(path, fs::Permissions::from_mode(permissions))?;
        }
    }
    Ok(())
}

#[cfg(not(unix))]
fn preserve_executable_permissions(_path: &Path, _mode: Option<u32>) -> AppResult<()> {
    Ok(())
}

fn find_valid_skill_files(root: &Path) -> AppResult<Vec<PathBuf>> {
    let mut files = Vec::new();
    find_valid_skill_files_in(root, 0, &mut files)?;
    Ok(files)
}

fn find_valid_skill_files_in(
    directory: &Path,
    depth: usize,
    files: &mut Vec<PathBuf>,
) -> AppResult<()> {
    if depth > MAX_SCAN_DEPTH {
        return Err(validation_error("skill archive nesting is too deep"));
    }
    let skill = directory.join(ENABLED_FILE);
    if skill.is_file() {
        parse_skill(&read_skill_text(&skill)?)?;
        files.push(skill);
        return Ok(());
    }
    let entries = fs::read_dir(directory)?.collect::<Result<Vec<_>, _>>()?;
    for entry in entries {
        let file_type = entry.file_type()?;
        if file_type.is_dir()
            && !file_type.is_symlink()
            && !entry.file_name().to_string_lossy().starts_with('.')
        {
            find_valid_skill_files_in(&entry.path(), depth + 1, files)?;
        }
    }
    Ok(())
}

fn write_install_manifest(payload: &Path, source_ref: &str) -> AppResult<()> {
    let manifest = InstallManifest {
        source_ref: source_ref.to_string(),
        installed_at: Utc::now().to_rfc3339(),
    };
    let bytes = serde_json::to_vec_pretty(&manifest)?;
    atomic_write(&payload.join(INSTALL_MANIFEST), &bytes, false)
}

fn safe_child_directory(root: &Path, name: &str) -> AppResult<PathBuf> {
    validate_github_component(name, "repository")?;
    let mut components = Path::new(name).components();
    match (components.next(), components.next()) {
        (Some(Component::Normal(_)), None) => Ok(root.join(name)),
        _ => Err(validation_error("invalid skill installation directory")),
    }
}

fn replace_directory(
    payload: &Path,
    target: &Path,
    stage: &Path,
    source_ref: &str,
) -> AppResult<()> {
    let previous = stage.join("previous");
    if target.exists() {
        let metadata = fs::symlink_metadata(target)?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(validation_error(
                "existing skill installation is not a regular directory",
            ));
        }
        validate_managed_installation(target, source_ref)?;
        fs::rename(target, &previous)?;
    }
    if let Err(error) = fs::rename(payload, target) {
        if previous.exists() {
            fs::rename(&previous, target).map_err(|restore_error| {
                AppError::with_details(
                    "atomic_write_failed",
                    format!("failed to install skill: {error}"),
                    format!("failed to restore previous skill: {restore_error}"),
                )
            })?;
        }
        return Err(AppError::new(
            "atomic_write_failed",
            format!("failed to install skill: {error}"),
        ));
    }
    if previous.exists() {
        fs::remove_dir_all(previous)?;
    }
    Ok(())
}

fn validate_managed_installation(target: &Path, source_ref: &str) -> AppResult<()> {
    let manifest_path = target.join(INSTALL_MANIFEST);
    if !manifest_path.is_file() {
        return Err(validation_error(
            "refusing to replace a skill directory not managed by XiaoBaiSwitch",
        ));
    }
    let manifest: InstallManifest = serde_json::from_slice(&fs::read(manifest_path)?)?;
    if normalize_source_ref(&manifest.source_ref) != normalize_source_ref(source_ref) {
        return Err(validation_error(
            "existing skill directory belongs to a different source",
        ));
    }
    Ok(())
}

fn managed_install_root(root: &Path, skill_directory: &Path) -> AppResult<Option<PathBuf>> {
    let mut current = Some(skill_directory);
    while let Some(directory) = current {
        if directory == root {
            break;
        }
        let manifest_path = directory.join(INSTALL_MANIFEST);
        if manifest_path.is_file() {
            let _: InstallManifest = serde_json::from_slice(&fs::read(manifest_path)?)?;
            return Ok(Some(directory.to_path_buf()));
        }
        current = directory.parent();
    }
    Ok(None)
}

fn resolve_uninstall_directory(root: &Path, skill_directory: &Path) -> AppResult<PathBuf> {
    let Some(managed_root) = managed_install_root(root, skill_directory)? else {
        return Ok(skill_directory.to_path_buf());
    };
    if managed_root == skill_directory {
        return Ok(managed_root);
    }
    let mut installed_skills = Vec::new();
    scan_directory(
        SkillTarget::Codex,
        &managed_root,
        &managed_root,
        0,
        &mut installed_skills,
    )?;
    let has_other_skill = installed_skills
        .iter()
        .map(|skill| Path::new(&skill.directory_path))
        .any(|directory| directory != skill_directory);
    Ok(if has_other_skill {
        skill_directory.to_path_buf()
    } else {
        managed_root
    })
}

fn installed_source_targets(
    roots: &[(SkillTarget, PathBuf)],
) -> AppResult<HashMap<String, Vec<SkillTarget>>> {
    let mut installed: HashMap<String, Vec<SkillTarget>> = HashMap::new();
    for (target, root) in roots {
        if !root.is_dir() {
            continue;
        }
        let mut refs = HashSet::new();
        collect_installed_source_refs(root, 0, &mut refs)?;
        for source_ref in refs {
            installed.entry(source_ref).or_default().push(*target);
        }
    }
    Ok(installed)
}

fn collect_installed_source_refs(
    directory: &Path,
    depth: usize,
    refs: &mut HashSet<String>,
) -> AppResult<()> {
    if depth > MAX_SCAN_DEPTH {
        return Err(validation_error("skill directory nesting is too deep"));
    }
    let manifest_path = directory.join(INSTALL_MANIFEST);
    if manifest_path.is_file() {
        let manifest: InstallManifest = serde_json::from_slice(&fs::read(manifest_path)?)?;
        refs.insert(manifest.source_ref.trim().to_ascii_lowercase());
    }
    for entry in fs::read_dir(directory)?.collect::<Result<Vec<_>, _>>()? {
        let file_type = entry.file_type()?;
        if file_type.is_dir()
            && !file_type.is_symlink()
            && !entry.file_name().to_string_lossy().starts_with('.')
        {
            collect_installed_source_refs(&entry.path(), depth + 1, refs)?;
        }
    }
    Ok(())
}

fn skills_sh_search_query(query: &str) -> AppResult<&str> {
    if query.is_empty() {
        return Ok(SKILLS_SH_BROWSE_QUERY);
    }
    if query.chars().count() < SKILLS_SH_MIN_QUERY_CHARS {
        return Err(validation_error(
            "skills.sh search query must be at least 2 characters",
        ));
    }
    Ok(query)
}

fn http_status_error(status: reqwest::StatusCode, body: &str) -> AppError {
    let detail = serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|value| {
            ["error", "message"]
                .into_iter()
                .find_map(|key| value.get(key).and_then(|item| item.as_str()))
                .map(str::to_string)
        });
    AppError::new(
        "network",
        detail.unwrap_or_else(|| format!("HTTP {}", status.as_u16())),
    )
}

async fn request_json(builder: reqwest::RequestBuilder) -> AppResult<serde_json::Value> {
    let response = builder.send().await?;
    let status = response.status();
    let body = response.text().await?;
    if !status.is_success() {
        return Err(http_status_error(status, &body));
    }
    serde_json::from_str(&body).map_err(Into::into)
}

async fn search_skills_sh(
    client: &reqwest::Client,
    query: &str,
    installed: &HashMap<String, Vec<SkillTarget>>,
) -> AppResult<Vec<MarketplaceSkill>> {
    let query = skills_sh_search_query(query)?;
    let body = request_json(
        client
            .get(SKILLS_SH_SEARCH_URL)
            .query(&[("q", query), ("limit", MARKETPLACE_LIMIT)]),
    )
    .await?;
    let items = body
        .get("skills")
        .and_then(|value| value.as_array())
        .ok_or_else(|| invalid_config("skills.sh returned an invalid response"))?;
    Ok(items
        .iter()
        .filter_map(|item| marketplace_from_skills_sh(item, installed))
        .take(20)
        .collect())
}

fn marketplace_from_skills_sh(
    item: &serde_json::Value,
    installed: &HashMap<String, Vec<SkillTarget>>,
) -> Option<MarketplaceSkill> {
    let name = item.get("name")?.as_str()?.trim();
    let repo = item.get("source")?.as_str()?.trim();
    if name.is_empty() || repo.is_empty() {
        return None;
    }
    let normalized = normalize_source_ref(repo);
    Some(MarketplaceSkill {
        name: name.to_string(),
        description: item
            .get("description")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_string(),
        repo: repo.to_string(),
        stars: item
            .get("stars")
            .and_then(|value| value.as_i64())
            .unwrap_or(0),
        installs: item
            .get("installs")
            .and_then(|value| value.as_i64())
            .unwrap_or(0),
        installed_targets: installed.get(&normalized).cloned().unwrap_or_default(),
    })
}

fn github_search_query(query: &str) -> String {
    if query.is_empty() {
        "topic:agent-skill".to_string()
    } else {
        format!("{query} topic:agent-skill")
    }
}

async fn search_github(
    client: &reqwest::Client,
    query: &str,
    installed: &HashMap<String, Vec<SkillTarget>>,
) -> AppResult<Vec<MarketplaceSkill>> {
    let search_query = github_search_query(query);
    let body = request_json(
        client
            .get("https://api.github.com/search/repositories")
            .header("Accept", "application/vnd.github+json")
            .query(&[
                ("q", search_query.as_str()),
                ("sort", "stars"),
                ("per_page", MARKETPLACE_LIMIT),
            ]),
    )
    .await?;
    let items = body
        .get("items")
        .and_then(|value| value.as_array())
        .ok_or_else(|| invalid_config("GitHub returned an invalid search response"))?;
    Ok(items
        .iter()
        .filter_map(|item| marketplace_from_github(item, installed))
        .collect())
}

fn marketplace_from_github(
    item: &serde_json::Value,
    installed: &HashMap<String, Vec<SkillTarget>>,
) -> Option<MarketplaceSkill> {
    let name = item.get("name")?.as_str()?.trim();
    let repo = item.get("full_name")?.as_str()?.trim();
    if name.is_empty() || repo.is_empty() {
        return None;
    }
    Some(MarketplaceSkill {
        name: name.to_string(),
        description: item
            .get("description")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_string(),
        repo: repo.to_string(),
        stars: item
            .get("stargazers_count")
            .and_then(|value| value.as_i64())
            .unwrap_or(0),
        installs: 0,
        installed_targets: installed
            .get(&normalize_source_ref(repo))
            .cloned()
            .unwrap_or_default(),
    })
}

fn normalize_source_ref(source: &str) -> String {
    parse_github_source(source)
        .map(|(owner, repo)| format!("{owner}/{repo}").to_ascii_lowercase())
        .unwrap_or_else(|_| source.trim().trim_end_matches('/').to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    const VALID_SKILL: &str = "---\nname: demo\ndescription: Demo skill\nmetadata:\n  author: XiaoBai\n  version: 1.2.3\n---\n\n# Demo\n";

    fn write_skill(root: &Path, directory: &str) -> PathBuf {
        let skill_dir = root.join(directory);
        fs::create_dir_all(&skill_dir).unwrap();
        let path = skill_dir.join(ENABLED_FILE);
        fs::write(&path, VALID_SKILL).unwrap();
        path
    }

    #[test]
    fn scans_enabled_and_disabled_skills() {
        let temp = tempdir().unwrap();
        let enabled = write_skill(temp.path(), "enabled");
        let disabled_dir = temp.path().join("disabled");
        fs::create_dir(&disabled_dir).unwrap();
        fs::write(disabled_dir.join(DISABLED_FILE), VALID_SKILL).unwrap();

        let skills = scan_target(SkillTarget::Codex, temp.path()).unwrap();

        assert_eq!(skills.len(), 2);
        assert!(skills
            .iter()
            .any(|skill| skill.enabled && skill.source_path == enabled.display().to_string()));
        assert!(skills.iter().any(|skill| !skill.enabled));
    }

    #[test]
    fn validation_rejects_skill_outside_target_root() {
        let root = tempdir().unwrap();
        let outside = tempdir().unwrap();
        let path = write_skill(outside.path(), "demo");

        let error = validate_skill_path(root.path(), &path).unwrap_err();

        assert!(error.to_string().contains("outside"));
    }

    #[test]
    fn toggles_skill_file_reversibly() {
        let root = tempdir().unwrap();
        let enabled = write_skill(root.path(), "demo");

        set_skill_enabled_at(root.path(), &enabled, false).unwrap();
        let disabled = enabled.with_file_name(DISABLED_FILE);
        assert!(disabled.exists());
        assert!(!enabled.exists());

        set_skill_enabled_at(root.path(), &disabled, true).unwrap();
        assert!(enabled.exists());
        assert!(!disabled.exists());
    }

    #[test]
    fn parses_supported_github_sources() {
        assert_eq!(
            parse_github_source("owner/repo").unwrap(),
            ("owner".to_string(), "repo".to_string())
        );
        assert_eq!(
            parse_github_source("https://github.com/owner/repo.git").unwrap(),
            ("owner".to_string(), "repo".to_string())
        );
        assert!(parse_github_source("https://example.com/owner/repo").is_err());
        assert!(parse_github_source("owner/repo/extra").is_err());
    }

    #[test]
    fn refuses_to_replace_unmanaged_skill_directory() {
        let root = tempdir().unwrap();
        let payload = root.path().join("payload");
        let target = root.path().join("target");
        let stage = root.path().join("stage");
        fs::create_dir(&payload).unwrap();
        fs::create_dir(&target).unwrap();
        fs::create_dir(&stage).unwrap();

        let error = replace_directory(&payload, &target, &stage, "owner/repo").unwrap_err();

        assert!(error.to_string().contains("not managed"));
        assert!(target.exists());
        assert!(payload.exists());
    }

    #[test]
    fn nested_uninstall_preserves_a_managed_repo_until_its_last_skill() {
        let root = tempdir().unwrap();
        let managed = root.path().join("repo");
        fs::create_dir(&managed).unwrap();
        write_install_manifest(&managed, "owner/repo").unwrap();
        let first = write_skill(&managed, "first");
        let second = write_skill(&managed, "second");
        let first_dir = first.parent().unwrap();
        let second_dir = second.parent().unwrap();

        assert_eq!(
            resolve_uninstall_directory(root.path(), first_dir).unwrap(),
            first_dir
        );
        fs::remove_dir_all(first_dir).unwrap();
        assert_eq!(
            resolve_uninstall_directory(root.path(), second_dir).unwrap(),
            managed
        );
    }

    #[test]
    fn empty_skills_sh_query_browses_popular_skills_instead_of_400() {
        assert_eq!(skills_sh_search_query("").unwrap(), "skill");
        assert_eq!(skills_sh_search_query("ai").unwrap(), "ai");
        assert!(skills_sh_search_query("a")
            .unwrap_err()
            .to_string()
            .contains("at least 2 characters"));
    }

    #[test]
    fn github_empty_query_stays_a_valid_topic_search() {
        assert_eq!(github_search_query(""), "topic:agent-skill");
        assert_eq!(github_search_query("design"), "design topic:agent-skill");
    }

    #[test]
    fn maps_skills_sh_payload_without_description() {
        let item = serde_json::json!({
            "id": "vercel-labs/skills/find-skills",
            "name": "find-skills",
            "installs": 3168186,
            "source": "vercel-labs/skills"
        });
        let skill = marketplace_from_skills_sh(&item, &HashMap::new()).unwrap();
        assert_eq!(skill.name, "find-skills");
        assert_eq!(skill.repo, "vercel-labs/skills");
        assert_eq!(skill.installs, 3168186);
        assert!(skill.description.is_empty());
    }

    #[test]
    fn surfaces_skills_sh_json_error_instead_of_raw_status() {
        let error = http_status_error(
            reqwest::StatusCode::BAD_REQUEST,
            r#"{"error":"Query must be at least 2 characters"}"#,
        );
        assert_eq!(error.to_string(), "Query must be at least 2 characters");
    }

    #[test]
    fn agents_skills_live_under_dot_agents() {
        let path = home_dir().unwrap().join(".agents").join("skills");
        assert!(path.ends_with(Path::new(".agents/skills")));
        assert_eq!(SkillTarget::ALL[0], SkillTarget::Agents);
        assert_eq!(serde_json::to_string(&SkillTarget::Agents).unwrap(), "\"agents\"");
        assert!(SkillTarget::Agents.sort_key() < SkillTarget::ClaudeCode.sort_key());
    }

    #[test]
    fn github_archive_candidates_prefer_public_zip_over_api() {
        let urls = github_archive_candidates("owner", "repo");
        assert_eq!(urls[0].0, "https://github.com/owner/repo/archive/HEAD.zip");
        assert!(!urls[0].1);
        assert!(urls.last().unwrap().0.contains("api.github.com"));
        assert!(urls.last().unwrap().1);
    }
}
