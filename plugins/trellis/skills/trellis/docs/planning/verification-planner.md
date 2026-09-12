# Verification Planner

This stage defines how Trellis will know the system is genuinely correct.

Interaction rules:

- Keep framing short.
- Ask for one verification layer at a time.
- Persist the approved verification design before moving on.

Capture:

- browser-testable walkthroughs
- contract test expectations
- acceptance conditions
- unresolved risks

Create or update:

- `docs/verification/outcome-checks.md`
- `docs/verification/compliance-checks.md`
- `docs/verification/browser-walkthroughs.md`
- `.odd/trellis-state.json`

Write verification in school language, not implementation language.

Persist with:

```bash
trellis planning-action record-verification-design --json '{
  "browserChecks": ["SENCO can add provision", "Review date is visible after save"],
  "contractChecks": ["Provision record shape matches contract map"],
  "acceptanceConditions": ["No duplicate provision is created"],
  "unresolvedRisks": ["MIS import timing still to confirm"]
}'
```

Payload notes:

- Keep every check observable in domain language.
- Put remaining uncertainty in `unresolvedRisks`, not hidden in prose.
