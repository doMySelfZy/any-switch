# Chapter 8: The Master Implementation Plan

The phase structure is not a project management decision. It is a consequence of the system's structure — derived from the contract map. Phase A contains outcomes with no dependencies. Each subsequent phase builds on the last.

You do not decide the build order. The contract map decides it. An outcome that consumes a contract cannot be built before the outcome that produces it. Follow it and the build proceeds smoothly. Ignore it and outcomes fail because the data they need does not exist yet.

## Key Principles

- **The phase structure is not a project management decision. It is a consequence of the system's structure.** Look at your contract map. Outcomes that consume nothing from other outcomes go in Phase A. Outcomes that consume only Phase A contracts go in Phase B. Continue until every outcome has a phase. That is your build order.

- **A plan that changes is working. A plan that never changes was never consulted.** As you build and verify outcomes, you will discover specification gaps, new edge cases, and better approaches. The plan should update to reflect these discoveries. A static plan is a sign that no one is learning from the build.

- **Phases enable parallel work within them.** Outcomes in the same phase have no dependencies on each other — they can be built simultaneously. This is why the swarm (Chapter 13) works: it builds all outcomes in a phase at once.

- **Phase boundaries are verification checkpoints.** Before moving to the next phase, every outcome in the current phase should be verified. The next phase depends on the contracts the current phase produces. If those contracts are wrong, everything downstream will be wrong too.

## Red Flags

- Phase A with more than five outcomes. If your first phase has too many outcomes, some of them likely have hidden dependencies. Review the contract map — are any Phase A outcomes consuming contracts from other Phase A outcomes?

- Phases that share no contract connections. If Phase B outcomes do not consume any contracts from Phase A, why are they in Phase B? Either the phasing is wrong or the contract map is incomplete.

- A build order based on priority rather than dependencies. "We want payments first" sounds reasonable but ignores structure. If payments consume a booking contract, bookings must be built first regardless of priority.

- Skipping verification between phases. Moving to Phase B with unverified Phase A outcomes means building on unconfirmed foundations. Verify before advancing.

## What This Means for You

If you have a contract map, derive your phases from it now. Group outcomes with no consumed contracts into Phase A. Layer the rest by dependency. If you do not have a contract map yet, go back to Chapter 6 — you need it before you can plan.

Next: Chapter 9 covers the practical setup — `npx trellis-cli init` and what it gives you.
