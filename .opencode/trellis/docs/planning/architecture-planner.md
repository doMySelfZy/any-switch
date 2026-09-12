# Architecture Planner

This stage turns contracts into a school-safe system shape.

Interaction rules:

- Keep framing short.
- Ask for one decision at a time.
- Persist the approved architecture summary before moving on.

Decide and document:

- tenancy decision
- storage pattern
- role and access boundary
- audit boundary
- aggregation boundary
- recovery assumptions

Create or update:

- `docs/architecture/system-overview.md`
- `docs/architecture/data-model.md`
- `docs/architecture/storage-decision.md`
- `docs/architecture/integration-map.md`
- `.odd/trellis-state.json`

Architecture must remain derived from outcomes and contracts, not invented separately.

Persist with:

```bash
trellis planning-action record-architecture --json '{
  "tenancyDecision": "school-tenant",
  "storagePattern": "m365-sharepoint-adapter",
  "roleAccessModel": "school-rbac",
  "auditBoundary": "all sensitive changes logged",
  "aggregationBoundary": "school-level only",
  "recoveryAssumptions": ["Daily export", "Manual restore path"]
}'
```

Payload notes:

- Use the canonical architecture keys above.
- `recoveryAssumptions` should stay short and operational.
