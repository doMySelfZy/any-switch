---
title: Introduction
description: "XiaoBaiSwitch is site-first: wire an upstream API to Claude Code, Codex, and Pi."
order: 1
---

**XiaoBaiSwitch** is a beginner-friendly desktop app for driving Claude Code, Codex, and Pi configuration from an upstream site.

There is one main line:

**Base URL + API key → models → target capability presets → apply to the target**

The site is the single source of truth. Get the upstream right, then write each CLI separately so URLs and models do not drift apart.

## Targets today

- **Claude Code**: writes `~/.claude/settings.json`
- **Codex**: writes `~/.codex/` and injects environment variables per your settings
- **Pi**: merges `~/.pi/agent/models.json`, `auth.json`, and `settings.json`

App data lives in **`~/.xiaobai-switch/`** (not the OS application-support folder, and not Tauri’s `app_data_dir`):

```text
~/.xiaobai-switch/
├── xiaobai-switch.db   # app state
├── master.key          # AES-256-GCM master key (mode 0600 on Unix)
└── backups/            # pre-apply backups
```

## Surfaces you will use

1. **Sites**: upstreams, routes, models, protocol, Codex capability presets
2. **Apply Center**: dedicated forms for Claude Code, Codex, and Pi
3. **Settings**: language, theme, tray, backup retention, updates, path overrides

Next: [Install](../install/), then [Quick start](../quick-start/).
