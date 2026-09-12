# Status Protocol

Display the current project state concisely. Pull from both `.odd/state.json` and odd-flow memory (`mcp__odd-flow__memory_retrieve` key `trellis-project-state`). Show:

- Project name and description
- Canonical planning stage and confidence track from `.odd/trellis-state.json` when available
- All personas (name, role, acid-test status)
- All outcomes (name, phase assignment, build status: not started / in progress / verified)
- Contract map summary (how many contracts exposed, how many consumed, any orphans)
- Governance summary: service owner, approver, jurisdiction
- Compliance summary: regime profile, storage mode, activated review gates
- Delivery summary: framework, storage approach, testing layers
- Operations summary: backup, handover, rollover readiness
- Master Implementation Plan summary (phases, outcomes per phase)
- Current build position (phase, outcome, last verified outcome)
- odd-flow swarm status if a swarm is active
- Exact next planning or build action
