---
title: Sites
description: Manage upstream names, routes, protocol, API keys, and Codex capability presets.
order: 4
---

A site is one upstream relay: a set of Base URLs plus an API key, then models and target presets.

## Core fields

| Field | Notes |
|-------|--------|
| Display name | Up to 128 characters |
| Routes / Base URL | Up to 20; the first item is the current / default route |
| API key | Encrypted in the app database; lists show a prefix, while editing decrypts it on demand into a hidden password field |
| Protocol | `OpenAI-compatible` (default) or `Anthropic` |
| Notes | Optional, up to 2000 characters |

Advanced settings (protocol, notes) are collapsed by default.

## API key quota

Opening a site’s details probes the current route with the stored API key against OpenAI-compatible billing endpoints:

- `GET /v1/dashboard/billing/credit_grants`
- `GET /v1/dashboard/billing/subscription`
- `GET /v1/dashboard/billing/usage`
- `GET /api/usage/token` (New API-style key quota; may be CNY)

The remaining / used / total balance is shown only when the response can be parsed. You can refresh it manually. Official OpenAI or Anthropic user keys usually cannot query a balance; in that case the row is hidden and is not treated as an error.

## Multiple routes

The first item is the current default. You can probe and switch; see [Route switching](../routes/).

The write preview shows the models URL, Claude Base URL, and Codex `base_url` so you can check suffixes such as `/v1`.

## Codex private capability presets

Site edit has a collapsed “Codex private capabilities” block. Kebab keys match import links:

- `codex-compact` remote compaction
- `codex-vision` vision
- `codex-imagegen` image generation
- `codex-search` built-in search

Most relays do not support these; turn them on only when the upstream actually does. Apply Center can follow the site preset or override for one apply.

## Enable and disable

When you disable a site that is already applied, you can choose whether to clear those target configs. If you skip clearing, the target shows as an orphan.
