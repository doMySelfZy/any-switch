---
name: "odd-debug"
version: "2.0.0"
description: "Trellis debug command. Activates a lightweight debug session (no full swarm init required), selects the correct debug strategy, and routes back to verification instead of drifting outside the ODD flow."
metadata:
  priority: 9
  pathPatterns:
    - '.odd/state.json'
    - 'docs/plan.md'
    - 'docs/outcomes/**'
    - 'docs/session-brief*.md'
  promptSignals:
    phrases:
      - "odd debug"
      - "start odd debug"
      - "continue odd debug"
      - "resume odd debug"
      - "debug this in odd"
    allOf:
      - [odd, debug]
    anyOf:
      - "debug"
      - "fix"
      - "broken"
      - "verification failed"
      - "regression"
    noneOf: []
    minScore: 5
retrieval:
  aliases:
    - odd debug
    - debug with odd
  intents:
    - start odd debug
    - continue odd debug
  entities:
    - debug strategy
    - failing outcome
    - verification failure
---

# /trellis-debug

You are executing the Trellis `*debug` command.

## What this command does

`*debug` activates a lightweight build session optimised for debugging, hotfixes, and security remediation work. It bypasses the full 9-step swarm init that `*build` requires — there is no agent dispatch, no task creation, no swarm spawn. The orchestrator writes code directly.

This is not a shortcut around ODD. The brief gate, verify gate, and outcome verification protocol are still enforced. What is relaxed is the agent-token requirement in `swarm-write` — the requirement that forces every source write through a background Task agent. That requirement exists to enable parallel multi-agent outcome builds. For a single targeted fix it is pure ceremony.

**When to use `*debug` instead of `*build`:**
- Fixing a failing test suite
- Applying a security patch
- Investigating and resolving a verification failure
- Any single-file or few-file change that does not require parallel agent execution

**When to use `*build` instead:**
- Building a new outcome from scratch
- Work that benefits from parallel backend + UI + QA agents
- Phase transitions

---

## Session Activation

Execute these steps before any investigation or fix work. Do them in order.

### Step 1 — State check

Read `.odd/state.json`. Confirm:
- `currentPhase` is `"build"` — if not, explain that `*debug` requires an active build phase
- `planApproved` is `true` — if not, route to `*plan`

If either check fails, stop here and explain what is needed.

### Step 2 — Activate the debug session marker

```bash
touch .odd/.odd-flow-swarm-active
```

This creates or refreshes the 24-hour build session marker. The `swarm-write` gate will now allow direct orchestrator writes (because `debugSession: true` bypasses Gate 2).

### Step 3 — Set debug session state

Update `.odd/state.json`:
- `debugSession: true`
- `buildMode: "debug"`
- `verificationConfirmed: false`
- `debugStartedAt: <current ISO timestamp>`
- `debugSummary: <one-sentence description of the issue — or "unknown" if not yet classified>`

### Step 4 — Store state to odd-flow

Call `mcp__odd-flow__memory_store`:
- Key: `trellis-project-state`
- Namespace: `trellis-project`
- Value: full contents of `.odd/state.json`
- upsert: true

### Step 5 — Confirm to user

Display:

---

Debug session active.

Source writes are unlocked. No agent dispatch required.
Brief gate and verify gate remain enforced.

State stored to odd-flow. Ready to investigate.

---

Then proceed immediately to Strategy Selection below.

---

## Strategy Selection

Read `.odd/state.json` to identify the active outcome and the latest failure. If the failure was described in the user's message, use that. If not yet clear, ask for a one-sentence description.

Choose exactly one debug strategy before inspecting code. State the chosen strategy and the reason.

- Choose `ui-behaviour` when the problem is visible in the interface and you do not yet have evidence of a backend or data fault
- Choose `full-stack` when the failure crosses a user action, server boundary, and persisted state
- Choose `auth-security` when access, identity, trust, or validation boundaries might be wrong
- Choose `integration-contract` when one part of the system expects data or sequencing another part does not produce
- Choose `background-process` when the failure depends on async handoff, jobs, retries, or event delivery
- Choose `performance-state` when the issue depends on timing, staleness, cache invalidation, or repeated actions

If more than one strategy seems plausible, do not fix anything yet. Gather one more piece of evidence, then choose the narrowest strategy that still explains the failure.

---

## Fix Protocol

After choosing the strategy, load `.opencode/trellis/docs/build/debug-protocol.md` for the detailed investigation procedure for that strategy.

When the fix is complete:

1. Run the relevant automated checks (tests, lint)
2. Update `.odd/state.json`:
   - `debugSession: false`
   - `buildMode: "verify"`
   - `debugStrategy: <chosen strategy>`
   - `debugTarget: <affected file or surface>`
   - `debugSummary: <resolved — one sentence>`
3. Call `mcp__odd-flow__memory_store` with key `trellis-project-state` to store final state
4. Return to the verification walkthrough from step one

Setting `debugSession: false` re-enables Gate 2 for subsequent work, returning to full swarm protocol.

---

## Non-negotiable rules

- Never mark an outcome verified during a debug session
- Never change multiple layers at once before reproducing the fault
- Never skip the reproduction step
- Never broaden the strategy after starting unless new evidence proves the original classification wrong
- Always clear `debugSession: false` when the fix is complete
