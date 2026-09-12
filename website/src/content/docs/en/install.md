---
title: Install
description: Download macOS and Windows installers from GitHub Releases, including the macOS “damaged” fix.
order: 2
---

**macOS** (Apple Silicon / Intel) and **Windows** (x64 / ARM64) are supported. Get the matching installer from [Releases](https://github.com/doMySelfZy/xiaobai-switch/releases), or use the site [download page](/en/download/).

## Package types

| Platform | Package |
|----------|---------|
| macOS Apple Silicon | `.dmg` (aarch64) |
| macOS Intel | `.dmg` (x86_64) |
| Windows x64 | `.msi` / NSIS `.exe`, plus a portable zip |
| Windows ARM64 | NSIS `.exe` only, plus a portable zip |

## macOS says the app is “damaged”

The macOS build is **ad-hoc signed** (no Apple Developer ID, not notarized). After a browser download, macOS may say the app is damaged — that is the quarantine flag, not a broken file. **Privacy & Security will not show “Open Anyway”.**

Drag the app to Applications, then run:

```bash
xattr -cr /Applications/XiaoBaiSwitch.app
```

Then right-click the app → Open.

## Automatic updates

Official desktop builds can check for updates and install them in-app. Check manually in **Settings → About**, or turn on automatic checks.

The official site is [https://xiaobaiswitch.com](https://xiaobaiswitch.com).
