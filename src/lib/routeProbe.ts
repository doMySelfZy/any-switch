import { invoke } from "@/lib/invoke";
import type { UrlProbeResult } from "@/types/domain";

const STORAGE_KEY = "any-switch.route-probe.v1";

export type ProbeColor = "green" | "yellow" | "red";

export interface ProbeEntry {
  url: string;
  ok: boolean;
  latencyMs: number;
  probedAt: number;
}

export function colorForLatency(ok: boolean, ms: number): ProbeColor {
  if (!ok) return "red";
  if (ms < 1000) return "green";
  if (ms <= 3000) return "yellow";
  return "red";
}

export function formatLatency(ms: number): string {
  if (ms >= 1000) return `${(ms / 1000).toFixed(1)}s`;
  return `${Math.round(ms)}ms`;
}

export function isFresh(entry: ProbeEntry, ttlMinutes: number, now = Date.now()): boolean {
  const ttl = Math.max(1, ttlMinutes) * 60 * 1000;
  return now - entry.probedAt < ttl;
}

export function readProbeCache(): Record<string, ProbeEntry> {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return {};
    const parsed = JSON.parse(raw) as unknown;
    if (!parsed || typeof parsed !== "object") return {};
    return parsed as Record<string, ProbeEntry>;
  } catch {
    return {};
  }
}

export function writeProbeCache(cache: Record<string, ProbeEntry>) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(cache));
  } catch {
    // quota / private mode
  }
}

export function getCachedProbe(url: string): ProbeEntry | undefined {
  return readProbeCache()[url.trim()];
}

export function upsertProbeResults(results: UrlProbeResult[], now = Date.now()) {
  const cache = readProbeCache();
  for (const r of results) {
    cache[r.url.trim()] = {
      url: r.url.trim(),
      ok: r.ok,
      latencyMs: r.latencyMs,
      probedAt: now,
    };
  }
  writeProbeCache(cache);
}

export function urlsNeedingProbe(urls: string[], ttlMinutes: number, now = Date.now()): string[] {
  const cache = readProbeCache();
  return urls.filter((url) => {
    const entry = cache[url.trim()];
    return !entry || !isFresh(entry, ttlMinutes, now);
  });
}

export async function probeUrls(urls: string[]): Promise<UrlProbeResult[]> {
  if (urls.length === 0) return [];
  const results = await invoke<UrlProbeResult[]>("probe_urls", { urls });
  upsertProbeResults(results);
  return results;
}

export function resetProbeCache() {
  try {
    localStorage.removeItem(STORAGE_KEY);
  } catch {
    // ignore
  }
}
