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
