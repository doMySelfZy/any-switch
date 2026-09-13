use anyhow::{Context, Result};
use std::process::Command;

/// Get NPM package name for the given agent kind
pub fn get_npm_package_name(kind: &str) -> Option<&'static str> {
    match kind {
        "claude_code" => Some("@anthropic-ai/claude-code"),
        "codex" => Some("@gitnexus/codex"),
        "pi" => Some("@picoding/pi"),
        "prime" => Some("@picoding/prime"),
        _ => None,
    }
}

/// Check current installed version and latest version from NPM
pub async fn check_npm_package_version(
    package_name: &str,
) -> Result<(Option<String>, Option<String>)> {
    // Check current installed version
    let current_version = get_current_installed_version(package_name).await;

    // Check latest version from NPM
    let latest_version = get_npm_latest_version(package_name).await?;

    Ok((current_version, Some(latest_version)))
}

/// Get currently installed version using `npm list -g --depth=0`
async fn get_current_installed_version(package_name: &str) -> Option<String> {
    let output = Command::new("npm")
        .args(["list", "-g", "--depth=0", package_name, "--json"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let json_str = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&json_str).ok()?;

    // Parse version from JSON structure
    json.get("dependencies")?
        .get(package_name)?
        .get("version")?
        .as_str()
        .map(|s| s.to_string())
}

/// Get latest version from NPM registry using `npm view`
async fn get_npm_latest_version(package_name: &str) -> Result<String> {
    let output = Command::new("npm")
        .args(["view", package_name, "version"])
        .output()
        .context("Failed to execute npm view command")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("npm view failed: {}", stderr);
    }

    let version = String::from_utf8(output.stdout)
        .context("Invalid UTF-8 in npm output")?
        .trim()
        .to_string();

    if version.is_empty() {
        anyhow::bail!("No version found for package {}", package_name);
    }

    Ok(version)
}

/// Update NPM package globally using `npm install -g`
pub async fn update_npm_package(package_name: &str) -> Result<String> {
    let output = Command::new("npm")
        .args(["install", "-g", package_name])
        .output()
        .context("Failed to execute npm install command")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("npm install failed: {}", stderr);
    }

    // Get the newly installed version
    let new_version = get_npm_latest_version(package_name).await?;

    Ok(new_version)
}
