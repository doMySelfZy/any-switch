const STORAGE_KEY = "any-switch.deepLink.handledStartupUrls";

function readHandled(): Set<string> {
  if (typeof sessionStorage === "undefined") return new Set();
  const raw = sessionStorage.getItem(STORAGE_KEY);
  if (!raw) return new Set();
  return new Set(raw.split("\n").filter(Boolean));
}

function writeHandled(handled: Set<string>) {
  if (typeof sessionStorage === "undefined") return;
  sessionStorage.setItem(STORAGE_KEY, [...handled].join("\n"));
}

/** Mark a deep link as already presented so webview reload does not reopen it. */
export function rememberHandledDeepLink(url: string) {
  const trimmed = url.trim();
  if (!trimmed) return;
  const handled = readHandled();
  handled.add(trimmed);
  writeHandled(handled);
}

/**
 * Filter startup `getCurrent()` URLs that were already shown in this tab session.
 * Right-click reload remounts React but keeps the last scheme URL in the plugin.
 */
export function consumeStartupDeepLinkUrls(urls: string[] | null | undefined): string[] {
  const handled = readHandled();
  const fresh: string[] = [];
  for (const raw of urls ?? []) {
    const url = raw.trim();
    if (!url || handled.has(url)) continue;
    handled.add(url);
    fresh.push(url);
  }
  writeHandled(handled);
  return fresh;
}

export function resetDeepLinkSession() {
  if (typeof sessionStorage === "undefined") return;
  sessionStorage.removeItem(STORAGE_KEY);
}
