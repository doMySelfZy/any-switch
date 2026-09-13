use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentUpdateStatus {
    pub kind: String,
    pub name: String,
    pub current_version: Option<String>,
    pub latest_version: Option<String>,
    pub has_update: bool,
    pub last_check_at: Option<i64>,
}

impl AgentUpdateStatus {
    pub fn new(kind: String, name: String) -> Self {
        Self {
            kind,
            name,
            current_version: None,
            latest_version: None,
            has_update: false,
            last_check_at: None,
        }
    }
}
