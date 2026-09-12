# Trellis — Build Protocol

The Build Protocol is the discipline that turns a well-written ODD plan into working software. Trellis handles all the mechanics — context, contracts, phase tracking, re-briefing, committing. The domain expert does one thing: verify that what was built is right for real users, and describe failures in plain language.

---

## The Session Rhythm

Every build session follows the same three steps. The tool manages continuity between sessions through odd-flow memory. The domain expert never re-briefs the AI, tracks state manually, or writes handover notes.

---

### Step 1 — Open the project

Type `/trellis`.

Trellis reads the full project state from odd-flow memory — personas, outcomes, contracts, phase order, verification status. It reports exactly where the build stands and what comes next. If the last session ended three days ago or three weeks ago, the state is the same. The domain expert does not need to remember anything.

---

### Step 2 — Build

For a single outcome: type `*build`.

For multiple independent outcomes in the current phase: type `*swarm`.

Trellis briefs the build AI with the full six-field specification, the relevant contracts, the context from previous outcomes, and the Code Excellence standard from `docs/build/code-excellence.md`. The domain expert waits. The build takes minutes.

**The Design-It-Twice rule applies to every build.** The build agent writes each solution twice internally — first to make it work, then to make it minimal. Only the second pass is committed. This is not optional. Verbose, over-abstracted code is harder to verify, harder to debug, and harder to change. The Code Excellence standard defines the specific constraints: 25-line function limit, 3-level nesting maximum, no premature abstractions, no defensive code for scenarios the outcome does not describe. Every line earns its place.

The domain expert does not re-brief the AI, paste context, identify shared infrastructure, or check dependencies. Trellis handles all of that from odd-flow memory.

### Step 2a — Build Execution Loop

The orchestrator must execute the build. Do not stop at odd-flow swarm setup, and do not treat `mcp__odd-flow__agent_spawn` as the code-writing step. Those calls only register coordination metadata.

After `coordination_sync` succeeds, run this loop:

1. Read the active session brief and identify the outcomes in scope for this build session.
2. For each in-scope outcome, break the work into concrete implementation stages. Use the smallest set that fits the outcome:
   - data/contracts
   - business logic/services
   - UI/screens
   - tests
   - verification fixes
3. Dispatch the real implementation work with the runtime executor agent tool. In Claude/Codex-style runtimes, this is the `Agent` tool. Use implementation agents for code changes and wait for each one to finish before moving to the next dependent stage.
4. After each agent completes, inspect the returned summary, review the changed files, and decide the next stage:
   - if the stage is complete, dispatch the next dependent stage
   - if the stage exposed a gap or regression, dispatch a follow-up fix agent before continuing
   - if stages are independent, you may dispatch them in parallel only when they do not write the same files
5. Continue until the outcome implementation and tests are complete, then run the test gate below.

Never end a turn with "Dispatching Stage [N]" unless the `Agent` tool call has already been made in that same turn.

**Executor prompt template**

Use this shape for each implementation dispatch. Keep the stage brief under roughly 200 lines of prose. Bias toward immediate writing work, not a long design essay.

```text
You are implementing Stage [stage name] for Outcome [number]: [outcome name].

Read first:
- docs/plan.md
- docs/outcomes/[active outcome file]
- docs/session-brief-[n].md (use only the sections needed for this stage)
- docs/contract-map.md (if this stage produces or consumes contracts)
- docs/ui/design-system.md (if this stage touches UI)
- docs/build/code-excellence.md

Rules:
- Stay inside this stage and the active outcome only.
- Design it twice: first working pass, then minimal pass.
- Write first, verify after. Do not spend the stage restating the brief back to the orchestrator.
- Do not broaden scope or edit unrelated files.
- Write or update tests for any pure business logic you introduce.
- Report any spec gap in plain language before guessing.

Deliver:
- implement the stage
- summarise files changed
- report tests run and results
- call out any follow-up stage the orchestrator should run next
```

