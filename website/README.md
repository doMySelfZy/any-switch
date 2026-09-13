# AnySwitch website

Official site for AnySwitch.

> **域名占位提醒**：`any-switch.example.com` 只是占位符，不是真实域名。原站点
> `xiaobaiswitch.com` 属于上游原作者，不可继续使用。启用 GitHub Pages 自定义域名前，请把
> `website/src/lib/site.ts`（`SITE_ORIGIN` / `SITE_HOST`）、`website/astro.config.mjs`、
> `website/public/CNAME`、`website/public/robots.txt` 以及 `.github/workflows/website.yml`
> 的 CNAME 校验统一替换为 AnySwitch 的实际域名。

Stack: Astro (static) + Tailwind CSS v4 + daisyUI. No VitePress / Starlight.

## Local

```bash
pnpm install
pnpm dev
```

```bash
pnpm build
pnpm preview
```

Brand images and screenshots are copied from `../assets` by `scripts/sync-assets.mjs` on `dev` / `build`.

## Deploy

GitHub Actions (`.github/workflows/website.yml`) builds `website/` when that tree or the workflow file changes on `main`, when the **Release** workflow finishes publishing a GitHub Release, or on manual `workflow_dispatch`. The download page bakes installer URLs into the static HTML at build time (no browser calls to the GitHub API). Version tags do not deploy the site by themselves.

`public/CNAME` must be replaced together with the placeholder above. The **Website** workflow
fails the build with `CNAME is still a placeholder` while the value ends in `.example.com`,
so a placeholder can never be deployed as a GitHub Pages custom domain. Repo Settings → Pages
should use **GitHub Actions** as the source (not the `/docs` folder).
