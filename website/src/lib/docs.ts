import { getCollection, type CollectionEntry } from "astro:content";
import type { Locale } from "./site";

export const DOC_SLUGS = [
  "introduction",
  "install",
  "quick-start",
  "sites",
  "models",
  "apply-claude",
  "apply-codex",
  "apply-pi",
  "apply-prime",
  "routes",
  "import-link",
  "backups",
  "settings",
  "security",
  "faq",
] as const;

export type DocSlug = (typeof DOC_SLUGS)[number];

export function docSlugOf(entry: CollectionEntry<"docs">): string {
  const slash = entry.id.indexOf("/");
  return slash >= 0 ? entry.id.slice(slash + 1) : entry.id;
}

export function docLocaleOf(entry: CollectionEntry<"docs">): Locale {
  const prefix = entry.id.split("/")[0];
  return prefix === "en" ? "en" : "zh";
}

export async function docsForLocale(locale: Locale): Promise<CollectionEntry<"docs">[]> {
  const all = await getCollection("docs");
  return all
    .filter((entry) => docLocaleOf(entry) === locale)
    .sort((a, b) => a.data.order - b.data.order);
}

export async function docsStaticPaths(locale: Locale) {
  const docs = await docsForLocale(locale);
  return docs.map((entry) => ({
    params: { slug: docSlugOf(entry) },
    props: { entry, locale, docs },
  }));
}