For `*swarm`, run the same loop per independent outcome. Keep one executor agent per outcome track, then converge on QA and verification after implementation agents finish.

---

### Step 2b — Test

After the build completes and before verification begins, the build agent runs the test suite automatically.

**What the build agent tests:**

Every outcome produces code. Some of that code is pure logic — functions that take inputs and return outputs without touching databases, APIs, or the browser. These functions MUST have tests written alongside the implementation. The build agent writes tests for:

- **Business rules** — access control, pricing, eligibility, classification logic
- **Data transformations** — formatting, aggregation, filtering, sorting
- **Validation** — input parsing, CSV import, form validation, regex matching
- **Calculations** — mastery scores, scheduling, priority ordering, time-based logic
- **Safety-critical logic** — safeguarding detection, content filtering, concern routing

**What is NOT tested at this stage:**

- Database queries (tested via verification walkthrough)
- UI rendering (tested via verification walkthrough)
- External API calls (tested via verification walkthrough)
- LLM prompt/response cycles (tested via verification walkthrough)

**The test gate:**

After the build completes, run `npm test`. If any tests fail:
1. The build agent fixes the failures immediately — do not proceed to verification with failing tests
2. Re-run `npm test` until all tests pass
3. Display to the domain expert: "All [n] tests passing. Ready for verification."

If no testable pure logic was produced by this outcome (e.g., a purely UI outcome), display: "No new business logic tests required for this outcome. Ready for verification."

Tests are committed alongside the implementation code. They live in `tests/` mirroring the source structure. Test files must never be deleted — they are regression guards for every future outcome.

---

### Step 3 — Verify

When Trellis reports the build is complete, the verification checklist is on screen.

Follow each step in order. Follow them as the persona — not as yourself reviewing a system, but as the specific person in the specific situation the outcome was written for.

**Record each step: pass / fail / missing.**

**Verify the failure paths.** These are not optional. They are where the AI made assumptions — what to show when the event is full, what to do when payment fails, what happens when the session expires. The happy path is what the AI builds most reliably. The failure paths are where assumptions accumulate.

**An outcome is verified when every step passes on a single complete run** — not when each step has passed at some point across multiple attempts.

---

### Design Verification (runs after functional verification passes)

Functional verification answers "does it work?" Design verification answers "does it look and feel right?" Both must pass before an outcome can be confirmed. This step prevents the most common failure mode in AI-built software: features that work correctly but are unreachable, inconsistent, or unusable because the navigation, layout, or visual hierarchy was not enforced.

**Record each check: pass / fail.**

1. **Navigation consistency:** Is the persistent navigation (sidebar, header, bottom nav) visible on this page? Can the user reach every primary destination without using the browser back button? If this page is a dead end — if the user cannot get to the feed, their profile, or notifications without browser back — it fails.

2. **Layout match:** Does this page follow the layout pattern specified in CLAUDE.md? If the specification says "three-column with sidebar on desktop, bottom nav on mobile", does this page render inside that structure?

3. **Component consistency:** Are buttons, inputs, cards, and forms using the component library (shadcn/ui) and the design tokens from the design system? Ad-hoc styled HTML elements that ignore the design system are a fail.

4. **Responsive check:** Does this page work at 375px (mobile), 768px (tablet), and 1280px (desktop)? Content must not overflow, text must not be unreadably small, touch targets must be at least 44px on mobile.

5. **Visual hierarchy:** Can you scan the page and immediately identify: (a) what page you are on, (b) the primary action, (c) the content? If the page is a wall of same-sized text with no clear entry point, the visual hierarchy fails.

6. **Design system compliance:** Are colours, spacing, border radius, and typography consistent with the design tokens in CLAUDE.md? Check that the page does not introduce ad-hoc hex values, inconsistent padding, or font sizes outside the established scale.

If any design verification step fails, describe the failure in domain language — what the persona sees that feels wrong, not what CSS class is missing — and fix before confirming.

**Design verification failures are described the same way as functional failures:**

