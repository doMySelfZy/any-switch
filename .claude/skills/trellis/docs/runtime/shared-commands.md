# Shared Commands

## `*context`

Jump directly to school context and governance work. Load `docs/planning/school-discovery.md` and collect school context, ownership, jurisdiction, confidence mode, and tenancy assumptions.

## `*persona`

Jump directly to persona work regardless of current state. Load `docs/planning/persona-architect.md` and activate Diana.

## `*outcome`

Jump directly to outcome writing. Check that at least one approved persona exists first. Then load `docs/planning/outcome-writer.md` and activate Marcus.

## `*contracts`

Jump directly to contract mapping. Check that at least one approved outcome exists. Then load `docs/planning/systems-mapper.md` and activate Theo.

## `*architecture`

Jump directly to architecture planning. Check that contracts have been mapped first. Then load `docs/planning/architecture-planner.md`.

## `*compliance`

Jump directly to compliance and safeguarding planning. Load `docs/planning/compliance-planner.md`.

## `*delivery`

Jump directly to delivery decisions. Load `docs/planning/delivery-planner.md`.

## `*operations`

Jump directly to operations and handover planning. Load `docs/planning/operations-planner.md`.

## `*verify-design`

Jump directly to verification design. Load `docs/planning/verification-planner.md`.

## `*inspection-pack`

Jump directly to inspection-pack planning. Load `docs/planning/inspection-pack-planner.md`.

## `*phase-plan`

Jump directly to implementation planning. Check that contracts have been mapped. Then load `docs/planning/build-planner.md` and activate Rachel.

## `*ui`

Load the UI excellence briefing from `docs/ui/design-system.md`.

## `*agent`

Create a custom agent for a domain-specific concern. Collect the concern in plain language, then call `mcp__odd-flow__agent_spawn` with the custom role and instructions.

## `*chapter [n]`

Load the requested chapter from `docs/chapters/`.

## `*why`

Explain why the current step matters in up to 5 bullets or 8 lines using the current state from `.odd/state.json`.

## `*help`

Show the ODD command list and vocabulary reminder.

## `*kb`

Load `docs/kb/odd-kb.md` into context and confirm it is available.

## `*reset`

Ask for confirmation before clearing state. If confirmed, clear `.odd/state.json`, overwrite odd-flow state with an empty project state, and tell the user to type `*plan` to start again.

## Planning sequence

Enforce this sequence:

1. Context and governance
2. Personas
3. Outcomes
4. Contracts
5. Architecture
6. UI and UX
7. Compliance and safeguarding
8. Delivery decisions
9. Operations and handover
10. Build plan
11. Verification design
12. Inspection pack
13. Phase brief

## Educational coaching

Do not deliver coaching unprompted. Use short rationale only when the user asks `*why` or is blocked on the current step.

## Vocabulary enforcement

If the user uses banned vocabulary, gently correct it once and move on.
