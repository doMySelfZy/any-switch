import type {
  ClaudeAuthKeyStyle,
  ClaudeEffortLevel,
  CodexCapabilitySource,
  CodexReasoningEffort,
  LiveSummary,
  Site,
  SiteModel,
  TargetLiveStatus,
} from "@/types/domain";
import {
  type CodexCapabilityFlags,
  EMPTY_CODEX_FLAGS,
  codexFlagsFromCapabilities,
} from "@/lib/siteCapabilities";

const CLAUDE_EFFORTS = new Set<ClaudeEffortLevel>(["low", "medium", "high", "max"]);
const CODEX_EFFORTS = new Set<CodexReasoningEffort>([
  "minimal",
  "low",
  "medium",
  "high",
  "xhigh",
]);

export function selectableApplySites<T extends { enabled: boolean }>(sites: T[]): T[] {
  return sites.filter((s) => s.enabled);
}

export interface PickApplySiteIdInput {
  sites: Pick<Site, "id">[];
  prefillSiteId?: string | null;
  selectedSiteId?: string | null;
  appliedSiteId?: string | null;
}

export function pickApplySiteId({
  sites,
  prefillSiteId,
  selectedSiteId,
  appliedSiteId,
}: PickApplySiteIdInput): string | null {
  const ids = new Set(sites.map((s) => s.id));
  if (prefillSiteId && ids.has(prefillSiteId)) return prefillSiteId;
  if (appliedSiteId && ids.has(appliedSiteId)) return appliedSiteId;
  if (selectedSiteId && ids.has(selectedSiteId)) return selectedSiteId;
  return sites[0]?.id ?? null;
}

export function liveStr(summary: LiveSummary | undefined, ...keys: string[]): string | undefined {
  if (!summary) return undefined;
  for (const key of keys) {
    const value = summary[key];
    if (typeof value === "string" && value.length > 0) return value;
  }
  return undefined;
}

export function parseClaudeEffort(raw: string | undefined): ClaudeEffortLevel | undefined {
  if (!raw) return undefined;
  return CLAUDE_EFFORTS.has(raw as ClaudeEffortLevel) ? (raw as ClaudeEffortLevel) : undefined;
}

export function parseCodexReasoning(raw: string | undefined): CodexReasoningEffort | undefined {
  if (!raw) return undefined;
  return CODEX_EFFORTS.has(raw as CodexReasoningEffort)
    ? (raw as CodexReasoningEffort)
    : undefined;
}

export function inferClaudeAuth(summary: LiveSummary | undefined): ClaudeAuthKeyStyle | undefined {
  if (!summary) return undefined;
  if (liveStr(summary, "ANTHROPIC_AUTH_TOKEN")) return "anthropic_auth_token";
  if (liveStr(summary, "ANTHROPIC_API_KEY")) return "anthropic_api_key";
  return undefined;
}

export interface ClaudeFormDefaults {
  modelId: string | undefined;
  opusModel: string | undefined;
  sonnetModel: string | undefined;
  haikuModel: string | undefined;
  effort: ClaudeEffortLevel | undefined;
  auth: ClaudeAuthKeyStyle;
}

export interface CodexFormDefaults {
  modelId: string | undefined;
  writeAllModels: boolean;
  reasoning: CodexReasoningEffort | undefined;
  capabilitySource: CodexCapabilitySource;
  remoteCompaction: boolean;
  imageUnderstanding: boolean;
  imageGeneration: boolean;
  webSearch: boolean;
}

export interface PiFormDefaults {
  modelId: string | undefined;
  writeAllModels: boolean;
}

function appliedOnSite(site: Site | null, status: TargetLiveStatus | undefined): boolean {
  return Boolean(site && status?.appliedSiteId && status.appliedSiteId === site.id);
}

export function hydrateClaudeForm(
  site: Site | null,
  status: TargetLiveStatus | undefined,
): ClaudeFormDefaults {
  const live = status?.liveSummary;
  const onSite = appliedOnSite(site, status);
  const liveModel = liveStr(live, "ANTHROPIC_MODEL", "model") ?? status?.appliedModelId ?? undefined;
  const fallbackAuth = site?.claudeAuthKeyStyle ?? "anthropic_auth_token";

  if (onSite) {
    return {
      modelId: liveModel ?? site?.selectedModelId ?? undefined,
      opusModel: liveStr(live, "ANTHROPIC_DEFAULT_OPUS_MODEL"),
      sonnetModel: liveStr(live, "ANTHROPIC_DEFAULT_SONNET_MODEL"),
      haikuModel: liveStr(live, "ANTHROPIC_DEFAULT_HAIKU_MODEL"),
      effort: parseClaudeEffort(liveStr(live, "CLAUDE_CODE_EFFORT_LEVEL", "effortLevel")),
      auth: inferClaudeAuth(live) ?? fallbackAuth,
    };
  }

  return {
    modelId: site?.selectedModelId ?? undefined,
    opusModel: undefined,
    sonnetModel: undefined,
    haikuModel: undefined,
    effort: parseClaudeEffort(liveStr(live, "CLAUDE_CODE_EFFORT_LEVEL", "effortLevel")),
    auth: site?.claudeAuthKeyStyle ?? fallbackAuth,
  };
}

