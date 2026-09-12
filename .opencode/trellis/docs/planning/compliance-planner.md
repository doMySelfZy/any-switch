# Compliance Planner

This stage defines how the system becomes safe, defensible, and reviewable.

Interaction rules:

- Keep framing short.
- Ask for one risk or control decision at a time.
- Persist the approved compliance model before moving on.

Resolve:

- compliance regime profile
- child-identifiable data handling
- storage mode and deployment region
- retention rules
- review gate triggers
- automated decision points

Create or update:

- `docs/compliance/regime-profile.md`
- `docs/compliance/compliance-contract.md`
- `docs/compliance/review-gates.md`
- `docs/compliance/data-handling.md`
- `docs/compliance/retention-policy.md`
- `docs/compliance/manifest.json`
- `.odd/trellis-state.json`

If the app touches high-stakes decisions, make the review-gate design explicit before moving on.

Persist with:

```bash
trellis planning-action record-compliance --json '{
  "regimeProfile": "uk-schools",
  "childIdentifiableData": true,
  "highStakesDecisions": true,
  "externalAccess": {
    "parents": false,
    "students": false
  },
  "storageMode": "m365-tenancy",
  "deploymentRegion": "uk",
  "retentionPolicies": ["Provision records retained per school policy"],
  "reviewGateTriggers": ["Provision change", "Sensitive note edit"],
  "manifestVersion": "v1"
}'
```

Payload notes:

- Keep `externalAccess` explicit even when both values are `false`.
- Use `reviewGateTriggers` for actions that must not complete silently.