- "I am on the write page and there is no way to get back to my feed without pressing the browser back button."
- "On my phone, the navigation items are tiny text links at the top of the page that I cannot tap accurately."
- "Every other page has the sidebar, but this page does not — it feels like a different website."
- "The buttons on this page are a different shape and colour from every other page."

---

### Describing failures

When a step fails, describe it in domain language — what the person sees, what should happen instead. Not technical causes. Not error codes.

**Good descriptions:**
- "The confirmation email arrives but there is no calendar invitation. I cannot add the event to my calendar from the email."
- "When I try to book an event that already has no remaining places, the booking goes through without any warning."
- "After I cancel my booking, my account still shows it as upcoming. It only disappears when I refresh the page."

Collect all failures from the verification run and send them in a single message. Trellis re-briefs the AI with the failures and triggers a fix. After the fix, re-verify from step one — all steps, not just the ones that failed.

---

### Confirming

When all steps pass on a single complete run, type `confirm`.

Trellis commits the verified state, updates odd-flow memory, and presents the next outcome.

The domain expert did not write a commit message, update a status file, identify security issues, or decide what comes next. The tool handled all of that.

---

## Integration Protocol

After all outcomes in a phase are individually verified, Trellis runs the Integration Protocol automatically. The domain expert does not initiate this — the post-phase hook triggers it when the last outcome in the phase is confirmed.

Four checks run:

**Handshake check.** For each pair of outcomes that share a contract, confirm the data passing between them is correct in domain terms. Does the organiser's view show the correct customer details from the booking? Does the refund outcome have access to the payment details the booking outcome recorded?

**Data flow trace.** Follow one entity — a customer, a booking, an event — through every outcome in the phase. Confirm the data is consistent. The customer's name should be the same everywhere. The booking reference should appear wherever it should appear.

**Cross-persona check.** Confirm each persona sees what they should see and cannot access what they should not. Navigate as a customer to a page that should only be accessible to an organiser. Confirm it is blocked.

**Navigation integration check.** Navigate between every outcome in this phase using only the platform's built-in navigation — sidebar, header, bottom tab bar. No browser back button. No typing URLs. Start on the feed page and navigate to every screen built in this phase, then navigate back. Every page must be reachable within two taps. If any page is a dead end — if the user has to use browser back or type a URL to leave it — the integration check fails. This is the check that prevents "every feature works but the platform feels broken" — the exact failure mode that occurs when outcomes are built in isolation without a shared navigation architecture.

When all four checks pass, the phase is complete. Trellis runs the Phase Transition procedure.

---

## Phase Transition

When a phase is complete and all integration checks pass, Trellis advances to the next phase. This is a four-step process: mark complete, reconcile the plan, generate the next numbered brief, and confirm with the domain expert.

---

### Transition Step 1: Mark the current phase complete

Update `.odd/state.json`:
- Set the current phase status to `"complete"`
- **Set `briefConfirmed` to `false`** — this re-locks the build gate for the next phase. The next phase's brief must be generated, reviewed, and confirmed before any build agents can be spawned.
- Update `lastSessionDate`

Store completion in odd-flow memory:

Call `mcp__odd-flow__memory_store`:
- Key: `odd-phase-[phase-name]-complete`
- Namespace: `trellis-project`
- Value: phase name, completion date, outcomes verified, integration checks passed

---

### Transition Step 2: Plan Reconciliation

During the build, things change. Specification gaps are discovered. The domain expert requests new features. A contract turns out to need a different shape. An outcome gets split into two. A service that was planned for a later phase gets pulled forward because the current phase needed it.

These changes happen *during* the build — they are recorded in odd-flow memory as specification gaps, outcome updates, and contract changes. But `docs/plan.md` still reflects the original plan. If the next phase's brief is generated from a stale plan, it will conflict with reality.

**Before generating the next phase's brief, reconcile the plan.**

**2a. Gather all changes from the completed phase.**

