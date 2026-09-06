pub mod atomic;
pub mod claude_code;
pub mod codex;
pub mod pi;
pub mod prime;
pub mod thinking;

use std::collections::HashMap;

pub struct RewriteOutcome {
    pub backup_paths: Vec<String>,
    pub live_summary: HashMap<String, Option<String>>,
    pub expected_fields: HashMap<String, String>,
    pub message: String,
}

pub struct RestoreOfficialOutcome {
    #[allow(dead_code)]
    pub backup_paths: Vec<String>,
    pub env_keys: Vec<String>,
}
