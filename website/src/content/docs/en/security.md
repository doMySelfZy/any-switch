---
title: Security
description: Keys are encrypted in the app; after Apply they may appear in plaintext in Claude / Codex / Pi / Prime config and the shell environment.
order: 13
---

API keys are **encrypted** in the app database (`~/.any-switch/any-switch.db` + `master.key`). Site lists and details show only a prefix. Opening the edit dialog decrypts the key on demand into a password field that stays hidden until you use the eye control. Logs never echo the raw secret.

<div class="not-prose">
<div role="alert" class="alert alert-warning my-4">
<p><strong>After Apply</strong>, keys may appear in plaintext in <code>~/.claude</code>, <code>~/.codex</code>, <code>~/.pi/agent/auth.json</code>, <code>~/.prime/agent/auth.json</code>, <code>codex.env</code>, or shell rc files. Do not sync those directories to untrusted cloud storage.</p>
</div>
</div>

## Data root

Use only `~/.any-switch/`. It is not the OS application-support folder or a bundle-id path.

On Unix, `master.key` is mode `0600`. Losing the master key means stored secrets cannot be decrypted.

## Public links

Do not put a real API key in a public `anyswitch://` page or chat log. Omit `apikey` and let the user finish it in the app. See [Import from a link](../import-link/).
