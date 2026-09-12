# Inspection Pack Planner

This stage makes the system inspectable and handover-ready.

Interaction rules:

- Keep framing short.
- Ask for one missing inspection artifact at a time.
- Persist the inspection readiness summary before closing planning.

Prepare:

- system summary
- data summary
- access summary
- change log
- safeguards summary

Create or update:

- `docs/inspection/system-summary.md`
- `docs/inspection/data-summary.md`
- `docs/inspection/access-summary.md`
- `docs/inspection/change-log.md`
- `.odd/trellis-state.json`

The aim is to make the application explainable to leadership, DPOs, auditors, Ofsted, or the ICO.

Persist with:

```bash
trellis planning-action record-inspection-pack --json '{
  "dataSummaryReady": true,
  "accessSummaryReady": true,
  "changeLogReady": true,
  "safeguardsSummaryReady": true
}'
```

Payload notes:

- Use `true` only when the corresponding artifact is actually ready.
- If one area is still missing, persist the real partial state instead of assuming completion.
