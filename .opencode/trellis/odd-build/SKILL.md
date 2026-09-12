---
name: "odd-build"
version: "1.0.1"
description: "Trellis build command. Reads project state from odd-flow memory and executes the *build protocol from the Trellis coach. Use /trellis-build to start or continue a build session."
metadata:
  priority: 9
  pathPatterns:
    - '.odd/state.json'
    - 'docs/plan.md'
    - 'docs/outcomes/**'
    - 'docs/session-brief*.md'
  promptSignals:
    phrases:
      - "odd build"
      - "start odd build"
      - "continue odd build"
      - "begin odd build"
      - "resume odd build"
    allOf:
      - [odd, build]
    anyOf:
      - "build"
      - "phase"
      - "verification"
      - "outcome"
    noneOf: []
    minScore: 5
retrieval:
  aliases:
    - odd build
    - build with odd
  intents:
    - start odd build
    - continue odd build
  entities:
    - current build phase
    - phase brief
---

# /trellis-build

You are executing the Trellis `*build` command.

Execute this flow:

1. Read `.opencode/trellis/docs/runtime/build-entry.md`.
2. Execute the build-entry checks exactly as written.
3. Then read `.opencode/trellis/docs/build/build-protocol.md` and execute the build flow exactly as written.

Do not spawn build agents or write source code before the brief gate passes.
