# ODD Debug Protocol

Debugging does not sit outside Outcome-Driven Development. It is a controlled sub-mode of the current build.

When something fails during verification or during an in-progress build, use `*debug`. Do not abandon the active outcome. Do not start free-form fixing. Do not guess.

## Purpose

`*debug` exists to keep failure analysis inside the ODD flow:

- The failing outcome remains the active unit of work
- The investigation approach is chosen deliberately, not guessed
- The fix stays tied to the outcome walkthrough and contracts
- The work returns to verification when the defect is resolved

## Entry Conditions

Before debugging:

1. Read `.odd/state.json`
2. Confirm `currentPhase` is `"build"`
3. Identify the active outcome and the latest failure in domain language
4. Set these fields in `.odd/state.json`:
   - `buildMode: "debug"`
   - `verificationConfirmed: false`
   - `debugStartedAt: <timestamp>`
   - `debugSummary: <one-sentence failure in domain language>`
5. Store the updated state to odd-flow with key `trellis-project-state`

Debugging must never mark an outcome verified, complete, or committed.

## Strategy Selection

Choose exactly one debug strategy before inspecting code. State the chosen strategy and the reason.

Use this routing rule so the coding tool does not guess:

- Choose `ui-behaviour` when the problem is visible in the interface and you do not yet have evidence of a backend or data fault
- Choose `full-stack` when the failure crosses a user action, server boundary, and persisted state
- Choose `auth-security` when access, identity, trust, or validation boundaries might be wrong
- Choose `integration-contract` when one part of the system expects data or sequencing another part does not produce
- Choose `background-process` when the failure depends on async handoff, jobs, retries, or event delivery
- Choose `performance-state` when the issue depends on timing, staleness, cache invalidation, or repeated actions

If more than one strategy seems plausible, do not fix anything yet. Gather one more piece of evidence, then choose the narrowest strategy that still explains the failure.

### 1. `ui-behaviour`

Use when the failure is visible in the interface only:
- layout is wrong
- a button does nothing
- a view does not update
- a message or validation state is missing

Approach:
- reproduce in browser first
- inspect the rendered path backwards to the triggering action
- verify whether the contract is correct and the rendering is wrong

### 2. `full-stack`

Use when the failure spans browser, route, service, and data:
- a form submits but the result is missing
- a saved change does not appear
- a payment or enrolment looks complete but data is inconsistent

Approach:
- trace the full request path
- identify the first boundary where expected data diverges
- fix the smallest broken boundary, not the symptom

### 3. `auth-security`

Use when the defect touches access, trust, or sensitive behaviour:
- the wrong person can see or do something
- a protected route is open
- a webhook or upload path behaves unsafely
- a session, role, or permission check is wrong

Approach:
- verify actor, boundary, and expected restriction first
- inspect authentication, authorisation, validation, and side-effect points in order
- prefer the fix that narrows access and restores explicit checks

### 4. `integration-contract`

Use when two outcomes disagree about shared data or sequencing:
- one screen expects data another workflow never produces
- a downstream step fails because an upstream assumption changed

Approach:
- inspect the contract map and the active outcome contracts
- find the first producer/consumer mismatch
- fix the contract implementation or update the outcome if the specification is wrong

### 5. `background-process`

Use when the failure depends on queues, jobs, webhooks, scheduled work, or async delivery.

Approach:
- identify the triggering event
- confirm the worker/task started
- inspect the persisted state before and after the async boundary
- fix the handoff, retry, or idempotency break

### 6. `performance-state`

Use when the issue is stale data, race conditions, repeated actions, caching, or timing-sensitive state.

Approach:
- reproduce twice
- confirm whether the fault is deterministic or timing-sensitive
- inspect cache/state invalidation boundaries before changing business logic

## Non-Negotiable Rules

- Never use “quick fix” or “patch it” reasoning
- Never change multiple layers at once before reproducing the fault
- Never skip the reproduction step
- Never jump to a fix before naming the failing boundary
- Never broaden the strategy after starting unless new evidence proves the original classification wrong
- Never leave `buildMode: "debug"` active after the fix is complete

## Fix Protocol

After choosing the strategy:

1. Reproduce the failure
2. Name the failing boundary
3. Inspect only the layers required by the chosen strategy
4. Apply the smallest fix that restores the specified behaviour
5. Run the relevant automated checks
6. Set these fields in `.odd/state.json`:
   - `buildMode: "verify"`
   - `debugStrategy: <chosen strategy>`
   - `debugTarget: <affected outcome/surface>`
   - `debugSummary: <resolved failure summary>`
7. Store the updated state to odd-flow with key `trellis-project-state`
8. Return to the verification walkthrough from step one

If the investigation reveals that the specification is wrong, stop debugging and update the outcome instead. That is not a bug fix. That is a specification correction.