Read from odd-flow memory all changes recorded during the phase:

Call `mcp__odd-flow__memory_search`:
- Query: `phase [completed-phase-name] changes`
- Namespace: `trellis-project`

Also check:
- All outcome files in `docs/outcomes/` — compare current versions against the plan's original descriptions. Look for updated walkthroughs, new verification steps, changed contracts.
- The contract map in odd-flow memory (`trellis-contract-map`) — compare against `docs/contract-map.md`. Look for new contracts, changed data shapes, removed dependencies.
- Any `*outcome` edits made during the phase (specification gap fixes).
- Any new outcomes added during the build that were not in the original plan.

**2b. Classify each change.**

For each change found, classify it:

1. **Contained change** — affects only the completed phase. No impact on future phases. Example: a verification step was reworded, a UI layout was adjusted, an error message was improved.

2. **Contract change** — a contract's shape, name, or data flow was altered. This affects any future outcome that consumes this contract. Example: the mastery level contract now includes a `confidence_score` field that wasn't in the original plan.

3. **New outcome** — a new outcome was identified during the build that needs to be added to a future phase. Example: during Phase 1, the domain expert realised students need a "review mistakes" feature that wasn't originally planned.

4. **Moved outcome** — an outcome originally in a later phase was partially or fully built during this phase because it was needed earlier than planned. Example: basic parent notifications were built in Phase 1 because the teacher dashboard needed to reference them.

5. **Removed or deferred outcome** — an outcome was removed from scope or moved to a later phase. Example: the gamification system was deprioritised to focus on core learning.

6. **Dependency change** — a dependency between phases changed. Example: Phase 3 no longer depends on Phase 2's grouping feature because the parent dashboard was redesigned to use individual student data instead.

**2c. Update `docs/plan.md`.**

For each change that is NOT contained (types 2-6), update `docs/plan.md`:

- **Contract changes**: Update the contract descriptions in the plan. Note what changed and why.
- **New outcomes**: Add them to the appropriate phase. Re-run the dependency logic — does this new outcome depend on something that hasn't been built yet? If so, assign it to the correct phase.
- **Moved outcomes**: Update their phase assignment. Mark what was already built and what remains.
- **Removed/deferred outcomes**: Move them to a "Deferred" section at the bottom of the plan, with the reason.
- **Dependency changes**: Update the phase dependency descriptions.

Add a **Change Log** section at the bottom of `docs/plan.md`:

```markdown
## Change Log

### After Phase [completed phase name] — [date]
- [Change type]: [description of what changed and why]
- [Change type]: [description]
- [Change type]: [description]
```

**2d. Store the reconciled plan in odd-flow memory.**

Call `mcp__odd-flow__memory_store`:
- Key: `trellis-plan`
- Namespace: `trellis-project`
- Value: the full updated Master Implementation Plan

Call `mcp__odd-flow__memory_store`:
- Key: `trellis-plan-reconciliation-phase-[phase-name]`
- Namespace: `trellis-project`
- Value: summary of all changes made during reconciliation, with classification and reasoning

**2e. Present the reconciliation to the domain expert.**

"Before I generate the next phase's brief, I need to account for what changed during the build. Here is what I found:

**Changes that affect future phases:**
- [list each non-contained change with its classification and impact]

**Changes contained to the completed phase (no impact on future phases):**
- [list contained changes — for information only]

**Updated plan:**
- [summary of how `docs/plan.md` was updated]
- [any outcomes added, moved, removed, or deferred]
- [any contract shapes that changed]

Does this reconciliation look correct? Are there any other changes from this phase that I missed?"

Wait for the domain expert to confirm before proceeding to Step 3.

---

### Transition Step 3: Generate the next numbered Session Brief

Session briefs are numbered sequentially. The first brief is `session-brief-0.md`. Each subsequent phase gets the next number.

Read `.odd/state.json` to get the current `sessionBriefCount`. The next brief is `docs/session-brief-[N].md` where N is the current count.

