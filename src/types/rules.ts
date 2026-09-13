import type { TargetKind } from "./domain";

/** 全局约束：整段 Markdown 正文 + 生效目标集合。 */
export interface AgentRules {
  body: string;
  targets: TargetKind[];
  updatedAt: number;
}

/** 某个目标上全局约束的实际落点。 */
export interface AgentRulesTargetPath {
  target: TargetKind;
  path: string;
  exists: boolean;
  /** 仅 Codex：同目录的 AGENTS.override.md 会整体遮蔽我们写入的内容。 */
  shadowedBy?: string;
}

/** 单个目标的写入结果；失败时 message 必须有内容。 */
export interface AgentRulesTargetResult {
  target: TargetKind;
  ok: boolean;
  path: string;
  changed: boolean;
  backupPaths: string[];
  message: string;
}

export interface AgentRulesApplyResult {
  results: AgentRulesTargetResult[];
  appliedAt: number;
}
