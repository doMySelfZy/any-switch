# Startup Protocol

Before doing anything else, run this state check silently:

1. Check whether `.odd/state.json` exists in the current working directory.
2. Check whether `.odd/trellis-state.json` exists in the current working directory.
3. Check whether `docs/plan.md` exists.
3. Attempt to retrieve project state from odd-flow memory:
   - Call `mcp__odd-flow__memory_retrieve` with key `trellis-project-state`, namespace `trellis-project`
4. **Reconciliation — strict, no silent merging.** If both local and odd-flow state exist, compare them field-by-field. Specifically check `currentBuildPhase`, `currentPhase`, `briefConfirmed`, `sessionBriefCount`, `personas.length`, and `outcomes.length`. If `.odd/trellis-state.json` exists locally, also compare `currentStage`, `project.technicalConfidence`, and `activatedModules.length`. If ANY of these disagree:
   - **STOP.** Do not display the welcome or status message yet.
   - Show the user a side-by-side diff of the disagreeing fields (local value vs odd-flow value).
   - Ask explicitly: "Local state and odd-flow state have drifted. Which should I trust as authoritative? Type `local`, `odd-flow`, or `inspect` to see the full diff."
   - On `local`: store the full local `state.json` to odd-flow with `mcp__odd-flow__memory_store` and proceed.
   - On `odd-flow`: write the odd-flow value to `.odd/state.json` and proceed.
   - On `inspect`: print the full diff and ask again.
   - Do NOT use heuristics like "richer wins" or "later phase wins" — they hide bugs. The user decides.
5. If local exists and odd-flow does not: store local to odd-flow immediately. If odd-flow exists and local does not: write odd-flow to local immediately.

If this is a new project, display this welcome message:

---

Welcome to Trellis v1.0.1.

We plan before we build. Trellis will keep this tight:

- capture school context and ownership
- define personas and outcomes
- map contracts, safety controls, and delivery choices
- generate the build plan

Default interaction mode:
- ask one focused question at a time
- keep summaries short
- explain in depth only if the user asks `*why`

Type `*plan` to begin, or `*help` to see commands.

---

If this is a returning project, display this status message with real values from `.odd/state.json`:

---

Welcome back to Trellis v1.0.1.

**Project:** [project.name]
**Current Phase:** [state.currentPhase]
**Last Session:** [state.lastSessionDate]

**Planning Stage:** [trellis.currentStage]

**Progress:**
- School context and governance: [trellis.governanceReady]
- Personas: [personas.length] documented ([personas.approved] approved)
- Outcomes: [outcomes.length] written ([outcomes.approved] approved)
- Contracts: [contractsMapped ? "Mapped" : "Not yet mapped"]
- Architecture and UI/UX: [trellis.architectureReady]
- Compliance and operations: [trellis.complianceReady]
- Master Plan: [planApproved ? "Approved" : "Not yet created"]

**What's next:** [state.nextStep]

Type `*plan` to continue planning, `*build` to enter build mode, `*debug` to investigate a failing outcome, or `*status` for full detail.

---

## Mandatory odd-flow checkpoints

These are non-negotiable tool executions. You MUST call these odd-flow tools — not describe them, not summarise them. Actually invoke the tool at each gate.

| Gate | Tool call required | When |
|---|---|---|
| Governance/context captured | `mcp__odd-flow__memory_store` key `trellis-project-context` namespace `trellis-project` | Immediately after the context and governance stage is confirmed |
| Persona approved | `mcp__odd-flow__memory_store` key `trellis-persona-[name]` namespace `trellis-project` | Immediately after Diana marks a persona approved |
| Outcome approved | `mcp__odd-flow__memory_store` key `trellis-outcome-[name]` namespace `trellis-project` | Immediately after Marcus marks an outcome approved |
| Contract map complete | `mcp__odd-flow__memory_store` key `trellis-contract-map` namespace `trellis-project` | Immediately after Theo completes the contract map |
| Architecture approved | `mcp__odd-flow__memory_store` key `trellis-architecture` namespace `trellis-project` | Immediately after the architecture stage is confirmed |
| Compliance model approved | `mcp__odd-flow__memory_store` key `trellis-compliance` namespace `trellis-project` | Immediately after the compliance stage is confirmed |
| Delivery decisions approved | `mcp__odd-flow__memory_store` key `trellis-delivery` namespace `trellis-project` | Immediately after the delivery stage is confirmed |
| Operations and handover approved | `mcp__odd-flow__memory_store` key `trellis-operations` namespace `trellis-project` | Immediately after the operations stage is confirmed |
| Plan approved | `mcp__odd-flow__memory_store` key `trellis-plan` namespace `trellis-project` | Immediately after Rachel's plan is approved |
| Design approach decided | `mcp__odd-flow__memory_store` key `trellis-design-approach` namespace `trellis-project` | Immediately after Rachel's design conversation completes |
| Phase brief confirmed | `mcp__odd-flow__memory_store` key `trellis-session-brief-[N]` namespace `trellis-project` | After domain expert confirms the phase brief, before building starts |
| State update (all stages) | Write updated `.odd/state.json` | After every persona, outcome, or plan approval |
| Session start | `mcp__odd-flow__memory_retrieve` key `trellis-project-state` namespace `trellis-project` | Before displaying any welcome or status message |
| Build work complete | `mcp__odd-flow__memory_store` key `trellis-project-state` namespace `trellis-project` | After any build, fix, or debugging work completes |
| Session end | `mcp__odd-flow__memory_store` key `trellis-project-state` namespace `trellis-project` | Before ending any session |

## Session end protocol

Before any session ends — whether the user says goodbye, the context is running low, or the conversation is closing — you MUST:

1. Read the current `.odd/state.json`
2. Read `.odd/trellis-state.json` if it exists
3. Call `mcp__odd-flow__memory_store` with key `trellis-project-state`, namespace `trellis-project`, value set to the full contents of `.odd/state.json`
4. If `.odd/trellis-state.json` exists, also call `mcp__odd-flow__memory_store` with key `trellis-project-canonical-state`, namespace `trellis-project`, value set to the full contents of `.odd/trellis-state.json`
5. Confirm to the user: "Session state saved to odd-flow."

If odd-flow is not available, continue without it and tell the user that cross-session memory will not persist this session.
