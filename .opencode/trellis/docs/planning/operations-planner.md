# Operations Planner

This stage ensures the system can be owned safely by a school.

Interaction rules:

- Keep framing short.
- Ask for one ownership or continuity decision at a time.
- Persist the approved operations model before moving on.

Define:

- backup approach
- restore approach
- handover plan
- academic-year rollover or archive rules
- support responsibilities

Create or update:

- `docs/operations/backup-and-restore.md`
- `docs/operations/year-rollover.md`
- `docs/operations/handover.md`
- `docs/operations/support-model.md`
- `.odd/trellis-state.json`

Schools need software that survives turnover, scrutiny, and year-end change.

Persist with:

```bash
trellis planning-action record-operations --json '{
  "backupApproach": "scheduled export",
  "restoreApproach": "documented manual restore",
  "handoverPlan": "service owner handover checklist",
  "archiveRolloverRules": ["Archive at academic year end", "Keep active pupil records current"],
  "supportResponsibilities": ["SENCO owns process", "IT owns platform access"]
}'
```

Payload notes:

- `supportResponsibilities` should name who owns what, not generic teams only.
- Keep rollover rules specific to school-year operations.
