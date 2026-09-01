import type { Site, SiteApiKeySummary } from "@/types/domain";

export function siteApiKeys(site: Site | null | undefined): SiteApiKeySummary[] {
  return site?.apiKeys ?? [];
}

export function activeApiKey(site: Site | null | undefined): SiteApiKeySummary | null {
  if (!site) return null;
  const keys = siteApiKeys(site);
  return keys.find((key) => key.id === site.activeApiKeyId) ?? keys.find((key) => key.isActive) ?? null;
}

export function activeApiKeyId(site: Site | null | undefined): string | null {
  return activeApiKey(site)?.id ?? site?.activeApiKeyId ?? null;
}

export function projectActiveKey(site: Site, key: SiteApiKeySummary | null): Site {
  if (!key) return site;
  return {
    ...site,
    activeApiKeyId: key.id,
    keyPrefix: key.keyPrefix,
    quotaRevision: key.quotaRevision,
    selectedModelId: key.selectedModelId,
    lastModelFetchAt: key.lastModelFetchAt,
    lastModelFetchLatencyMs: key.lastModelFetchLatencyMs,
    lastModelFetchError: key.lastModelFetchError,
    hasKey: true,
    apiKeys: siteApiKeys(site).map((item) => ({ ...item, isActive: item.id === key.id })),
  };
}
