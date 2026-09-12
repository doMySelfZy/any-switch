---
name: "odd-sync"
description: "Sync ODD project state to odd-flow memory. MUST be run before building. Creates enforcement markers that allow build agents to spawn."
---

# /trellis-sync

You are syncing ODD project state to odd-flow memory. Follow these steps exactly:

## Step 1 — Read project state

Read `.odd/state.json` and store the full contents in a variable.

## Step 2 — Store state in odd-flow

Call `mcp__odd-flow__memory_store` with:
- **key**: `trellis-project-state`
- **namespace**: `trellis-project`
- **value**: the full contents of `.odd/state.json`

## Step 3 — Read and store the current session brief

Read `sessionBriefCount` from state.json. If it is greater than 0, read the file `docs/session-brief-[N].md` where N equals the sessionBriefCount value.

If the brief file exists, call `mcp__odd-flow__memory_store` with:
- **key**: `trellis-session-brief-[N]` (replace [N] with the actual number)
- **namespace**: `trellis-project`
- **value**: the full contents of the session brief file

## Step 4 — Create marker files

Run via Bash:
```bash
touch .odd/.odd-flow-phase-synced
```

If a session brief was stored in Step 3, also run:
```bash
touch .odd/.odd-flow-brief-stored
```

## Step 5 — Confirm

Report to the user:

> odd-flow synced. Phase [X] state and brief [N] stored. Build agents unlocked.

Replace [X] with `currentBuildPhase` from state.json and [N] with the session brief number. If no brief was stored, say:

> odd-flow synced. Phase [X] state stored. No session brief found — store one before confirming.
