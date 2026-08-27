import type {
  AppPaths,
  AppSettings,
  ApplyResult,
  BackupOperationResult,
  BackupOverview,
  BackupInfo,
  BackupPreview,
  CliToolInfo,
  CreateSiteInput,
  DeepLinkSiteImportInput,
  DeepLinkSiteImportResult,
  FetchModelsResult,
  HttpBytesResult,
  ModelProbeResult,
  RemoteBackupInfo,
  Site,
  SiteQuota,
  SiteModel,
  SwitchRouteResult,
  TargetKind,
  TargetLiveStatus,
  UpdateSiteInput,
  UrlProbeResult,
  WebDavConfigView,
} from "@/types/domain";
import { keyPrefix, normalizeBaseUrl } from "./urlNormalize";

const DEFAULT_SETTINGS: AppSettings = {
  language: "zh-CN",
  themeMode: "system",
  primaryColor: "#1677ff",
  autoStart: false,
  alwaysOnTop: false,
  claudeHomeOverride: null,
  codexHomeOverride: null,
  piAgentDirOverride: null,
  codexEnvInjectMode: "auto",
  forceExclusiveClaudeAuthKey: false,
  autoCheckUpdate: true,
  updateCheckInterval: 60,
  maxBackupCopies: 30,
  proxyMode: "system",
  proxyProtocol: "http",
  proxyHost: null,
  proxyPort: null,
  routeProbeTtlMinutes: 10,
  closeToTray: true,
  startInTray: false,
};

function defaultTargetStatuses(): TargetLiveStatus[] {
  return [
    {
      kind: "claude_code",
      installed: false,
      version: null,
      configPath: "~/.claude/settings.json",
      status: "not_applied",
      appliedSiteId: null,
      appliedSiteName: null,
      appliedModelId: null,
      providerId: null,
      orphan: false,
      liveSummary: {},
      lastAppliedAt: null,
      staleReason: null,
    },
    {
      kind: "codex",
      installed: false,
      version: null,
      configPath: "~/.codex/config.toml",
      status: "not_applied",
      appliedSiteId: null,
      appliedSiteName: null,
      appliedModelId: null,
      providerId: null,
      orphan: false,
      liveSummary: {},
      lastAppliedAt: null,
      staleReason: null,
    },
    {
      kind: "pi",
      installed: false,
      version: null,
      configPath: "~/.pi/agent/models.json",
      status: "not_applied",
      appliedSiteId: null,
      appliedSiteName: null,
      appliedModelId: null,
      providerId: null,
      orphan: false,
      liveSummary: {},
      lastAppliedAt: null,
      staleReason: null,
    },
  ];
}

let settings: AppSettings = { ...DEFAULT_SETTINGS };
let sites: Site[] = [];
let backups: BackupInfo[] = [];
let targetStatuses: TargetLiveStatus[] = defaultTargetStatuses();
let webdavConfig: WebDavConfigView = {
  baseUrl: "",
  username: "",
  remotePath: "xiaobai-switch",
  acceptInvalidCerts: false,
  hasPassword: false,
  autoSyncEnabled: false,
  syncIntervalMinutes: 60,
  maxRemoteBackups: 10,
};
let remoteBackups: RemoteBackupInfo[] = [];
let latestLocalBackupAt: number | null = null;
let webdavLastAttemptAt: number | null = null;
let webdavLastSuccessAt: number | null = null;
const models = new Map<string, SiteModel[]>();
const keys = new Map<string, string>();
const exclusions = new Map<string, Set<string>>();

export function resetBrowserMock() {
  settings = { ...DEFAULT_SETTINGS };
  sites = [];
  backups = [];
  targetStatuses = defaultTargetStatuses();
  webdavConfig = {
    baseUrl: "",
    username: "",
    remotePath: "xiaobai-switch",
    acceptInvalidCerts: false,
    hasPassword: false,
    autoSyncEnabled: false,
    syncIntervalMinutes: 60,
    maxRemoteBackups: 10,
  };
  remoteBackups = [];
  latestLocalBackupAt = null;
  webdavLastAttemptAt = null;
  webdavLastSuccessAt = null;
  models.clear();
  keys.clear();
  exclusions.clear();
}

export function seedTargetStatuses(items: TargetLiveStatus[]) {
  targetStatuses = items;
}

export function seedBackups(items: BackupInfo[]) {
  backups = items;
}

