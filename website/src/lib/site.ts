// 域名占位：`any-switch.example.com` 不是真实域名，需替换为 AnySwitch 自己的域名。
// 原 xiaobaiswitch.com 属于上游原作者，不可继续使用（见 website/README.md）。
export const SITE_ORIGIN = "https://any-switch.example.com";
export const SITE_HOST = "any-switch.example.com";
export const APP_NAME = "AnySwitch";
export const GITHUB_REPO_URL = "https://github.com/doMySelfZy/any-switch";
export const GITHUB_RELEASES_URL = `${GITHUB_REPO_URL}/releases`;
export const GITHUB_LATEST_RELEASE_URL = `${GITHUB_RELEASES_URL}/latest`;
export const GITHUB_ISSUES_URL = `${GITHUB_REPO_URL}/issues`;
export const THEME_STORAGE_KEY = "anyswitch-theme";
export const LOCALE_STORAGE_KEY = "anyswitch-locale";

export const LOCALES = ["zh", "en"] as const;
export type Locale = (typeof LOCALES)[number];

export function htmlLang(locale: Locale): string {
  return locale === "zh" ? "zh-CN" : "en";
}

export function ogLocale(locale: Locale): string {
  return locale === "zh" ? "zh_CN" : "en_US";
}

export function hreflang(locale: Locale): string {
  return locale === "zh" ? "zh-CN" : "en";
}

/** Locale-stripped slug: "" | "docs" | "docs/install" | "download" */
export function localePath(locale: Locale, slug = ""): string {
  const inner = slug.replace(/^\/+|\/+$/g, "");
  const prefix = locale === "en" ? "/en" : "";
  if (!inner) return prefix ? `${prefix}/` : "/";
  return `${prefix}/${inner}/`;
}

export function absUrl(locale: Locale, slug = ""): string {
  return `${SITE_ORIGIN}${localePath(locale, slug)}`;
}

export function assetUrl(path: string): string {
  const base = import.meta.env.BASE_URL;
  const clean = path.replace(/^\/+/, "");
  return `${base}${clean}`;
}

export function otherLocale(locale: Locale): Locale {
  return locale === "zh" ? "en" : "zh";
}
