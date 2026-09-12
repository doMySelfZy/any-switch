---
name: "odd-status"
version: "1.0.1"
description: "Trellis status command. Show current project state, phase progress, and the exact next action scaffold."
metadata:
  priority: 9
  pathPatterns:
    - '.odd/state.json'
    - 'docs/plan.md'
    - 'docs/session-brief*.md'
  promptSignals:
    phrases:
      - "odd status"
      - "show odd status"
      - "resume odd status"
      - "where are we in odd"
      - "what is the odd status"
    allOf:
      - [odd, status]
    anyOf:
      - "status"
      - "progress"
      - "phase"
      - "resume"
    noneOf: []
    minScore: 5
retrieval:
  aliases:
    - odd status
    - odd progress
  intents:
    - show odd status
    - inspect odd progress
  entities:
    - current phase
    - next step
---

# /trellis-status

You are executing the Trellis `*status` command.

Execute this flow:

1. Read `.claude/skills/trellis/docs/runtime/status-protocol.md`.
2. Execute the status protocol exactly as written.
3. When the current stage is still in planning, surface the exact `trellis planning-action ...` command and starter payload for the next confirmed step.
