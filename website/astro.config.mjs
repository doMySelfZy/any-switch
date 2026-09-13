// @ts-check
import { defineConfig } from "astro/config";
import { unified } from "@astrojs/markdown-remark";
import sitemap from "@astrojs/sitemap";
import icon from "astro-icon";
import tailwindcss from "@tailwindcss/vite";
import rehypeSlug from "rehype-slug";

const site = "https://any-switch.example.com";

export default defineConfig({
  site,
  base: "/",
  output: "static",
  trailingSlash: "always",
  i18n: {
    defaultLocale: "zh",
    locales: ["zh", "en"],
    routing: {
      prefixDefaultLocale: false,
    },
  },
  integrations: [
    icon(),
    sitemap({
      i18n: {
        defaultLocale: "zh",
        locales: {
          zh: "zh-CN",
          en: "en-US",
        },
      },
    }),
  ],
  markdown: {
    processor: unified({
      rehypePlugins: [rehypeSlug],
    }),
    shikiConfig: {
      theme: "github-dark",
    },
  },
  vite: {
    plugins: [tailwindcss()],
  },
});