export function seedWebDavMock(
  config: Partial<WebDavConfigView>,
  items: RemoteBackupInfo[] = [],
) {
  webdavConfig = { ...webdavConfig, ...config };
  remoteBackups = items;
}

function now() {
  return Date.now();
}

function uid() {
  return crypto.randomUUID();
}

export async function handleBrowserCommand<T>(
  cmd: string,
  args?: Record<string, unknown>,
): Promise<T> {
  switch (cmd) {
    case "get_settings":
      return settings as T;
    case "save_settings": {
      const partial = (args?.partial ?? {}) as Partial<AppSettings>;
      settings = { ...settings, ...partial };
      if (!settings.closeToTray) settings.startInTray = false;
      return settings as T;
    }
    case "restore_main_window":
    case "force_quit":
    case "refresh_tray_menu":
      return undefined as T;
    case "list_sites":
      return sites as T;
    case "get_site": {
      const site = sites.find((s) => s.id === args?.id);
      if (!site) throw { code: "not_found", message: "Site not found" };
      return site as T;
    }
    case "get_site_api_key": {
      const id = args?.id as string;
      if (!sites.some((site) => site.id === id)) {
        throw { code: "not_found", message: "Site not found" };
      }
      return (keys.get(id) ?? "") as T;
    }
    case "create_site": {
      const input = args?.input as CreateSiteInput;
      const id = uid();
      const t = now();
      keys.set(id, input.apiKey);
      const urls =
        input.baseUrls && input.baseUrls.length > 0
          ? input.baseUrls
          : [input.baseUrl ?? ""];
      const site: Site = {
        id,
        name: input.name,
        baseUrl: urls[0] ?? "",
        baseUrls: urls,
        keyPrefix: keyPrefix(input.apiKey),
        hasKey: true,
        protocol: input.protocol ?? "openai_compatible",
        claudeAuthKeyStyle: input.claudeAuthKeyStyle ?? "anthropic_auth_token",
        notes: input.notes ?? null,
        enabled: true,
        sortOrder: sites.length,
        selectedModelId: null,
        lastModelFetchAt: null,
        lastModelFetchLatencyMs: null,
        lastModelFetchError: null,
        createdAt: t,
        updatedAt: t,
        capabilities: input.capabilities ?? {},
      };
      sites = [...sites, site];
      return site as T;
    }
    case "import_site_from_deep_link": {
      const input = args?.input as DeepLinkSiteImportInput;
      const name = input?.name?.trim() ?? "";
      const apiKey = input?.apiKey?.trim() ?? "";
      if (!name) throw { code: "validation_failed", message: "site name is required" };
      if (!apiKey) throw { code: "validation_failed", message: "API key is required" };
      const protocol = input.protocol === "anthropic" ? "anthropic" : "openai_compatible";
      const urls =
        input.baseUrls && input.baseUrls.length > 0
          ? input.baseUrls
          : [];
      if (urls.length === 0) {
        throw { code: "validation_failed", message: "at least one base URL is required" };
      }
      const sameSet = (a: string[], b: string[]) => {
        if (a.length !== b.length) return false;
        const sa = [...a].sort();
        const sb = [...b].sort();
        return sa.every((v, i) => v === sb[i]);
      };
      const existing = sites.find(
        (s) => s.protocol === protocol && sameSet(s.baseUrls ?? [s.baseUrl], urls),
      );
      if (existing) {
        const sameKey = keys.get(existing.id) === apiKey;
        const updated: Site = {
          ...existing,
          name,
          notes: input.notes !== undefined ? input.notes : existing.notes,
          keyPrefix: sameKey ? existing.keyPrefix : keyPrefix(apiKey),
          capabilities:
            input.capabilities !== undefined ? input.capabilities : existing.capabilities,
          updatedAt: now(),
        };
        if (!sameKey) keys.set(existing.id, apiKey);
        sites = sites.map((s) => (s.id === existing.id ? updated : s));
        const result: DeepLinkSiteImportResult = {
          site: updated,
          created: false,
          updatedKey: !sameKey,
          reused: sameKey,
        };
        return result as T;
      }
      const created = await handleBrowserCommand<Site>("create_site", {
        input: {
          name,
          baseUrls: urls,
          baseUrl: urls[0],
          apiKey,
          protocol,
          notes: input.notes ?? null,
          capabilities: input.capabilities,
        } satisfies CreateSiteInput,
      });
      const result: DeepLinkSiteImportResult = {
        site: created,
        created: true,
        updatedKey: false,
        reused: false,
      };
      return result as T;
    }
    case "update_site": {
      const id = args?.id as string;
      const input = (args?.input ?? {}) as UpdateSiteInput;
      sites = sites.map((s) => {
        if (s.id !== id) return s;
        if (input.apiKey) {
          keys.set(id, input.apiKey);
        }
        let baseUrls = s.baseUrls?.length ? s.baseUrls : [s.baseUrl];
        let baseUrl = s.baseUrl;
        if (input.baseUrls && input.baseUrls.length > 0) {
          baseUrls = input.baseUrls;
          baseUrl = baseUrls[0];
        } else if (input.baseUrl) {
          if (baseUrls.includes(input.baseUrl)) {
            baseUrls = [input.baseUrl, ...baseUrls.filter((u) => u !== input.baseUrl)];
          } else {
            baseUrls = [input.baseUrl, ...baseUrls.slice(1)];
          }
          baseUrl = input.baseUrl;
        }
        return {
          ...s,
          name: input.name ?? s.name,
          baseUrl,
          baseUrls,
          keyPrefix: input.apiKey ? keyPrefix(input.apiKey) : s.keyPrefix,
          protocol: input.protocol ?? s.protocol,
          claudeAuthKeyStyle: input.claudeAuthKeyStyle ?? s.claudeAuthKeyStyle,
          notes: input.notes !== undefined ? input.notes : s.notes,
          enabled: input.enabled ?? s.enabled,
          selectedModelId:
            input.selectedModelId !== undefined ? input.selectedModelId : s.selectedModelId,
          sortOrder: input.sortOrder ?? s.sortOrder,
          capabilities: input.capabilities !== undefined ? input.capabilities : s.capabilities,
          updatedAt: now(),
        };
      });
      const site = sites.find((s) => s.id === id);
      if (!site) throw { code: "not_found", message: "Site not found" };
      return site as T;
    }
    case "delete_site": {
      const id = args?.id as string;
      sites = sites.filter((s) => s.id !== id);
      models.delete(id);
      keys.delete(id);
      exclusions.delete(id);
      return undefined as T;
    }
    case "reorder_sites": {
      const ids = args?.ids as string[];
      sites = ids
        .map((id, i) => {
          const s = sites.find((x) => x.id === id);
          return s ? { ...s, sortOrder: i } : null;
        })
        .filter(Boolean) as Site[];
      return undefined as T;
    }
    case "fetch_site_models": {
      const siteId = args?.siteId as string;
      const site = sites.find((s) => s.id === siteId);
      if (!site) throw { code: "not_found", message: "Site not found" };
      const sample: SiteModel[] = [
        {
          id: uid(),
          siteId,
          modelId: "gpt-4.1",
          displayName: "gpt-4.1",
          ownedBy: "mock",
          raw: null,
          isManual: false,
        },
        {
          id: uid(),
          siteId,
          modelId: "claude-sonnet-4",
          displayName: "claude-sonnet-4",
          ownedBy: "mock",
          raw: null,
          isManual: false,
        },
      ];
      const existing = models.get(siteId) ?? [];
      const hidden = exclusions.get(siteId) ?? new Set<string>();
      const visibleSample = sample.filter((m) => !hidden.has(m.modelId));
      const fetchedIds = new Set(visibleSample.map((m) => m.modelId));
      const manuals = existing.filter((m) => m.isManual && !fetchedIds.has(m.modelId) && !hidden.has(m.modelId));
      const merged = [...visibleSample, ...manuals];
      models.set(siteId, merged);
      sites = sites.map((s) =>
        s.id === siteId
          ? {
              ...s,
              lastModelFetchAt: now(),
              lastModelFetchLatencyMs: 42,
              lastModelFetchError: null,
              updatedAt: now(),
            }
          : s,
      );
      const result: FetchModelsResult = {
        models: merged,
        latencyMs: 42,
        endpoint: `${site.baseUrl}/v1/models`,
        fetchedAt: now(),
      };
      return result as T;
    }
    case "list_site_models":
      return (models.get(args?.siteId as string) ?? []) as T;
    case "set_selected_model": {
      const siteId = args?.siteId as string;
      const modelId = args?.modelId as string;
      exclusions.get(siteId)?.delete(modelId);
      sites = sites.map((s) =>
        s.id === siteId ? { ...s, selectedModelId: modelId, updatedAt: now() } : s,
      );
      const list = models.get(siteId) ?? [];
      if (!list.some((m) => m.modelId === modelId)) {
        models.set(siteId, [
          ...list,
          {
            id: uid(),
            siteId,
            modelId,
            displayName: modelId,
            ownedBy: null,
            raw: null,
            isManual: true,
          },
        ]);
      }
      return undefined as T;
    }
    case "clear_site_models": {
      const siteId = args?.siteId as string;
      const site = sites.find((s) => s.id === siteId);
      if (!site) throw { code: "not_found", message: "Site not found" };
      models.set(siteId, []);
      site.selectedModelId = null;
      site.updatedAt = now();
      return site as T;
    }
    case "delete_site_model": {
      const siteId = args?.siteId as string;
      const modelId = args?.modelId as string;
      const list = (models.get(siteId) ?? []).filter((m) => m.modelId !== modelId);
      if (list.length === (models.get(siteId) ?? []).length) {
        throw { code: "not_found", message: "model not found" };
      }
      const hidden = exclusions.get(siteId) ?? new Set<string>();
      hidden.add(modelId);
      exclusions.set(siteId, hidden);
      models.set(siteId, list);
      const site = sites.find((s) => s.id === siteId);
      if (!site) throw { code: "not_found", message: "Site not found" };
      if (site.selectedModelId === modelId) {
        site.selectedModelId = list[0]?.modelId ?? null;
        site.updatedAt = now();
      }
      return site as T;
    }
    case "list_target_status": {
      return targetStatuses as T;
    }
    case "apply_site": {
      const siteId = args?.siteId as string;
      const modelId = args?.modelId as string;
      const site = sites.find((s) => s.id === siteId);
      const targets = ((args?.targets as string[]) ?? []) as TargetKind[];
      const appliedAt = now();
      const piWriteAllModels = Boolean(args?.piWriteAllModels);
      const piModelCount = piWriteAllModels ? Math.max(models.get(siteId)?.length ?? 0, 1) : 1;
      targetStatuses = targetStatuses.map((row) =>
        targets.includes(row.kind)
          ? {
              ...row,
              status: "applied",
              appliedSiteId: siteId,
              appliedSiteName: site?.name ?? null,
              appliedModelId: modelId,
              providerId: row.kind === "pi" ? `xiaobai_${siteId.slice(0, 8)}` : row.providerId,
              liveSummary:
                row.kind === "pi"
                  ? {
                      defaultProvider: `xiaobai_${siteId.slice(0, 8)}`,
                      defaultModel: modelId,
                      modelCount: String(piModelCount),
                      writeAllModels: String(piWriteAllModels),
                    }
                  : row.liveSummary,
              lastAppliedAt: appliedAt,
            }
          : row,
      );
      const result: ApplyResult = {
        siteId,
        modelId,
        results: targets.map((t) => ({
          target: t,
          ok: true,
          status: "applied",
          backupPaths: [],
          message: "Browser mock: apply simulated (no filesystem writes)",
        })),
        appliedAt,
      };
      return result as T;
    }
    case "revert_target":
    case "restore_official_target": {
      const target = args?.target as TargetKind;
      targetStatuses = targetStatuses.map((row) =>
        row.kind === target
          ? {
              ...row,
              status: "not_applied",
              appliedSiteId: null,
              appliedSiteName: null,
              appliedModelId: null,
              lastAppliedAt: null,
              liveSummary: {},
              orphan: false,
              staleReason: null,
            }
          : row,
      );
      return undefined as T;
    }
    case "cleanup_orphan_target":
      return undefined as T;
    case "list_apply_records":
      return [] as T;
    case "list_backups": {
      const target = args?.target as TargetKind | undefined;
      const list = target ? backups.filter((b) => b.target === target) : backups;
      return list as T;
    }
    case "preview_backup": {
      const id = String(args?.id ?? "");
      const b = backups.find((x) => x.id === id);
      const preview: BackupPreview = {
        id,
        summary: {
          ANTHROPIC_MODEL: b?.modelId ?? "gpt-5.6",
          ANTHROPIC_BASE_URL: "https://api.example.com",
        },
        files: (b?.files ?? []).map((name) => ({
          name,
          path: `${b?.dir ?? ""}/${name}`,
        })),
      };
      return preview as T;
    }
    case "delete_backup": {
      const id = String(args?.id ?? "");
      backups = backups.filter((b) => b.id !== id);
      return undefined as T;
    }
    case "restore_backup":
      return undefined as T;
    case "get_webdav_config":
      return webdavConfig as T;
    case "save_webdav_config": {
      const input = args?.input as {
        baseUrl: string;
        username: string;
        password?: string | null;
        remotePath: string;
        acceptInvalidCerts: boolean;
        autoSyncEnabled: boolean;
        syncIntervalMinutes: number;
        maxRemoteBackups: number;
      };
      webdavConfig = {
        baseUrl: input.baseUrl,
        username: input.username,
        remotePath: input.remotePath,
        acceptInvalidCerts: input.acceptInvalidCerts,
        hasPassword: webdavConfig.hasPassword || Boolean(input.password),
        autoSyncEnabled: input.autoSyncEnabled,
        syncIntervalMinutes: input.syncIntervalMinutes,
        maxRemoteBackups: input.maxRemoteBackups,
      };
      return webdavConfig as T;
    }
    case "test_webdav_connection":
      return undefined as T;
    case "create_app_backup": {
      const destination = String(args?.destination ?? "");
      const createdAt = now();
      const fileName = `xiaobai-switch-backup-20260827_120000.browser.12345678.zip`;
      if (destination === "local") latestLocalBackupAt = createdAt;
      if (destination === "webdav") {
        if (!webdavConfig.baseUrl) {
          throw { code: "webdav_not_configured", message: "WebDAV is not configured" };
        }
        webdavLastAttemptAt = createdAt;
        webdavLastSuccessAt = createdAt;
        remoteBackups = [
          {
            fileName,
            size: 1024,
            lastModified: new Date(createdAt).toUTCString(),
            deviceName: "browser",
          },
          ...remoteBackups,
        ];
      }
      const result: BackupOperationResult = {
        fileName,
        localPath:
          destination === "local" ? `~/.xiaobai-switch/backups/app/${fileName}` : null,
        uploaded: destination === "webdav",
        warning: null,
      };
      return result as T;
    }
    case "get_backup_overview": {
      const overview: BackupOverview = {
        latestLocalBackupAt,
        webdavConfigured: Boolean(webdavConfig.baseUrl),
        webdavAutoSyncEnabled: webdavConfig.autoSyncEnabled,
        webdavSync: {
          lastAttemptAt: webdavLastAttemptAt,
          lastSuccessAt: webdavLastSuccessAt,
          status: webdavLastSuccessAt ? "success" : "never",
          error: null,
        },
        nextScheduledAt:
          webdavConfig.autoSyncEnabled && webdavLastAttemptAt
            ? webdavLastAttemptAt + webdavConfig.syncIntervalMinutes * 60_000
            : null,
      };
      return overview as T;
    }
    case "list_webdav_backups":
      return remoteBackups as T;
    case "delete_webdav_backup":
      remoteBackups = remoteBackups.filter((backup) => backup.fileName !== args?.fileName);
      return undefined as T;
    case "restore_webdav_backup":
      return undefined as T;
    case "take_restore_result":
      return null as T;
    case "detect_cli_tools": {
      const tools: CliToolInfo[] = [
        { kind: "claude_code", installed: false, version: null, path: null },
        { kind: "codex", installed: false, version: null, path: null },
        { kind: "pi", installed: false, version: null, path: null },
      ];
      return tools as T;
    }
    case "get_app_paths": {
      const paths: AppPaths = {
        appDir: "~/.xiaobai-switch",
        dbPath: "~/.xiaobai-switch/xiaobai-switch.db",
        masterKeyPath: "~/.xiaobai-switch/master.key",
        backupsDir: "~/.xiaobai-switch/backups",
        codexEnvPath: "~/.xiaobai-switch/env/codex.env",
        logsDir: "~/.xiaobai-switch/logs",
      };
      return paths as T;
    }
    case "sync_windows_chrome":
    case "set_always_on_top":
    case "minimize_window":
    case "toggle_maximize_window":
    case "open_path":
    case "open_url":
      return undefined as T;
    case "take_pending_deep_link":
      return null as T;
    case "fetch_http_text": {
      const url = String(args?.url ?? "");
      return {
        status: 0,
        contentType: "",
        finalUrl: url,
        body: "",
      } as T;
    }
    case "fetch_http_bytes": {
      const url = String(args?.url ?? "");
      const empty: HttpBytesResult = {
        status: 0,
        contentType: "",
        finalUrl: url,
        base64: "",
      };
      return empty as T;
    }
    case "resolve_http_proxy":
      return null as T;
    case "check_app_update":
      return null as T;
    case "probe_urls": {
      const urls = (args?.urls as string[]) ?? [];
      const results: UrlProbeResult[] = urls.map((url, i) => ({
        url,
        ok: true,
        latencyMs: [80, 1500, 4000][i % 3] ?? 80,
        status: 200,
        error: null,
      }));
      return results as T;
    }
    case "probe_site_quota": {
      const siteId = String(args?.siteId ?? "");
      const site = sites.find((s) => s.id === siteId);
      if (!site) throw { code: "not_found", message: "Site not found" };
      if (!site.hasKey || /no-quota/i.test(site.baseUrl) || /no-quota/i.test(site.name)) {
        const unsupported: SiteQuota = {
          status: "unsupported",
          remainingUsd: null,
          usedUsd: null,
          totalUsd: null,
          unlimited: false,
          expiresAt: null,
          source: null,
          endpoint: null,
          fetchedAt: now(),
          latencyMs: 4,
          error: null,
        };
        return unsupported as T;
      }
      const result: SiteQuota = {
        status: "available",
        remainingUsd: 87.5,
        usedUsd: 12.5,
        totalUsd: 100,
        unlimited: false,
        unit: "USD",
        expiresAt: Math.floor(Date.UTC(2026, 11, 31) / 1000),
        source: "credit_grants",
        endpoint: `${normalizeBaseUrl(site.baseUrl).codexBaseUrl}/dashboard/billing/credit_grants`,
        fetchedAt: now(),
        latencyMs: 12,
        error: null,
      };
      return result as T;
    }
    case "probe_site_model": {
      const siteId = args?.siteId as string;
      const modelId = String(args?.modelId ?? "").trim();
      const site = sites.find((s) => s.id === siteId);
      if (!site) throw { code: "not_found", message: "Site not found" };
      if (!modelId) throw { code: "validation_failed", message: "model id required" };
      await new Promise((r) => setTimeout(r, 5));
      const endpoint = `${normalizeBaseUrl(site.baseUrl).codexBaseUrl}/chat/completions`;
      if (/fail-long/i.test(modelId)) {
        const result: ModelProbeResult = {
          modelId,
          ok: false,
          latencyMs: 8,
          status: 400,
          error:
            "mock upstream rejected this model because the requested identifier is not available on this gateway and the provider returned a very long diagnostic payload",
          endpoint,
        };
        return result as T;
      }
      if (/fail/i.test(modelId)) {
        const result: ModelProbeResult = {
          modelId,
          ok: false,
          latencyMs: 8,
          status: 400,
          error: "mock upstream rejected this model",
          endpoint,
        };
        return result as T;
      }
      const result: ModelProbeResult = {
        modelId,
        ok: true,
        latencyMs: 12,
        status: 200,
        error: null,
        endpoint,
      };
      return result as T;
    }
    case "switch_site_route": {
      const siteId = args?.siteId as string;
      const baseUrl = String(args?.baseUrl ?? "").trim();
      const current = sites.find((s) => s.id === siteId);
      if (!current) throw { code: "not_found", message: "Site not found" };
      const urls = current.baseUrls?.length ? current.baseUrls : [current.baseUrl];
      if (!urls.includes(baseUrl)) {
        throw { code: "validation_failed", message: "base URL is not a configured route" };
      }
      const next = [baseUrl, ...urls.filter((u) => u !== baseUrl)];
      sites = sites.map((s) =>
        s.id === siteId ? { ...s, baseUrl, baseUrls: next, updatedAt: now() } : s,
      );
      const site = sites.find((s) => s.id === siteId)!;
      const result: SwitchRouteResult = { site, results: [] };
      return result as T;
    }
    case "preview_urls":
      return normalizeBaseUrl(String(args?.baseUrl ?? "")) as T;
    default:
      throw {
        code: "internal",
        message: `Unknown command in browser mock: ${cmd}`,
      };
  }
}
