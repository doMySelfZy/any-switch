# Chapter 16: Managing Change

Change in ODD is cheap because the scope is explicit. Trellis knows which outcomes consume which contracts. A changed contract flags only consuming outcomes for re-verification. Non-consuming outcomes are untouched.

Change is inevitable. Requirements evolve, users give feedback, you learn things during the build that you could not have known before. The question is not whether change will happen but how expensive it will be. In ODD, the answer is: it depends on contract dependencies, and those are mapped.

## Key Principles

- **The scope of a change is determined by contract dependencies, not by intuition.** When something changes, your instinct might be to re-check everything. But the contract map tells you exactly what is affected. If you change a booking contract, only outcomes that consume the booking contract need re-verification. Everything else is structurally unaffected.

- **Change is expensive without a methodology because nothing records what depends on what.** In a traditional project, a change to one part might affect any other part. In ODD, the contract map is the dependency record. It tells you the blast radius of every change.

- **Specification changes and verification failures are different things.** A specification change means the requirement has changed. A verification failure means the build did not match the specification. A specification change triggers an outcome update and rebuild; a verification failure triggers a fix and re-verification.

- **Changed outcomes propagate through the contract map.** If a changed outcome produces a different contract than before, every outcome consuming that contract is flagged for re-verification. Trellis handles this tracking automatically.

## Red Flags

- Re-verifying outcomes that don't consume the changed contract. This is wasted effort. Trust the contract map. If an outcome does not consume the changed contract, it is not affected.

- Treating a specification gap as a verification failure. If during verification you realise the specification itself is wrong — not that the build failed to match it, but that the specification does not capture what you actually need — that is a specification change. Update the outcome, rebuild, then re-verify.

- Avoiding change because it feels expensive. Check the contract map first. The actual scope may be much smaller than the felt scope.

- Making changes without updating the specification. If you fix something without updating the outcome, the specification and system are out of sync. The next build will reintroduce the problem.

## What This Means for You

When a change is needed — and it will be — start with the contract map. Identify which contract is affected. Update the specification for the outcome that produces it. Rebuild. Then re-verify only the outcomes that consume it. Trellis flags these for you.

Change is not a failure of planning. It is a sign that you are learning. The methodology makes it cheap.

Next: Chapter 17 goes deeper into the swarm — how it works, why it fails, and what to do about it.
