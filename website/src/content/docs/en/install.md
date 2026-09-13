---
title: Install
description: Download macOS and Windows installers from GitHub Releases, including the macOS “damaged” fix.
order: 2
---

**macOS** (Apple Silicon / Intel) and **Windows** (x64 / ARM64) are supported. Get the matching installer from [Releases](https://github.com/doMySelfZy/any-switch/releases), or use the site [download page](/en/download/).

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
xattr -cr /Applications/AnySwitch.app
```

Then right-click the app → Open.

## Automatic updates

Official desktop builds can check for updates and install them in-app. Check manually in **Settings → About**, or turn on automatic checks.

The official site is [https://any-switch.example.com](https://any-switch.example.com).

## Upgrading from XiaoBaiSwitch

AnySwitch continues XiaoBaiSwitch: on first launch it copies your data from `~/.xiaobai-switch/` to `~/.any-switch/` and keeps the old directory. **Uninstall or disable the old XiaoBaiSwitch after upgrading (including its launch-at-login).** Running both versions writes the same target CLI configs while each keeps its own WebDAV sync bookkeeping, so the old build can upload stale data that the new one pulls back. Also note rollback is asymmetric: new backups are `any-switch-backup-*`, which the old build does not recognize — restore new backups inside AnySwitch.
