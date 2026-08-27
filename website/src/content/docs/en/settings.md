---
title: Settings and updates
description: Language, theme, tray, proxy, paths, backup policy, Codex injection, and auto-update.
order: 12
---

Open Settings from the title bar; Esc returns. Most switches save immediately.

## General

- **Language**: Simplified Chinese / English
- **Theme**: system, light, dark
- **Always on top**
- **Launch at login**
- **Close window to tray**
- **Start hidden in tray** (requires close-to-tray)

## Network

- Proxy: system / none / custom. System proxy does not evaluate PAC; if only PAC is configured, `HTTP_PROXY` / `HTTPS_PROXY` are used as fallback
- Route probe TTL, default 10 minutes

## Paths

Override the Claude, Codex, and Pi Agent config directories. With the Pi field empty, `PI_CODING_AGENT_DIR` is checked before `~/.pi/agent`.

## Apply behavior

- **Codex secret injection**: automatic (per platform), shell rc only, user environment variables, or `codex.env` only
- **Force exclusive Claude auth key**: writing the selected key deletes the other field

## Backups

See [Backups](../backups/).

## About and updates

Shows the version. You can check for updates manually, or enable automatic checks and set the interval. Official builds fetch `latest.json` from GitHub Releases.
