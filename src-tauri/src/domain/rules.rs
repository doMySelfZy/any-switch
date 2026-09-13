use super::TargetKind;
use serde::{Deserialize, Serialize};

/// 全局约束：用户级 Agent 指令。整段 Markdown 正文 + 生效目标集合。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentRules {
    pub body: String,
    pub targets: Vec<TargetKind>,
    pub updated_at: i64,
}

/// 某个目标上全局约束的实际落点。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentRulesTargetPath {
    pub target: TargetKind,
    pub path: String,
    pub exists: bool,
    /// 仅 Codex 会用到：同目录的 `AGENTS.override.md` 会整体遮蔽我们写入的 `AGENTS.md`。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shadowed_by: Option<String>,
}

/// 单个目标的写入结果。失败必须回传消息，UI 不能静默。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentRulesTargetResult {
    pub target: TargetKind,
    pub ok: bool,
    pub path: String,
    /// 文件内容是否真的变了；未变时不会备份、不会写盘。
    pub changed: bool,
    pub backup_paths: Vec<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentRulesApplyResult {
    pub results: Vec<AgentRulesTargetResult>,
    pub applied_at: i64,
}
