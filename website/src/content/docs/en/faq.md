---
title: FAQ
description: macOS will not open, config does not take effect, key display, and import behavior.
order: 14
---

## macOS says the app is damaged

See [Install](../install/). Run `xattr -cr /Applications/AnySwitch.app`, then right-click Open.

## Apply succeeded, but the CLI still uses the old URL

Restart the terminal, or fully quit and reopen Claude Code / Codex / Pi. Apply only writes config files; it does not hot-reload a running process. In Pi, open `/model` to reload the model list.

## How can I view the full API key?

Site lists and details show only a prefix. Open the site edit dialog, wait for the complete key to load, then use the eye control on the password field. The key remains encrypted in the app database.

## Does an import link change Claude / Codex immediately?

No. Import only creates or updates a site. You still apply in Apply Center.

## Is Linux supported?

Current releases are macOS and Windows.

## Should I uninstall the old XiaoBaiSwitch after upgrading?

Yes. The old build may still be launched at login; running both versions writes the same target CLI configs while each keeps independent sync bookkeeping, so stale data can be uploaded by the old build and pulled back by the new one. Uninstall it or at least turn off its launch-at-login. Rollback is asymmetric too: new backups use the `any-switch-backup-*` prefix, which the old build does not recognize — restore new backups inside AnySwitch.

## What is the official site URL?

[https://any-switch.example.com](https://any-switch.example.com) (GitHub Pages custom domain). Docs live at `/docs/`; English at `/en/`.