Generate the brief from the **reconciled** `docs/plan.md` (not the original plan). The brief must reflect any changes from Transition Step 2.

**The brief MUST be generated using the `*export` command in SKILL.md.** This ensures every session brief follows the exact same mandatory structure with full 6-field specifications, complete verification checklists, contracts with field names, known failure paths, and all required sections. Do NOT generate a shortened or summarised version. Read all outcome documents from `docs/outcomes/` for the next phase and include their FULL content in the brief.

The brief follows the same structure as the `*export` command's mandatory structure, with these additions:

- **"Available From Previous Phases"** section listing all contracts and infrastructure produced by completed phases
- **"Changes From Original Plan"** section listing any reconciliation changes that affect this phase — new outcomes added, contracts that changed shape, dependencies that shifted. If none: "No changes — this phase matches the original plan."

Save to `docs/session-brief-[N].md`.

Store in odd-flow memory:

Call `mcp__odd-flow__memory_store`:
- Key: `trellis-session-brief-[N]`
- Namespace: `trellis-project`
- Value: the contents of the new session brief

Update `.odd/state.json`:
- Set `currentBuildPhase` to the next phase
- Increment `sessionBriefCount`

---

### Transition Step 4: Confirm the transition

"Phase [completed phase name] is complete. All outcomes verified. All integration checks passed.

[If reconciliation found changes]: The plan has been updated to reflect [N] changes from this phase. Review the updated plan at `docs/plan.md`.

Session Brief [N] has been generated for Phase [next phase name]: [phase description]. This phase contains [n] outcomes: [list outcome names].

[If changes affect this phase]: Note: this phase has been updated since the original plan — [brief summary of what changed].

All previous session briefs are retained:
[list: session-brief-0.md through session-brief-[N-1].md]

Review the Session Brief at `docs/session-brief-[N].md` and confirm before we build.

**Important:** The build will NOT start until you confirm the brief. This is a hard gate — no code will be written, no agents will be spawned, and no files will be created until the brief is reviewed and confirmed."

---

### Transition Step 5: If this was the final phase

If no phases remain, the project build is complete. Update `.odd/state.json`:
- Set `currentPhase` to `"complete"`
- Set `buildComplete` to `true`

Confirm:

"All phases are complete. Every outcome has been verified and every integration check has passed. Your project is built.

Session briefs retained: [list all numbered briefs].
Plan reconciliation history: [list all reconciliation entries from the change log].

Review the full system end-to-end against your original personas. Does Alex experience what you documented? Does Sarah see what you specified? Does Jennifer receive what you described? If yes, your ODD project is done."

---

## Handling failures

### Verification failure

A step fails. Describe what you see and what should happen instead. One message for all failures. Trellis routes the fix to the AI and triggers re-verification.

### Specification gap

Verification reveals not a specific failure but a wrong assumption about what the outcome should do — a case that was never specified, a requirement that was invisible until the system was working.

This is not a verification failure. It is a specification gap. Type `*outcome` to open the relevant outcome and update the field that was wrong. Trellis saves the updated specification to odd-flow memory and re-briefs the AI with the corrected version. Then rebuild and re-verify.

### When something is consistently wrong

If the same outcome fails verification repeatedly, the most likely cause is a specification problem — a walkthrough that left a case ambiguous, a verification step that cannot actually be passed as written. Return to `*outcome`, update the specification, and rebuild from the corrected version.

---

## Red flags

**"The build looks fine."** Fine is not verified. Fine means a visual scan was performed. An outcome that looks fine has not had its failure paths tested.

**"I tested the main flow."** The happy path was checked. The failure paths — where the AI made assumptions — were not.

**Describing failures in technical language.** "The API returns a 500 error" is a technical description. "When I try to book, the page shows a generic error message instead of telling me the event is full" is a domain description. Use the second.

**Skipping verification steps.** A step skipped because it seems fine is a step that has not been verified. The outcome is verified when all steps pass on the same complete run.
