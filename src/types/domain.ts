/** Model list / probe protocol */
export type SiteProtocol = "openai_compatible" | "anthropic";

/** Only affects Claude Code auth env key name. Codex ignores this. */
export type ClaudeAuthKeyStyle = "anthropic_auth_token" | "anthropic_api_key";

export type TargetKind = "claude_code" | "codex" | "pi";

export type ProxyMode = "system" | "none" | "custom";
export type ProxyProtocol = "http" | "https" | "socks5";

export type ApplyStatus = "applied" | "stale" | "orphan" | "not_applied" | "failed";

/** Kebab capability flags shared by site JSON and xiaobaiswitch:// query keys. */
export type SiteCapabilities = Record<string, boolean>;

export type CodexCapabilitySource = "site" | "custom";

export interface AppError {
  code:
    | "network"
    | "timeout"
    | "unauthorized"
    | "not_found"
    | "invalid_response"
    | "ssl"
    | "atomic_write_failed"
    | "backup_failed"
    | "validation_failed"
    | "lock_busy"
    | "master_key_missing"
    | "invalid_config"
    | "internal"
    | "autostart_failed"
    | "webdav_not_configured"
    | "webdav_auth_failed"
    | "backup_invalid"
    | "restore_pending";
  message: string;
  details?: string | null;
}

export interface Site {
  id: string;
  name: string;
  baseUrl: string;
  baseUrls: string[];
  keyPrefix: string;
  hasKey: boolean;
  protocol: SiteProtocol;
  claudeAuthKeyStyle: ClaudeAuthKeyStyle;
  notes: string | null;
  enabled: boolean;
  sortOrder: number;
  selectedModelId: string | null;
  lastModelFetchAt: number | null;
  lastModelFetchLatencyMs: number | null;
  lastModelFetchError: string | null;
  createdAt: number;
  updatedAt: number;
  capabilities?: SiteCapabilities;
}

export interface SiteModel {
  id: string;
  siteId: string;
  modelId: string;
  displayName: string;
  ownedBy: string | null;
  raw: Record<string, unknown> | null;
  isManual?: boolean;
}

export interface DeepLinkSiteImportInput {
  name: string;
  baseUrls: string[];
  apiKey: string;
  protocol?: SiteProtocol;
  notes?: string | null;
  capabilities?: SiteCapabilities;
}

export interface DeepLinkSiteImportResult {
  site: Site;
  created: boolean;
  updatedKey: boolean;
  reused: boolean;
}

export interface CreateSiteInput {
  name: string;
  baseUrl?: string;
  baseUrls?: string[];
  apiKey: string;
  protocol?: SiteProtocol;
  claudeAuthKeyStyle?: ClaudeAuthKeyStyle;
  notes?: string | null;
  capabilities?: SiteCapabilities;
}

export interface UpdateSiteInput {
  name?: string;
  baseUrl?: string;
  baseUrls?: string[];
  apiKey?: string | null;
  protocol?: SiteProtocol;
  claudeAuthKeyStyle?: ClaudeAuthKeyStyle;
  notes?: string | null;
  enabled?: boolean;
  selectedModelId?: string | null;
  sortOrder?: number;
  capabilities?: SiteCapabilities;
}

export interface FetchModelsResult {
  models: SiteModel[];
  latencyMs: number;
  endpoint: string;
  fetchedAt: number;
}

export type LiveSummary = Record<string, string | null>;

export interface TargetLiveStatus {
  kind: TargetKind;
  installed: boolean;
  version: string | null;
  configPath: string;
  status: ApplyStatus;
  appliedSiteId: string | null;
  appliedSiteName: string | null;
  appliedModelId: string | null;
  providerId: string | null;
  orphan: boolean;
  liveSummary: LiveSummary;
  lastAppliedAt: number | null;
  staleReason: string | null;
}

/** Claude Code effort / thinking level */
export type ClaudeEffortLevel = "low" | "medium" | "high" | "max";

/** Codex reasoning effort in config.toml */
export type CodexReasoningEffort = "minimal" | "low" | "medium" | "high" | "xhigh";

export interface ApplyRequest {
  siteId: string;
  targets: TargetKind[];
  modelId: string;
  claudeAuthKeyStyle?: ClaudeAuthKeyStyle;
  /** Maps Claude Code "opus" alias to a site model id */
  claudeOpusModelId?: string | null;
  /** Maps Claude Code "sonnet" alias to a site model id */
  claudeSonnetModelId?: string | null;
  /** Maps Claude Code "haiku" alias to a site model id */
  claudeHaikuModelId?: string | null;
  claudeEffortLevel?: ClaudeEffortLevel | null;
  /** Write site model list into Codex model catalog for switching */
  codexWriteAllModels?: boolean;
  codexReasoningEffort?: CodexReasoningEffort | null;
  /** Codex: enable remote history compaction for the current provider */
  codexRemoteCompaction?: boolean;
  /** Codex: allow sending local images to the model */
  codexImageUnderstanding?: boolean;
  /** Codex: allow the hosted image generation tool */
  codexImageGeneration?: boolean;
  /** Codex: allow the hosted web_search tool */
  codexWebSearch?: boolean;
  /** `site` reads the site preset; `custom` uses the four bools above. */
  codexCapabilitySource?: CodexCapabilitySource;
  /** Write the site model list into Pi's managed provider. */
  piWriteAllModels?: boolean;
}

