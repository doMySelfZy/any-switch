---
title: Apply to Claude Code
description: Choose site, model, auth field, alias map, and effort, then write settings.json.
order: 6
---

In Apply Center, select **Claude Code** on the left. Each target has its own form, not a shared checkbox panel.

The default write path is `~/.claude/settings.json` (the Claude config directory can be overridden in Settings).

## Site and model

- Pick an enabled site
- The **default model** is written as `ANTHROPIC_MODEL`
- Make sure the site already has models

The same model id may not be accepted by Claude Code, Codex, and Pi. If you see a multi-target warning you can still force-apply, but picking separately is usually better.

## Auth field

Claude Code only:

- `ANTHROPIC_AUTH_TOKEN` (recommended)
- `ANTHROPIC_API_KEY`

Settings can turn on “force exclusive Claude auth key”, which deletes the other field when writing the one you selected.

## Model alias map

Map Claude Code’s built-in **opus / sonnet / haiku** aliases to model ids on the current site. They may match the default model, or be cleared.

## Thinking effort

Effort is written as `effortLevel` and `CLAUDE_CODE_EFFORT_LEVEL` (Minimal / Low / Medium / High / Max / Extra High).

## After apply

The status card shows the applied site and a summary. Restart the terminal or reopen Claude Code.

**Restore official config** removes the relay Base URL, auth keys, and model overrides so Claude Code can use a claude.ai account again. Saved official credentials are not deleted; the current file is backed up first.
