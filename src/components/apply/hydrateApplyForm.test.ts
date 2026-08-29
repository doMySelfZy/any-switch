import { describe, expect, it } from "vitest";
import type { Site, TargetLiveStatus } from "@/types/domain";
import {
  buildModelOptions,
  hydrateClaudeForm,
  hydrateCodexForm,
  hydratePiForm,
  hydratePrimeForm,
  pickApplySiteId,
  selectableApplySites,
} from "./hydrateApplyForm";

function site(partial: Partial<Site> & Pick<Site, "id">): Site {
  return {
    name: partial.name ?? partial.id,
    baseUrl: "https://api.example.com",
    baseUrls: ["https://api.example.com"],
    keyPrefix: "sk-xx",
    quotaRevision: "rev-1",
    hasKey: true,
    protocol: "openai_compatible",
    claudeAuthKeyStyle: "anthropic_auth_token",
    notes: null,
    enabled: true,
    sortOrder: 0,
    selectedModelId: null,
    lastModelFetchAt: null,
    lastModelFetchLatencyMs: null,
    lastModelFetchError: null,
    createdAt: 1,
    updatedAt: 1,
    ...partial,
  };
}

function status(partial: Partial<TargetLiveStatus> = {}): TargetLiveStatus {
  return {
    kind: "claude_code",
    installed: true,
    version: "2.1.0",
    configPath: "/tmp/settings.json",
    status: "applied",
    appliedSiteId: "shuai",
    appliedSiteName: "shuai",
    appliedModelId: "codex-auto-review",
    providerId: null,
    orphan: false,
    liveSummary: {
      ANTHROPIC_MODEL: "codex-auto-review",
      ANTHROPIC_DEFAULT_OPUS_MODEL: "opus-live",
      ANTHROPIC_DEFAULT_SONNET_MODEL: "sonnet-live",
      ANTHROPIC_DEFAULT_HAIKU_MODEL: "haiku-live",
      ANTHROPIC_AUTH_TOKEN: "sk-live",
      CLAUDE_CODE_EFFORT_LEVEL: "high",
    },
    lastAppliedAt: 99,
    staleReason: null,
    ...partial,
  };
}

describe("selectableApplySites", () => {
  it("only treats enabled sites as selectable", () => {
    const enabled = site({ id: "on", enabled: true });
    const off = site({ id: "off", enabled: false });
    expect(selectableApplySites([enabled, off])).toEqual([enabled]);
    expect(selectableApplySites([off])).toEqual([]);
  });
});

describe("pickApplySiteId", () => {
  const sites = [{ id: "gptnb" }, { id: "shuai" }];

  it("prefers an explicit go-apply prefill", () => {
    expect(
      pickApplySiteId({
        sites,
        prefillSiteId: "gptnb",
        selectedSiteId: "shuai",
        appliedSiteId: "shuai",
      }),
    ).toBe("gptnb");
  });

  it("falls back to the currently applied site", () => {
    expect(
      pickApplySiteId({
        sites,
        selectedSiteId: "gptnb",
        appliedSiteId: "shuai",
      }),
    ).toBe("shuai");
  });

  it("then uses the globally selected site, then the first site", () => {
    expect(pickApplySiteId({ sites, selectedSiteId: "gptnb" })).toBe("gptnb");
    expect(pickApplySiteId({ sites })).toBe("gptnb");
    expect(pickApplySiteId({ sites: [] })).toBeNull();
  });
});

