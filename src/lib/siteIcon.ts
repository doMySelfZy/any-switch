import { invoke } from "@/lib/invoke";
import type { HttpBytesResult } from "@/types/domain";

const STORAGE_KEY = "any-switch.site-icons.v1";
const BODY_HTML_RE = /<!doctype\s+html|<html[\s>]|<head[\s>]/i;

export interface HttpTextResult {
  status: number;
  contentType: string;
  finalUrl: string;
  body: string;
}

export interface SiteIconCacheEntry {
  origin: string;
  iconUrl: string | null;
}

const inflight = new Map<string, Promise<string | null>>();
const listeners = new Set<() => void>();

function emit() {
  for (const fn of listeners) fn();
}

export function subscribeSiteIconCache(fn: () => void): () => void {
  listeners.add(fn);
  return () => {
    listeners.delete(fn);
  };
}

export function originFromBaseUrl(baseUrl: string): string | null {
  try {
    const u = new URL(baseUrl.trim());
    if (u.protocol !== "http:" && u.protocol !== "https:") return null;
    if (!u.hostname) return null;
    return u.origin;
  } catch {
    return null;
  }
}

function readCache(): Record<string, SiteIconCacheEntry> {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return {};
    const parsed = JSON.parse(raw) as unknown;
    if (!parsed || typeof parsed !== "object") return {};
    return parsed as Record<string, SiteIconCacheEntry>;
  } catch {
    return {};
  }
}

function writeCache(cache: Record<string, SiteIconCacheEntry>) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(cache));
  } catch {
    // quota / private mode
  }
  emit();
}

/** `undefined` = miss; `null` = resolved with no icon. */
export function getCachedSiteIcon(siteId: string, origin: string): string | null | undefined {
  const entry = readCache()[siteId];
  if (!entry || entry.origin !== origin) return undefined;
  return entry.iconUrl;
}

export function setCachedSiteIcon(siteId: string, origin: string, iconUrl: string | null) {
  const cache = readCache();
  cache[siteId] = { origin, iconUrl };
  writeCache(cache);
}

export function invalidateSiteIconCache(siteId: string) {
  const cache = readCache();
  if (!(siteId in cache)) {
    emit();
    return;
  }
  delete cache[siteId];
  writeCache(cache);
}

export function resetSiteIconCache() {
  inflight.clear();
  try {
    localStorage.removeItem(STORAGE_KEY);
  } catch {
    // ignore
  }
  emit();
}

function isHtml(contentType: string, body: string): boolean {
  const ct = contentType.toLowerCase();
  if (ct.includes("text/html") || ct.includes("application/xhtml")) return true;
  if (ct.includes("json") || ct.includes("javascript") || ct.includes("image/")) return false;
  return BODY_HTML_RE.test(body.slice(0, 2000));
}

function iconPriority(rel: string, sizes: string | null): number {
  const tokens = rel.toLowerCase().split(/\s+/).filter(Boolean);
  let score = 0;
  const apple =
    tokens.includes("apple-touch-icon") || tokens.includes("apple-touch-icon-precomposed");
  if (tokens.includes("icon") && !apple) score += 40;
  if (!tokens.includes("shortcut")) score += 5;
  if (apple) score += 10;
  if (sizes && sizes !== "any") {
    const n = sizes
      .split(/\s+/)
      .map((part) => {
        const m = part.match(/(\d+)/);
        return m ? Number(m[1]) : 0;
      })
      .reduce((a, b) => Math.max(a, b), 0);
    score += Math.min(n, 256) / 16;
  }
  return score;
}

export function parseIconHrefs(html: string, pageUrl: string): string[] {
  if (typeof DOMParser === "undefined") return [];
  const doc = new DOMParser().parseFromString(html, "text/html");
  const ranked: { href: string; score: number }[] = [];
  for (const link of Array.from(doc.querySelectorAll("link[rel][href]"))) {
    const rel = link.getAttribute("rel") ?? "";
    const href = link.getAttribute("href")?.trim();
    if (!href) continue;
    const tokens = rel.toLowerCase().split(/\s+/).filter(Boolean);
    if (tokens.includes("mask-icon")) continue;
    const isIcon =
      tokens.includes("icon") ||
      tokens.includes("apple-touch-icon") ||
      tokens.includes("apple-touch-icon-precomposed");
    if (!isIcon) continue;
    try {
      const abs = new URL(href, pageUrl).href;
      ranked.push({ href: abs, score: iconPriority(rel, link.getAttribute("sizes")) });
    } catch {
      // skip invalid href
    }
  }
  ranked.sort((a, b) => b.score - a.score);
  return [...new Set(ranked.map((r) => r.href))];
}

function dataUrlFromBytes(r: HttpBytesResult): string | null {
  if (r.status < 200 || r.status >= 400 || !r.base64) return null;
  const ct = (r.contentType.split(";")[0] ?? "").trim().toLowerCase();
  const looksImage =
    ct.startsWith("image/") ||
    ct.includes("icon") ||
    ct === "application/octet-stream" ||
    ct === "";
  if (!looksImage) return null;
  const mime = ct.startsWith("image/") || ct.includes("icon") ? ct || "image/x-icon" : "image/x-icon";
  return `data:${mime};base64,${r.base64}`;
}

export async function probeImage(url: string): Promise<string | null> {
  try {
    const r = await invoke<HttpBytesResult>("fetch_http_bytes", { url });
    return dataUrlFromBytes(r);
  } catch {
    return null;
  }
}

async function resolveFromNetwork(origin: string): Promise<string | null> {
  try {
    const page = await invoke<HttpTextResult>("fetch_http_text", { url: `${origin}/` });
    const ok = page.status >= 200 && page.status < 400;
    if (ok && isHtml(page.contentType, page.body)) {
      const base = page.finalUrl || `${origin}/`;
      for (const href of parseIconHrefs(page.body, base)) {
        const data = await probeImage(href);
        if (data) return data;
      }
    }
  } catch {
    // try favicon next
  }
  const favicon = `${origin}/favicon.ico`;
  return probeImage(favicon);
}

export async function resolveSiteIcon(siteId: string, baseUrl: string): Promise<string | null> {
  const origin = originFromBaseUrl(baseUrl);
  if (!origin) return null;

  const cached = getCachedSiteIcon(siteId, origin);
  if (cached !== undefined) return cached;

  const key = `${siteId}:${origin}`;
  const pending = inflight.get(key);
  if (pending) return pending;

  const task = (async () => {
    try {
      const iconUrl = await resolveFromNetwork(origin);
      setCachedSiteIcon(siteId, origin, iconUrl);
      return iconUrl;
    } finally {
      inflight.delete(key);
    }
  })();
  inflight.set(key, task);
  return task;
}
