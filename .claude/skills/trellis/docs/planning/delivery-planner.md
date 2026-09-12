# Delivery Planner

This stage binds implementation choices to the already-decided safety model.

Interaction rules:

- Keep framing short.
- Decide one layer at a time.
- Persist the approved delivery choices before moving on.

Decide and document:

- framework
- UI system
- data access mode
- ORM or adapter strategy
- testing layers
- hosting target
- email or notification provider

Create or update:

- `docs/delivery/stack-decision.md`
- `docs/delivery/testing-strategy.md`
- `docs/delivery/data-access-strategy.md`
- `docs/delivery/deployment-strategy.md`
- `docs/delivery/integration-strategy.md`
- `.odd/trellis-state.json`

Do not let delivery choices override the compliance contract.

Persist with:

```bash
trellis planning-action record-delivery --json '{
  "framework": "nextjs",
  "uiSystem": "shadcn",
  "dataAccessMode": "repository-adapter",
  "orm": "drizzle",
  "testingLayers": ["browser", "contract", "unit"],
  "hosting": "vercel",
  "emailProvider": "resend"
}'
```

Payload notes:

- Use `dataAccessMode` even when an ORM is present.
- Omit `orm` entirely when the chosen storage path is adapter-only.
