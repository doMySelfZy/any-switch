---
title: Route switching
description: Probe latency, change the default route, and optionally sync already-applied Claude Code / Codex / Pi URLs.
order: 9
---

A site can have several Base URLs. The first item is the current / default route.

## Probe

The route dropdown can run a speed test. Settings control how long a result stays valid (default 10 minutes); after that, opening the dropdown probes again.

## What switching does

The confirm dialog explains that, by default, **API URLs already applied to Claude Code / Codex / Pi** are updated to this route. You can also “skip apply” and only change the site’s current route.

Outcomes you may see:

- Route switched and applied URLs synced
- Route switched, some apply syncs failed
- Route switched (site only)
- Route switched without syncing already-applied tools
