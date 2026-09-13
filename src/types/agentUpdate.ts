export interface AgentUpdateStatus {
  kind: string;
  name: string;
  currentVersion: string | null;
  latestVersion: string | null;
  hasUpdate: boolean;
  lastCheckAt: number | null;
}

export interface BatchUpdateResult {
  successes: string[];
  failures: [string, string][];
}
