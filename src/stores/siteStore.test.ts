import { beforeEach, describe, expect, it } from "vitest";
import { resetBrowserMock } from "@/lib/browserMock";
import { resetQuotaInflight, useSiteStore } from "./siteStore";

describe("siteStore fetchModels", () => {
  beforeEach(() => {
    resetBrowserMock();
    resetQuotaInflight();
    useSiteStore.setState({
      sites: [],
      modelsBySite: {},
      modelsLoadingBySite: {},
      quotaBySite: {},
      quotaCacheKeyBySite: {},
      quotaLoadingBySite: {},
      loading: false,
      hydrated: false,
      fetchingModels: false,
      error: null,
    });
  });

  it("keeps a manually added model after refreshing the catalog", async () => {
    const site = await useSiteStore.getState().createSite({
      name: "Relay",
      baseUrl: "https://api.example.com",
      apiKey: "sk-test",
    });

    await useSiteStore.getState().setSelectedModel(site.id, "gpt-5.6-terra");
    await useSiteStore.getState().listModels(site.id, { force: true });
    expect(
      useSiteStore.getState().modelsBySite[site.id]?.some((m) => m.modelId === "gpt-5.6-terra"),
    ).toBe(true);

    await useSiteStore.getState().fetchModels(site.id);

    const ids = (useSiteStore.getState().modelsBySite[site.id] ?? []).map((m) => m.modelId);
    expect(ids).toContain("gpt-4.1");
    expect(ids).toContain("gpt-5.6-terra");
  });

  it("loads the complete API key for site editing", async () => {
    const site = await useSiteStore.getState().createSite({
      name: "Relay",
      baseUrl: "https://api.example.com",
      apiKey: "sk-full-secret",
    });

    await expect(useSiteStore.getState().getSiteApiKey(site.id)).resolves.toBe(
      "sk-full-secret",
    );
  });

  it("deletes a model and does not bring it back on the next fetch", async () => {
    const site = await useSiteStore.getState().createSite({
      name: "Relay",
      baseUrl: "https://api.example.com",
      apiKey: "sk-test",
    });
    await useSiteStore.getState().fetchModels(site.id);
    await useSiteStore.getState().deleteModel(site.id, "gpt-4.1");

    expect(
      useSiteStore.getState().modelsBySite[site.id]?.some((m) => m.modelId === "gpt-4.1"),
    ).toBe(false);

    await useSiteStore.getState().fetchModels(site.id);
    expect(
      useSiteStore.getState().modelsBySite[site.id]?.some((m) => m.modelId === "gpt-4.1"),
    ).toBe(false);
    expect(
      useSiteStore.getState().modelsBySite[site.id]?.some((m) => m.modelId === "claude-sonnet-4"),
    ).toBe(true);
  });

  it("persists Codex capability presets on create, update, and import", async () => {
    const created = await useSiteStore.getState().createSite({
      name: "Relay",
      baseUrl: "https://api.example.com",
      apiKey: "sk-test",
      capabilities: { "codex-vision": true },
    });
    expect(created.capabilities?.["codex-vision"]).toBe(true);

    const updated = await useSiteStore.getState().updateSite(created.id, {
      capabilities: { "codex-search": true, "codex-vision": false },
    });
    expect(updated.capabilities?.["codex-search"]).toBe(true);
    expect(updated.capabilities?.["codex-vision"]).toBe(false);

    const imported = await useSiteStore.getState().importSiteFromDeepLink({
      name: "Other",
      baseUrls: ["https://other.example.com"],
      apiKey: "sk-other",
      capabilities: { "codex-compact": true, "codex-vision": true },
    });
    expect(imported.created).toBe(true);
    expect(imported.site.capabilities?.["codex-compact"]).toBe(true);
    expect(imported.site.capabilities?.["codex-vision"]).toBe(true);
  });

  it("imports a deep-link site and reuses the same protocol + URL set", async () => {
    const created = await useSiteStore.getState().importSiteFromDeepLink({
      name: "Relay",
      baseUrls: ["https://b.example.com", "https://a.example.com"],
      apiKey: "sk-test",
      protocol: "openai_compatible",
    });
    expect(created.created).toBe(true);
    expect(created.site.baseUrl).toBe("https://b.example.com");

    const reused = await useSiteStore.getState().importSiteFromDeepLink({
      name: "Relay",
      baseUrls: ["https://a.example.com", "https://b.example.com"],
      apiKey: "sk-test",
      protocol: "openai_compatible",
    });
    expect(reused.created).toBe(false);
    expect(reused.reused).toBe(true);
    expect(reused.site.id).toBe(created.site.id);
    expect(reused.site.baseUrl).toBe("https://b.example.com");
    expect(useSiteStore.getState().sites).toHaveLength(1);

    const updated = await useSiteStore.getState().importSiteFromDeepLink({
      name: "Relay 2",
      baseUrls: ["https://a.example.com", "https://b.example.com"],
      apiKey: "sk-other",
      protocol: "openai_compatible",
    });
    expect(updated.updatedKey).toBe(true);
    expect(updated.site.id).toBe(created.site.id);
    expect(updated.site.name).toBe("Relay 2");
  });

  it("switchRoute moves the selected url to the front", async () => {
    const site = await useSiteStore.getState().createSite({
      name: "Relay",
      baseUrl: "https://a.example.com",
      apiKey: "sk-test",
    });
    await useSiteStore.getState().updateSite(site.id, {
      baseUrls: ["https://a.example.com", "https://b.example.com"],
    });
    const result = await useSiteStore.getState().switchRoute(site.id, "https://b.example.com");
    expect(result.site.baseUrl).toBe("https://b.example.com");
    expect(result.site.baseUrls[0]).toBe("https://b.example.com");
    expect(useSiteStore.getState().sites[0]?.baseUrl).toBe("https://b.example.com");
  });

  it("switchRoute can skip applying target urls", async () => {
    const site = await useSiteStore.getState().createSite({
      name: "Relay",
      baseUrl: "https://a.example.com",
      apiKey: "sk-test",
    });
    await useSiteStore.getState().updateSite(site.id, {
      baseUrls: ["https://a.example.com", "https://b.example.com"],
    });
    const result = await useSiteStore.getState().switchRoute(site.id, "https://b.example.com", {
      apply: false,
    });
    expect(result.site.baseUrl).toBe("https://b.example.com");
    expect(result.results).toEqual([]);
  });

  it("clears the catalog without blocking the next fetch", async () => {
    const site = await useSiteStore.getState().createSite({
      name: "Relay",
      baseUrl: "https://api.example.com",
      apiKey: "sk-test",
    });
    await useSiteStore.getState().fetchModels(site.id);
    await useSiteStore.getState().clearModels(site.id);

    expect(useSiteStore.getState().modelsBySite[site.id]).toEqual([]);
    expect(useSiteStore.getState().sites[0]?.selectedModelId).toBeNull();

    await useSiteStore.getState().fetchModels(site.id);
    expect(
      useSiteStore.getState().modelsBySite[site.id]?.some((m) => m.modelId === "gpt-4.1"),
    ).toBe(true);
  });

  it("caches quota probes until force refresh", async () => {
    const site = await useSiteStore.getState().createSite({
      name: "Relay",
      baseUrl: "https://api.example.com",
      apiKey: "sk-test",
    });
    const first = await useSiteStore.getState().probeQuota(site.id);
    expect(first.status).toBe("available");
    expect(first.remainingUsd).toBe(87.5);
    const cached = await useSiteStore.getState().probeQuota(site.id);
    expect(cached.fetchedAt).toBe(first.fetchedAt);

    await new Promise((r) => setTimeout(r, 5));
    const forced = await useSiteStore.getState().probeQuota(site.id, { force: true });
    expect(forced.status).toBe("available");
    expect(forced.fetchedAt).toBeGreaterThan(first.fetchedAt);
  });
});
