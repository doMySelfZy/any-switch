use crate::error::AppResult;
use crate::paths::home_dir;
use std::fs;
use std::path::{Path, PathBuf};

const BEGIN: &str = "# >>> AnySwitch >>>";
const END: &str = "# <<< AnySwitch <<<";
/// Managed-block markers written by earlier brand names. They are still
/// recognized so an upgrade replaces the existing block instead of appending a
/// duplicate (which would source the env file twice).
const LEGACY_MARKERS: [(&str, &str); 2] = [
    ("# >>> XiaoBaiSwitch >>>", "# <<< XiaoBaiSwitch <<<"),
    ("# >>> Xiaobai Switch >>>", "# <<< Xiaobai Switch <<<"),
];

pub fn ensure_source_block(env_file: &Path) -> AppResult<String> {
    let home = home_dir()?;
    let candidates = [
        home.join(".zshrc"),
        home.join(".bashrc"),
        home.join(".profile"),
    ];
    let mut updated = Vec::new();
    for rc in candidates {
        // only write if file exists, or create .zshrc on mac/linux as primary
        if rc.exists() || rc.file_name().and_then(|s| s.to_str()) == Some(".zshrc") {
            if upsert_block(&rc, env_file)? {
                updated.push(rc.display().to_string());
            }
        }
    }
    if updated.is_empty() {
        // force create zshrc
        let zshrc = home.join(".zshrc");
        upsert_block(&zshrc, env_file)?;
        updated.push(zshrc.display().to_string());
    }
    Ok(updated.join(", "))
}

fn find_managed_block(content: &str) -> Option<(usize, usize)> {
    let mut markers: Vec<(&str, &str)> = vec![(BEGIN, END)];
    markers.extend(LEGACY_MARKERS.iter().copied());
    for (begin, end) in markers {
        if let (Some(start), Some(end_at)) = (content.find(begin), content.find(end)) {
            if end_at >= start {
                return Some((start, end_at + end.len()));
            }
        }
    }
    None
}

fn upsert_block(rc_path: &PathBuf, env_file: &Path) -> AppResult<bool> {
    let block = format!(
        "{BEGIN}\n# Managed by AnySwitch\n[ -f \"{}\" ] && . \"{}\"\n{END}\n",
        env_file.display(),
        env_file.display()
    );
    let content = if rc_path.exists() {
        fs::read_to_string(rc_path)?
    } else {
        String::new()
    };

    let new_content = if let Some((start, end_pos)) = find_managed_block(&content) {
        let mut s = String::new();
        s.push_str(&content[..start]);
        s.push_str(&block);
        if end_pos < content.len() {
            let rest = content[end_pos..].trim_start_matches('\n');
            if !rest.is_empty() {
                s.push('\n');
                s.push_str(rest);
            }
        }
        s
    } else {
        let mut s = content.clone();
        if !s.is_empty() && !s.ends_with('\n') {
            s.push('\n');
        }
        s.push('\n');
        s.push_str(&block);
        s
    };

    if new_content != content {
        if let Some(parent) = rc_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(rc_path, new_content)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_the_legacy_managed_block_instead_of_duplicating_it() {
        let temp = tempfile::tempdir().unwrap();
        let rc = temp.path().join(".zshrc");
        let env_file = temp.path().join("env").join("codex.env");
        fs::write(
            &rc,
            "# user config\n# >>> XiaoBaiSwitch >>>\n# Managed by XiaoBaiSwitch\n[ -f \"/old/path\" ] && . \"/old/path\"\n# <<< XiaoBaiSwitch <<<\nexport KEEP=1\n",
        )
        .unwrap();

        assert!(upsert_block(&rc, &env_file).unwrap());

        let content = fs::read_to_string(&rc).unwrap();
        assert_eq!(content.matches("# >>> AnySwitch >>>").count(), 1);
        assert!(!content.contains("XiaoBaiSwitch"));
        assert!(content.contains("export KEEP=1"));
        assert!(content.contains("# user config"));
        assert!(content.contains(&env_file.display().to_string()));
    }

    #[test]
    fn appends_the_managed_block_once() {
        let temp = tempfile::tempdir().unwrap();
        let rc = temp.path().join(".zshrc");
        let env_file = temp.path().join("env").join("codex.env");
        fs::write(&rc, "export A=1\n").unwrap();

        assert!(upsert_block(&rc, &env_file).unwrap());
        assert!(!upsert_block(&rc, &env_file).unwrap());

        let content = fs::read_to_string(&rc).unwrap();
        assert_eq!(content.matches("# >>> AnySwitch >>>").count(), 1);
    }
}