function liveTruthy(summary: LiveSummary | undefined, ...keys: string[]): boolean {
  const raw = liveStr(summary, ...keys);
  if (!raw) return false;
  const normalized = raw.trim().toLowerCase();
  return normalized === "1" || normalized === "true" || normalized === "on";
}

function hydrateRemoteCompaction(live: LiveSummary | undefined): boolean {
  if (liveTruthy(live, "remote_compaction")) return true;
  return liveStr(live, "provider_display_name") === "OpenAI";
}

function hydrateWebSearch(live: LiveSummary | undefined): boolean {
  const raw = liveStr(live, "web_search");
  if (raw) return raw.trim().toLowerCase() !== "disabled";
  if (liveStr(live, "model", "model_provider")) return true;
  return false;
}

function flagsToDefaults(flags: CodexCapabilityFlags) {
  return {
    remoteCompaction: flags.compact,
    imageUnderstanding: flags.vision,
    imageGeneration: flags.imagegen,
    webSearch: flags.search,
  };
}

function liveCapabilityFlags(live: LiveSummary | undefined): CodexCapabilityFlags {
  return {
    compact: hydrateRemoteCompaction(live),
    vision: liveTruthy(live, "tools_view_image", "view_image"),
    imagegen: liveTruthy(live, "features_image_generation", "image_generation"),
    search: hydrateWebSearch(live),
  };
}

export function hydrateCodexForm(
  site: Site | null,
  status: TargetLiveStatus | undefined,
): CodexFormDefaults {
  const live = status?.liveSummary;
  const onSite = appliedOnSite(site, status);
  const liveModel = liveStr(live, "model") ?? status?.appliedModelId ?? undefined;
  const siteFlags = site ? codexFlagsFromCapabilities(site.capabilities) : EMPTY_CODEX_FLAGS;
  const explicitSource = liveStr(live, "capability_source");
  const useCustomLive = onSite && explicitSource === "custom";
  const capabilitySource: CodexCapabilitySource = useCustomLive ? "custom" : "site";
  const flags = useCustomLive ? liveCapabilityFlags(live) : siteFlags;

  return {
    modelId: onSite ? (liveModel ?? site?.selectedModelId ?? undefined) : (site?.selectedModelId ?? undefined),
    writeAllModels: Boolean(liveStr(live, "model_catalog_json")),
    reasoning: parseCodexReasoning(liveStr(live, "model_reasoning_effort")),
    capabilitySource,
    ...flagsToDefaults(flags),
  };
}

export function hydratePiForm(
  site: Site | null,
  status: TargetLiveStatus | undefined,
): PiFormDefaults {
  const live = status?.liveSummary;
  const onSite = appliedOnSite(site, status);
  const liveModel = liveStr(live, "defaultModel") ?? status?.appliedModelId ?? undefined;
  const modelCount = Number.parseInt(liveStr(live, "modelCount") ?? "0", 10);
  return {
    modelId: onSite
      ? (liveModel ?? site?.selectedModelId ?? undefined)
      : (site?.selectedModelId ?? undefined),
    writeAllModels:
      liveStr(live, "writeAllModels") === "true" ||
      (Number.isFinite(modelCount) && modelCount > 1),
  };
}

export function buildModelOptions(
  models: SiteModel[],
  extraIds: Array<string | undefined>,
): { value: string; label: string }[] {
  const opts = models.map((m) => ({
    value: m.modelId,
    label: m.displayName && m.displayName !== m.modelId ? `${m.displayName} (${m.modelId})` : m.modelId,
  }));
  const seen = new Set(opts.map((o) => o.value));
  for (const id of extraIds) {
    if (!id || seen.has(id)) continue;
    opts.unshift({ value: id, label: id });
    seen.add(id);
  }
  return opts;
}
