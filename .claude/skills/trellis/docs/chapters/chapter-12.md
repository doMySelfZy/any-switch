# Chapter 12: Building One Outcome

One command, one build, one verification. This chapter is the methodology in action — a complete cycle from `*build` to `confirm`, showing exactly what happens at each stage and what your role is at each moment.

No new principles here. Everything has been introduced in earlier chapters. This chapter shows them working together in sequence.

## The Cycle

**Step 1: *build.** You type `*build`. Trellis reads the next outcome from your phase plan, loads the contract map, checks which contracts are available from previously built outcomes, and briefs the AI. The AI builds the outcome. You wait.

**Step 2: The checklist appears.** When the build completes, Trellis presents the verification checklist derived from the outcome's verification field. Each step corresponds to something you specified. The checklist is your specification, turned into a sequence of checks.

**Step 3: You verify as the persona.** Step into the persona's situation. Follow each checklist item in order. Check the actual result, not your expectation of it. If the persona is on a phone, verify on a phone.

**Step 4: Pass or fail.** If every step passes on a single complete run, the outcome is verified. If any step fails, describe the failure in domain language — what you expected to see and what you actually saw. Not "the API returned a 500." Instead: "I completed the booking but no confirmation email arrived."

**Step 5: Failure handling.** When you describe a failure, Trellis handles the fix. It re-reads the specification, makes the correction, and presents the checklist again. You verify again from the beginning — a partial re-verification does not count.

**Step 6: confirm.** When every step passes, you type `confirm`. Trellis commits the verified outcome to git, updates odd-flow with the new project state, and advances the plan to the next outcome. The cycle is complete.

## What You Do vs. What the Tool Does

**You do:** verify the result as the persona. Describe failures in domain language. Confirm when satisfied.

**The tool does:** load context, brief the AI, run the build, present the checklist, handle fixes, commit, update state.

## Red Flags

- Describing failures in technical language. "The database query is wrong" is not your job to diagnose. "The booking page shows last month's events instead of upcoming ones" is.

- Confirming without completing verification. Every unverified step is a hidden risk. Do not confirm until every step passes.

- Modifying the checklist during verification. The checklist comes from your specification. If the checklist is wrong, the specification needs updating — that is a separate step, not something to fix mid-verification.

## What This Means for You

This is the rhythm you will repeat for every outcome in your project. It is simple by design. The complexity is in the specification — which you have already done. The build cycle is just the execution.

Next: Chapter 13 introduces the swarm — the same cycle, applied to multiple outcomes simultaneously.
