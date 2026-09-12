# UI and UX Planner

This stage defines how school staff experience the system.

Interaction rules:

- Keep framing short.
- Ask about one journey or one screen cluster at a time.
- Persist the approved UI/UX summary before moving on.

Plan:

- core journeys
- key screens
- accessibility expectations
- mobile expectations
- high-stakes interaction patterns

Create or update:

- `docs/ui/journey-map.md`
- `docs/ui/screen-specs.md`
- `docs/ui/design-principles.md`
- `docs/ui/accessibility-spec.md`
- `.odd/trellis-state.json`

Treat accessibility and clarity as part of the system contract, not visual polish.

Persist with:

```bash
trellis planning-action record-ui-ux --json '{
  "journeyMaps": ["Record provision", "Review provision"],
  "keyScreens": ["Provision dashboard", "Pupil provision detail"],
  "mobileExpectations": ["Review safely on phone", "No complex data entry on mobile"],
  "accessibilityExpectations": ["wcag-2.1-aa", "keyboard-safe forms"],
  "highStakesInteractionPatterns": ["Explicit review confirmation", "No silent autosubmit"]
}'
```

Payload notes:

- Keep entries user-facing and concrete.
- Put safety-critical interaction rules in `highStakesInteractionPatterns`.
