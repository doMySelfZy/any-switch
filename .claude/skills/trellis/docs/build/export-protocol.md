# Export Protocol

Generate the IDE Session Brief. This is a standalone document that a developer or AI coding agent can use to execute a build session without needing to ask planning questions.

1. Read `sessionBriefCount` from `.odd/state.json` (default 0 if not set).
2. Read `docs/plan.md` to identify which phase has outcomes not yet briefed.
3. Load all outcome files from `docs/outcomes/` for that phase.
4. Generate `docs/session-brief-[N].md` following the Session Brief structure in `docs/planning/build-planner.md` Step 10.

The brief must include:

1. Overview
2. Active Personas This Phase
3. Outcomes In Scope, with the complete six-field specification for each outcome
4. Available From Previous Phases
5. New Tables Required This Phase
6. Build Sequence
7. Known Failure Paths
8. Not In Scope
9. Infrastructure Notes
10. Design System Reminder
11. Changes From Original Plan

Validation gate before presenting the brief:

- Every outcome has its full walkthrough
- Every outcome has its complete verification checklist
- Every outcome has its contracts listed with field names
- The brief stays concise and execution-focused
- The brief is usually no longer than 200 lines of prose; if it runs long, remove repeated explanation and point agents to the canonical outcome files instead

If the brief fails validation, regenerate it.

Present the brief to the domain expert and wait for confirmation. If they request changes, update it and re-present.

Once confirmed:

- increment `sessionBriefCount`
- update `currentBuildPhase`
- update `currentPhase` to `build`
- set `briefConfirmed: true`
- store the brief in odd-flow memory with key `trellis-session-brief-[N]`, namespace `trellis-project`

Then display:

`Session Brief [N] confirmed and written to docs/session-brief-[N].md. Build gate unlocked. The build can now begin.`
