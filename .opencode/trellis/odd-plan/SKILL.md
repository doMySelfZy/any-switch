---
name: "trellis-plan"
version: "1.0.1"
description: "Trellis planning command. Start or continue the full planning phase — school context, governance, personas, outcomes, contracts, architecture, UI/UX, compliance, delivery, operations, verification, and build plan."
metadata:
  priority: 9
  pathPatterns:
    - '.odd/state.json'
    - '.odd/trellis-state.json'
    - 'docs/plan.md'
    - 'docs/personas/**'
    - 'docs/outcomes/**'
    - 'docs/contracts/**'
    - 'docs/governance/**'
    - 'docs/compliance/**'
  promptSignals:
    phrases:
      - "odd plan"
      - "start odd planning"
      - "continue odd planning"
      - "begin odd planning"
      - "resume odd planning"
    allOf:
      - [odd, plan]
    anyOf:
      - "governance"
      - "persona"
      - "outcome"
      - "contract"
      - "compliance"
      - "architecture"
      - "delivery"
      - "plan"
    noneOf: []
    minScore: 5
retrieval:
  aliases:
    - odd plan
    - plan with odd
  intents:
    - start odd planning
    - continue odd planning
  entities:
    - persona
    - outcome
    - contract map
---

# /trellis-plan

You are executing the Trellis `*plan` command.

Execute this flow:

1. Read `.odd/state.json`.
2. Read `.odd/trellis-state.json` if it exists. Treat it as the canonical planning model.
3. Route to the next incomplete planning stage using `currentStage` from `.odd/trellis-state.json` when available:
   - `context-governance` → `.opencode/trellis/docs/planning/school-discovery.md`
   - `personas` → `.opencode/trellis/docs/planning/persona-architect.md`
   - `outcomes` → `.opencode/trellis/docs/planning/outcome-writer.md`
   - `contracts` → `.opencode/trellis/docs/planning/systems-mapper.md`
   - `architecture` → `.opencode/trellis/docs/planning/architecture-planner.md`
   - `ui-ux` → `.opencode/trellis/docs/planning/ui-ux-planner.md`
   - `compliance-safeguarding` → `.opencode/trellis/docs/planning/compliance-planner.md`
   - `delivery-decisions` → `.opencode/trellis/docs/planning/delivery-planner.md`
   - `operations-handover` → `.opencode/trellis/docs/planning/operations-planner.md`
   - `build-plan` → `.opencode/trellis/docs/planning/build-planner.md`
   - `verify` → `.opencode/trellis/docs/planning/verification-planner.md`
   - `inspection-pack` → `.opencode/trellis/docs/planning/inspection-pack-planner.md`
4. If `.odd/trellis-state.json` does not exist, fall back to the legacy routing from `.odd/state.json`, but immediately steer the user toward the richer Trellis stages once the next planning artifact is created.
5. Announce the stage in one short line, then ask for the single next thing needed.
6. Follow the selected planning document exactly.

## Concise Planning Mode

- Keep stage framing to 2 short lines max.
- Ask one focused question at a time.
- Prefer direct prompts over coaching copy.
- Summarise only when a checkpoint is reached or the user asks.
- Only explain why a stage matters when the user asks `*why` or is genuinely stuck.