export interface ApplyTargetResult {
  target: TargetKind;
  ok: boolean;
  status: ApplyStatus;
  backupPaths: string[];
  message: string;
  liveSummary?: LiveSummary;
  touchedKeys?: string[];
}

export interface ApplyResult {
  siteId: string;
  modelId: string;
  results: ApplyTargetResult[];
  appliedAt: number;
}

export interface AppSettings {
  language: "zh-CN" | "en-US";
  themeMode: "system" | "light" | "dark";
  primaryColor: string;
  autoStart: boolean;
  alwaysOnTop: boolean;
  claudeHomeOverride: string | null;
  codexHomeOverride: string | null;
  piAgentDirOverride: string | null;
  codexEnvInjectMode: "auto" | "shell_rc" | "user_env" | "file_only";
  forceExclusiveClaudeAuthKey: boolean;
  autoCheckUpdate: boolean;
  /** Auto update check interval in minutes. Default 60. */
  updateCheckInterval: number;
  /** Max backup copies kept per target. Default 30. */
  maxBackupCopies: number;
  proxyMode: ProxyMode;
  proxyProtocol: ProxyProtocol;
  proxyHost: string | null;
  proxyPort: number | null;
  routeProbeTtlMinutes: number;
  /** Hide to the menu bar / tray instead of quitting on window close. */
  closeToTray: boolean;
  /** Keep the main window hidden on launch. Disabled when closeToTray is off. */
  startInTray: boolean;
}

export interface WebDavConfigView {
  baseUrl: string;
  username: string;
  remotePath: string;
  acceptInvalidCerts: boolean;
  hasPassword: boolean;
  autoSyncEnabled: boolean;
  syncIntervalMinutes: number;
  maxRemoteBackups: number;
}

export interface SaveWebDavConfigInput {
  baseUrl: string;
  username: string;
  password?: string | null;
  remotePath: string;
  acceptInvalidCerts: boolean;
  autoSyncEnabled: boolean;
  syncIntervalMinutes: number;
  maxRemoteBackups: number;
}

export interface TestWebDavConnectionInput {
  baseUrl: string;
  username: string;
  password?: string | null;
  remotePath: string;
  acceptInvalidCerts: boolean;
}

export interface RemoteBackupInfo {
  fileName: string;
  size: number;
  lastModified: string;
  deviceName: string;
}

export interface LocalBackupInfo {
  fileName: string;
  size: number;
  createdAt: number;
  deviceName: string;
  reason: string | null;
  appVersion: string | null;
  error: string | null;
}

export interface WebDavSyncStatus {
  lastAttemptAt: number | null;
  lastSuccessAt: number | null;
  status: "never" | "running" | "success" | "warning" | "failed";
  error: string | null;
}

export interface BackupOverview {
  latestLocalBackupAt: number | null;
  webdavConfigured: boolean;
  webdavAutoSyncEnabled: boolean;
  webdavSync: WebDavSyncStatus;
  nextScheduledAt: number | null;
}

export interface BackupOperationResult {
  fileName: string;
  localPath: string | null;
  uploaded: boolean;
  warning: string | null;
}

export interface RestoreStartupResult {
  status: "applied" | "failed";
  message: string;
}

export interface SwitchRouteResult {
  site: Site;
  results: ApplyTargetResult[];
}

export interface UrlProbeResult {
  url: string;
  ok: boolean;
  latencyMs: number;
  status?: number | null;
  error?: string | null;
}

export interface ModelProbeResult {
  modelId: string;
  ok: boolean;
  latencyMs: number;
  status?: number | null;
  error?: string | null;
  endpoint: string;
}

export type QuotaProbeStatus = "available" | "unsupported" | "unauthorized" | "error";

export type QuotaSource =
  | "credit_grants"
  | "subscription_usage"
  | "subscription_only"
  | "usage_only"
  | "token_usage";

export interface SiteQuota {
  status: QuotaProbeStatus;
  remainingUsd: number | null;
  usedUsd: number | null;
  totalUsd: number | null;
  unlimited: boolean;
  unit?: string | null;
  expiresAt: number | null;
  source: QuotaSource | null;
  endpoint: string | null;
  fetchedAt: number;
  latencyMs: number;
  error: string | null;
}

export interface HttpBytesResult {
  status: number;
  contentType: string;
  finalUrl: string;
  base64: string;
}

export interface ApplyRecord {
  id: string;
  siteId: string | null;
  siteNameSnapshot: string;
  target: TargetKind;
  modelId: string;
  providerId: string | null;
  status: "success" | "failed" | "rolled_back";
  backupDir: string | null;
  error: string | null;
  appliedAt: number;
}

export interface BackupInfo {
  id: string;
  target: TargetKind;
  dir: string;
  createdAt: number;
  files: string[];
  applyRecordId: string | null;
  siteNameSnapshot: string | null;
  modelId?: string | null;
}

export interface BackupFileInfo {
  name: string;
  path: string;
}

export interface BackupPreview {
  id: string;
  summary: LiveSummary;
  files: BackupFileInfo[];
}

export interface CliToolInfo {
  kind: TargetKind;
  installed: boolean;
  version: string | null;
  path: string | null;
}

export interface AppPaths {
  appDir: string;
  dbPath: string;
  masterKeyPath: string;
  backupsDir: string;
  appBackupsDir: string;
  codexEnvPath: string;
  logsDir: string;
}

export interface UrlWritePreview {
  modelsUrl: string;
  claudeBaseUrl: string;
  codexBaseUrl: string;
}
