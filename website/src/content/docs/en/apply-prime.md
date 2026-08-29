---
title: Apply to Prime
description: Use Prime's official models.json, auth.json, and settings.json while preserving other providers and OAuth logins.
order: 8.5
---

Choose **Prime** in the Apply Center. Its config directory resolves in this order: app path setting, `PRIME_AGENT_CODING_AGENT_DIR`, then `~/.prime/agent`. Do not write Prime into `~/.pi/agent`.

## Files written

XiaoBaiSwitch uses Prime's official config interfaces and does not install an extension:

- `models.json`: one managed `xiaobai_` provider, Base URL, protocol, and model list
- `auth.json`: the matching API key
- `settings.json`: the default provider and model for new sessions

`openai_compatible` maps to `openai-completions` with a `/v1` Base URL. `anthropic` maps to `anthropic-messages`.

## Model catalog

With “Write all site models into Prime” off, only the default model is written. Turn it on to write the site's catalog, then open `/model` in Prime to reload and switch.

The first version writes only reliable model IDs and display names. Reasoning, image input, context windows, output limits, and compatibility options use Prime's official defaults instead of being guessed from model names.

## Preservation and single-active behavior

Prime keeps one XiaoBaiSwitch-managed provider at a time. Switching sites replaces only the `xiaobai_` namespace; custom providers, OAuth logins, comments, trailing commas, and unknown settings are preserved.

Project `.prime/agent/settings.json`, CLI flags, or a restored session can override the global default. That does not make the managed provider stale.

## Remove XiaoBai config

This removes only the managed provider and credential. Pre-apply defaults are restored only while Prime still points at the managed provider, so later user choices are not overwritten.

`auth.json` contains the plaintext key and is set to mode `0600` on Unix. See [Security](../security/).
