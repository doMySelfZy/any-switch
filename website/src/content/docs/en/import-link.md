---
title: Import from a link
description: "Import a site with xiaobaiswitch:// ; import does not apply to Claude Code, Codex, or Pi automatically."
order: 10
---

After the desktop app is installed, a browser or another app can open a `xiaobaiswitch://` link to launch XiaoBaiSwitch and import an upstream site. **Import does not apply to tools automatically**; confirm in Apply Center yourself.

1. Install and open the desktop app
2. Click the import link; the app switches to Sites and shows a confirm dialog
3. Check the name, routes, protocol, notes, and API key prefix, then confirm
4. If the link has no `apikey`, confirm opens a prefilled add-site form so you can finish the key and save

<div class="not-prose">
<div role="alert" class="alert alert-warning my-4">
<p>Putting an API key in a URL can leave it in browser history, extensions, or system logs. Do not put a real key on a public page. Generate the link from a private, signed-in console, or omit <code>apikey</code> and let the user finish it in the app.</p>
</div>
</div>

## When two links are the same site

The same protocol plus the same set of routes (order does not matter) counts as the same site: a matching key is reused, a different key updates the stored key; adding or removing a route creates a new site instead of merging.

## Link format

```text
xiaobaiswitch://sites?name=<name>&baseurls=<url>[&baseurls=<url>…][&apikey=<key>][&protocol=openai_compatible|anthropic][&notes=<notes>][&codex-compact=1][&codex-vision=1][&codex-imagegen=1][&codex-search=1]
```

| Parameter | Required | Description |
|-----------|----------|-------------|
| `name` | Yes | Site name, up to 128 characters |
| `baseurls` | Yes | Route Base URL; repeatable, up to 20 |
| `apikey` | No | API key; if omitted, confirm opens a prefilled form |
| `protocol` | No | `openai_compatible` (default) or `anthropic` |
| `notes` | No | Notes, up to 2000 characters |
| `codex-compact` | No | Codex remote compaction; `1` / `true` / `on` / `yes` turns it on |
| `codex-vision` | No | Codex vision preset |
| `codex-imagegen` | No | Codex image-generation preset |
| `codex-search` | No | Codex built-in search preset |

Aliases: `baseurl` = `baseurls`, `type=openai` / `type=anthropic` = `protocol`. Other `platform-capability` kebab keys are stored as-is; the current UI only shows the four Codex ones. If the link includes any capability parameter, it is treated as a full Codex preset (omitted known keys are off). Older links without these parameters will not overwrite presets already on the site.

## Multiple routes

The first item is the current / default route. Prefer repeating `baseurls` so a URL that contains a comma is not split by mistake:

```text
xiaobaiswitch://sites?name=Example%20Relay&baseurls=https://a.example.com/v1&baseurls=https://b.example.com/v1&protocol=openai_compatible
```

A single parameter also works, separated by commas or `|`. `baseurl` and `baseurls` can be mixed and are merged in query-string order.
