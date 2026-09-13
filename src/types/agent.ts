export interface AgentUpdateStatus {
  kind: string; // "claude_code" | "codex" | "pi" | "prime"
  name: string;
  currentVersion: string | null;
  latestVersion: string | null;
  hasUpdate: boolean;
  lastCheckAt: number | null;
}

export interface BatchUpdateResult {
  successes: string[];
  failures: [string, string][]; // [kind, error]
}
