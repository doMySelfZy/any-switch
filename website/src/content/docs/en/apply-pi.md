---
title: Apply to Pi
description: Use Pi's official models.json, auth.json, and settings.json while preserving other providers and OAuth logins.
order: 8
---

Choose **Pi** in the Apply Center. Its config directory resolves in this order: app path setting, `PI_CODING_AGENT_DIR`, then `~/.pi/agent`.

## Files written

XiaoBaiSwitch uses Pi's official config interfaces and does not install an extension:

- `models.json`: one managed `xiaobai_` provider, Base URL, protocol, and model list
- `auth.json`: the matching API key
- `settings.json`: the default provider and model for new sessions

`openai_compatible` maps to `openai-completions` with a `/v1` Base URL. `anthropic` maps to `anthropic-messages`.

## Model catalog

With “Write all site models into Pi” off, only the default model is written. Turn it on to write the site's catalog, then open `/model` in Pi to reload and switch.

Model IDs, display names, and image input are written to the catalog. Thinking support is not inferred from model names: enable `reasoning` per model in the Apply Center, then optionally write `defaultThinkingLevel` and extended-level mappings. Pi Anthropic needs Adaptive thinking (`compat.forceAdaptiveThinking`) before `xhigh` / `max`. Context windows, output limits, and other fields still use Pi's official defaults.

## Preservation and single-active behavior

Pi keeps one XiaoBaiSwitch-managed provider at a time. Switching sites replaces only the `xiaobai_` namespace; custom providers, OAuth logins, comments, trailing commas, and unknown settings are preserved.

Project `.pi/settings.json`, CLI flags, or a restored session can override the global default. That does not make the managed provider stale.

## Remove XiaoBai config

This removes only the managed provider and credential. Pre-apply defaults are restored only while Pi still points at the managed provider, so later user choices are not overwritten.

`auth.json` contains the plaintext key and is set to mode `0600` on Unix. See [Security](../security/).