describe("hydrateClaudeForm", () => {
  it("uses live target config when the selected site is the applied site", () => {
    const defaults = hydrateClaudeForm(
      site({ id: "shuai", selectedModelId: "gpt-4.1", claudeAuthKeyStyle: "anthropic_api_key" }),
      status(),
    );
    expect(defaults.modelId).toBe("codex-auto-review");
    expect(defaults.opusModel).toBe("opus-live");
    expect(defaults.sonnetModel).toBe("sonnet-live");
    expect(defaults.haikuModel).toBe("haiku-live");
    expect(defaults.effort).toBe("high");
    expect(defaults.auth).toBe("anthropic_auth_token");
    expect(defaults.use1mContext).toBe(false);
  });

  it("hydrates the 1M declaration without leaking the suffix into model selects", () => {
    const defaults = hydrateClaudeForm(
      site({ id: "shuai", selectedModelId: "gpt-4.1" }),
      status({
        liveSummary: {
          ANTHROPIC_MODEL: "codex-auto-review[1m]",
          ANTHROPIC_DEFAULT_OPUS_MODEL: "opus-live[1M]",
          ANTHROPIC_DEFAULT_SONNET_MODEL: "sonnet-live[1m]",
          ANTHROPIC_DEFAULT_HAIKU_MODEL: "haiku-live",
        },
      }),
    );
    expect(defaults.modelId).toBe("codex-auto-review");
    expect(defaults.opusModel).toBe("opus-live");
    expect(defaults.sonnetModel).toBe("sonnet-live");
    expect(defaults.haikuModel).toBe("haiku-live");
    expect(defaults.use1mContext).toBe(true);
  });

  it("does not copy the site primary model into aliases when applying a different site", () => {
    const defaults = hydrateClaudeForm(
      site({ id: "gptnb", selectedModelId: "gpt-4.1", claudeAuthKeyStyle: "anthropic_api_key" }),
      status(),
    );
    expect(defaults.modelId).toBe("gpt-4.1");
    expect(defaults.opusModel).toBeUndefined();
    expect(defaults.sonnetModel).toBeUndefined();
    expect(defaults.haikuModel).toBeUndefined();
    expect(defaults.auth).toBe("anthropic_api_key");
    expect(defaults.effort).toBe("high");
    expect(defaults.use1mContext).toBe(false);
  });

  it("leaves effort empty when nothing is written yet", () => {
    const defaults = hydrateClaudeForm(site({ id: "shuai", selectedModelId: "gpt-4.1" }), undefined);
    expect(defaults.modelId).toBe("gpt-4.1");
    expect(defaults.effort).toBeUndefined();
    expect(defaults.opusModel).toBeUndefined();
    expect(defaults.use1mContext).toBe(false);
  });
});

describe("hydrateCodexForm", () => {
  const live: TargetLiveStatus = {
    ...status({
      kind: "codex",
      appliedModelId: "codex-auto-review",
      liveSummary: {
        model: "codex-auto-review",
        model_reasoning_effort: "xhigh",
        model_catalog_json: "/tmp/models.json",
      },
    }),
  };

  it("uses live model / catalog / reasoning for the applied site", () => {
    const defaults = hydrateCodexForm(site({ id: "shuai", selectedModelId: "gpt-4.1" }), live);
    expect(defaults.modelId).toBe("codex-auto-review");
    expect(defaults.writeAllModels).toBe(true);
    expect(defaults.reasoning).toBe("xhigh");
  });

  it("defaults platform capabilities off when nothing is written", () => {
    const defaults = hydrateCodexForm(site({ id: "shuai", selectedModelId: "gpt-4.1" }), undefined);
    expect(defaults.capabilitySource).toBe("site");
    expect(defaults.remoteCompaction).toBe(false);
    expect(defaults.imageUnderstanding).toBe(false);
    expect(defaults.imageGeneration).toBe(false);
    expect(defaults.webSearch).toBe(false);
  });

  it("defaults to follow-site when live has no capability_source", () => {
    const defaults = hydrateCodexForm(
      site({
        id: "shuai",
        selectedModelId: "gpt-4.1",
        capabilities: { "codex-search": true },
      }),
      status({
        kind: "codex",
        appliedSiteId: "shuai",
        liveSummary: { model: "gpt-5.4", web_search: "disabled" },
      }),
    );
    expect(defaults.capabilitySource).toBe("site");
    expect(defaults.webSearch).toBe(true);
  });

  it("follows current site presets when capability_source is site", () => {
    const defaults = hydrateCodexForm(
      site({
        id: "shuai",
        selectedModelId: "gpt-4.1",
        capabilities: { "codex-vision": true, "codex-compact": true },
      }),
      status({
        kind: "codex",
        appliedSiteId: "shuai",
        liveSummary: {
          capability_source: "site",
          remote_compaction: "off",
          web_search: "disabled",
        },
      }),
    );
    expect(defaults.capabilitySource).toBe("site");
    expect(defaults.remoteCompaction).toBe(true);
    expect(defaults.imageUnderstanding).toBe(true);
    expect(defaults.imageGeneration).toBe(false);
    expect(defaults.webSearch).toBe(false);
  });

  it("reads platform capabilities from live summary", () => {
    const defaults = hydrateCodexForm(
      site({ id: "shuai", selectedModelId: "gpt-4.1" }),
      status({
        kind: "codex",
        appliedSiteId: "shuai",
        liveSummary: {
          capability_source: "custom",
          remote_compaction: "on",
          tools_view_image: "true",
          features_image_generation: "true",
          web_search: "cached",
        },
      }),
    );
    expect(defaults.remoteCompaction).toBe(true);
    expect(defaults.imageUnderstanding).toBe(true);
    expect(defaults.imageGeneration).toBe(true);
    expect(defaults.webSearch).toBe(true);
  });

  it("treats disabled web_search and false image flags as off", () => {
    const defaults = hydrateCodexForm(
      site({ id: "shuai" }),
      status({
        kind: "codex",
        appliedSiteId: "shuai",
        liveSummary: {
          capability_source: "custom",
          remote_compaction: "off",
          tools_view_image: "false",
          features_image_generation: "false",
          web_search: "disabled",
        },
      }),
    );
    expect(defaults.remoteCompaction).toBe(false);
    expect(defaults.imageUnderstanding).toBe(false);
    expect(defaults.imageGeneration).toBe(false);
    expect(defaults.webSearch).toBe(false);
  });

  it("falls back to OpenAI provider name for remote compaction", () => {
    const defaults = hydrateCodexForm(
      site({ id: "shuai", name: "Relay One" }),
      status({
        kind: "codex",
        appliedSiteId: "shuai",
        liveSummary: { capability_source: "custom", provider_display_name: "OpenAI" },
      }),
    );
    expect(defaults.remoteCompaction).toBe(true);
  });

  it("treats a live config without web_search as search enabled", () => {
    const defaults = hydrateCodexForm(
      site({ id: "shuai" }),
      status({
        kind: "codex",
        appliedSiteId: "shuai",
        liveSummary: { capability_source: "custom", model: "gpt-5.4" },
      }),
    );
    expect(defaults.webSearch).toBe(true);
    expect(defaults.remoteCompaction).toBe(false);
    expect(defaults.imageUnderstanding).toBe(false);
    expect(defaults.imageGeneration).toBe(false);
  });

  it("uses the site primary model when switching to another site", () => {
    const defaults = hydrateCodexForm(site({ id: "gptnb", selectedModelId: "gpt-4.1" }), live);
    expect(defaults.modelId).toBe("gpt-4.1");
    expect(defaults.writeAllModels).toBe(true);
    expect(defaults.reasoning).toBe("xhigh");
  });

  it("does not force write-all or medium reasoning when nothing is written", () => {
    const defaults = hydrateCodexForm(site({ id: "shuai", selectedModelId: "gpt-4.1" }), undefined);
    expect(defaults.writeAllModels).toBe(false);
    expect(defaults.reasoning).toBeUndefined();
  });
});

