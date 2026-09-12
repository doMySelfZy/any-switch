# Confirm Protocol

The domain expert types `confirm` when all verification steps for the current outcome have passed on a single complete run.

## Verification gate

Before `confirm` can execute:

1. Every numbered verification step from the session brief must have been presented in order
2. Each step must have been tested
3. The domain expert must have confirmed each step passes
4. `verificationConfirmed` must be `true` in `.odd/state.json`

This is enforced by the verify gate. `verificationConfirmed` must be reset to `false` at the start of each new outcome's verification.

## What verification looks like

- Present each verification step in order
- For browser-testable steps, use Playwright to check the UI
- For multi-user steps, note them as deferred only with explicit agreement
- For absence checks, verify the UI does not contain the element
- Report each step as PASS / FAIL / DEFERRED

## Execute in order

1. Commit the verified state via git with message: `feat: verified [outcome name] — [phase]`
2. Call `mcp__odd-flow__memory_store` key `trellis-outcome-[name]` with status `verified`, namespace `trellis-project`
3. Update `.odd/state.json`: mark outcome as verified and set `nextStep`
4. Call `mcp__odd-flow__memory_store` key `trellis-project-state`, namespace `trellis-project`, value set to the full updated `.odd/state.json`

Then display:

---

**[Outcome name] — verified and committed.**

**Next:** [next outcome name and one-sentence description]

Type `*build` to begin, or `*status` to see the full phase progress.

---
