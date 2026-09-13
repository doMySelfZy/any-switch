import type { TargetKind } from "./domain";

export type McpKind = "stdio" | "sse" | "http";

export interface McpServerSummary {
  id: string;
  name: string;
  kind: McpKind;
  enabled: boolean;
  targets: TargetKind[];
  createdAt: number;
  updatedAt: number;
}

export interface McpServer {
  id: string;
  name: string;
  kind: McpKind;
  enabled: boolean;
  targets: TargetKind[];
  config: Record<string, unknown>;
  env: Record<string, unknown>;
  headers: Record<string, unknown>;
  createdAt: number;
  updatedAt: number;
}

export interface McpServerInput {
  id?: string;
  name: string;
  kind?: McpKind;
  enabled?: boolean;
  targets?: TargetKind[];
  config?: Record<string, unknown>;
  env?: Record<string, unknown>;
  headers?: Record<string, unknown>;
}

export interface McpApplyTargetResult {
  target: TargetKind;
  ok: boolean;
  backupPaths: string[];
  message: string;
}

export interface McpApplyResult {
  results: McpApplyTargetResult[];
  appliedAt: number;
}

export interface McpSaveResult {
  server: McpServer;
  sweep: McpApplyResult;
}

/** 官方 MCP Registry 条目的安装方式。 */
export type RegistryInstallKind = "package" | "remote";

/** 需要用户自己填的字段（仓库标为必填且没有默认值）。 */
export interface RegistryRequiredField {
  name: string;
  description?: string | null;
  /** 敏感字段（仓库标记 isSecret），用密码框并要求用户确认。 */
  secret: boolean;
  /** 决定填到表单的环境变量还是请求头输入框。 */
  kind: "env" | "header";
}

/** 仓库条目换算出的安装草稿：表单初值 + 还需用户补的必填项。 */
export interface RegistryInstallDraft {
  name: string;
  displayName: string;
  kind: McpKind;
  config: Record<string, unknown>;
  env: Record<string, unknown>;
  headers: Record<string, unknown>;
  requiredFields: RegistryRequiredField[];
  repositoryUrl?: string | null;
}

export interface RegistryCandidate {
  name: string;
  description?: string | null;
  version?: string | null;
  repositoryUrl?: string | null;
  installKinds: RegistryInstallKind[];
  /** 无法安装的条目没有草稿，界面应禁用「安装」。 */
  draft?: RegistryInstallDraft | null;
}

export interface RegistrySearchResult {
  candidates: RegistryCandidate[];
  nextCursor?: string | null;
}