describe("buildModelOptions", () => {
  it("prepends live model ids that are missing from the site catalog", () => {
    const opts = buildModelOptions(
      [{ id: "1", siteId: "s", modelId: "gpt-4.1", displayName: "GPT", ownedBy: null, raw: null }],
      ["codex-auto-review", "gpt-4.1"],
    );
    expect(opts[0]).toEqual({ value: "codex-auto-review", label: "codex-auto-review" });
    expect(opts).toHaveLength(2);
  });
});

describe("hydratePiForm", () => {
  it("hydrates the applied model and write-all state from Pi summary", () => {
    const defaults = hydratePiForm(
      site({ id: "shuai", selectedModelId: "fallback" }),
      status({
        kind: "pi",
        appliedSiteId: "shuai",
        appliedModelId: "model-a",
        liveSummary: { defaultModel: "model-a", modelCount: "1", writeAllModels: "true" },
      }),
    );
    expect(defaults).toEqual({ modelId: "model-a", writeAllModels: true });
  });

  it("uses the selected site model when switching sites", () => {
    const defaults = hydratePiForm(
      site({ id: "other", selectedModelId: "site-model" }),
      status({ kind: "pi", appliedSiteId: "shuai", liveSummary: { modelCount: "3" } }),
    );
    expect(defaults.modelId).toBe("site-model");
    expect(defaults.writeAllModels).toBe(true);
  });
});

describe("hydratePrimeForm", () => {
  it("hydrates the applied model and write-all state from Prime summary", () => {
    const defaults = hydratePrimeForm(
      site({ id: "shuai", selectedModelId: "fallback" }),
      status({
        kind: "prime",
        appliedSiteId: "shuai",
        appliedModelId: "model-a",
        liveSummary: { defaultModel: "model-a", modelCount: "1", writeAllModels: "true" },
      }),
    );
    expect(defaults).toEqual({ modelId: "model-a", writeAllModels: true });
  });
});
