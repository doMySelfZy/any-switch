import type { SiteCapabilities, SiteProtocol } from "@/types/domain";
import { redactSecret } from "@/lib/redact";
import { normalizeBaseUrls } from "@/lib/urlNormalize";
import {
  appendCapabilitiesToSearchParams,
  capabilitiesFromSearchParams,
} from "@/lib/siteCapabilities";

export interface SiteDeepLinkPayload {
  name: string;
  baseUrls: string[];
  apiKey: string | null;
  protocol: SiteProtocol;
  notes: string | null;
  capabilities: SiteCapabilities;
  hasCapabilityParams: boolean;
  keyName: string | null;
}

export const SITE_DEEP_LINK_SCHEME = "anyswitch:";
export const SITE_DEEP_LINK_TARGET = "sites";
export const MAX_SITE_DEEP_LINK_NAME = 128;
export const MAX_SITE_DEEP_LINK_NOTES = 2000;
export const MAX_SITE_DEEP_LINK_ROUTES = 20;
export const MAX_SITE_DEEP_LINK_URL = 2048;

function getDeepLinkTarget(url: URL): string {
  if (url.hostname) return url.hostname;
  return url.pathname.replace(/^\/+/, "").split("/")[0] ?? "";
}

function parseProtocol(raw: string | null): SiteProtocol | null {
  if (!raw) return "openai_compatible";
  const v = raw.trim().toLowerCase();
  if (v === "anthropic") return "anthropic";
  if (v === "openai" || v === "openai_compatible") return "openai_compatible";
  return null;
}

function splitRouteToken(raw: string): string[] {
  return raw
    .split(/[,|]/)
    .map((s) => s.trim())
    .filter(Boolean);
}

export function parseSiteDeepLink(rawUrl: string): SiteDeepLinkPayload | null {
  let url: URL;
  try {
    url = new URL(rawUrl);
  } catch {
    return null;
  }
  if (url.protocol !== SITE_DEEP_LINK_SCHEME) return null;
  if (getDeepLinkTarget(url) !== SITE_DEEP_LINK_TARGET) return null;

  const name = url.searchParams.get("name")?.trim() ?? "";
  if (!name || name.length > MAX_SITE_DEEP_LINK_NAME) return null;

  const rawRoutes: string[] = [];
  for (const [key, value] of url.searchParams.entries()) {
    if (key === "baseurls" || key === "baseurl") {
      rawRoutes.push(...splitRouteToken(value));
    }
  }
  if (rawRoutes.some((u) => u.length > MAX_SITE_DEEP_LINK_URL)) return null;

  let baseUrls: string[];
  try {
    baseUrls = normalizeBaseUrls(rawRoutes);
  } catch {
    return null;
  }
  if (baseUrls.length > MAX_SITE_DEEP_LINK_ROUTES) return null;

  const protocol = parseProtocol(
    url.searchParams.get("protocol") ?? url.searchParams.get("type"),
  );
  if (!protocol) return null;

  const apiKey = url.searchParams.get("apikey")?.trim() || null;
  const keyName = url.searchParams.get("keyname")?.trim() || null;
  const notesRaw = url.searchParams.get("notes")?.trim() || "";
  if (notesRaw.length > MAX_SITE_DEEP_LINK_NOTES) return null;

  const parsedCaps = capabilitiesFromSearchParams(url.searchParams);

  return {
    name,
    baseUrls,
    apiKey,
    protocol,
    notes: notesRaw || null,
    capabilities: parsedCaps.capabilities,
    hasCapabilityParams: parsedCaps.present,
    keyName,
  };
}

export function buildSiteDeepLink(payload: SiteDeepLinkPayload): string {
  const params = new URLSearchParams();
  params.set("name", payload.name);
  for (const u of payload.baseUrls) params.append("baseurls", u);
  if (payload.apiKey) params.set("apikey", payload.apiKey);
  if (payload.keyName) params.set("keyname", payload.keyName);
  params.set("protocol", payload.protocol);
  if (payload.notes) params.set("notes", payload.notes);
  if (payload.hasCapabilityParams) {
    appendCapabilitiesToSearchParams(params, payload.capabilities);
  }
  return `anyswitch://sites?${params.toString()}`;
}

export function getSiteDeepLinkKeyPrefix(apiKey: string | null): string {
  return apiKey ? redactSecret(apiKey) : "—";
}
