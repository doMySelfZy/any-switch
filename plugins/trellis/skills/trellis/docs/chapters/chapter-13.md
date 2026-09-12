# Chapter 13: Concurrent Outcomes and the Swarm

`*swarm` builds all independent outcomes in the current phase simultaneously. Four agents — coordinator, backend, UI, QA — work from the same contract map. Your role is unchanged: verify each outcome, describe failures in domain language.

The swarm is not a different methodology. It is the same methodology — build, verify, confirm — applied to several outcomes at once. The contract map guarantees that outcomes within a phase have no dependencies on each other, so they can be built in parallel without conflicts.

## Key Principles

- **The swarm is not an advanced feature. It is the same methodology applied to several outcomes at once.** If you can build one outcome, you can use the swarm. The process is identical: each outcome gets built, each outcome gets a verification checklist, you verify each one. The only difference is that the builds happen in parallel instead of sequentially.

- **Four agents, clear roles.** The coordinator manages build order and resolves conflicts. The backend agent builds data and logic. The UI agent builds the interface. The QA agent runs automated checks. They work from your specification automatically.

- **Contract conflicts are resolved by the coordinator.** When two outcomes touch related contracts, the coordinator ensures consistency. You see the result in verification, not during the build.

## Red Flags

- Trying to prevent contract conflicts before the swarm runs. The coordinator handles this. Your job is to write precise specifications and verify the results. Trying to micromanage the agents' work is solving a problem that the tool already solves.

- Routing failures to specific agents. If a verification fails, describe what went wrong in domain language. "The booking confirmation shows the wrong date." Do not say "the backend agent stored the wrong value." The coordinator determines which agent needs to address the issue.

- Using the swarm for outcomes with dependencies. The swarm builds outcomes within a single phase — outcomes that are independent by definition. If you try to swarm outcomes from different phases, dependent outcomes will fail because the contracts they consume do not exist yet.

- Skipping verification because the swarm "handles quality." The QA agent runs automated tests. Automated tests confirm the code does what it was told. You still need to confirm it was told the right thing. Verification is still your job.

## What This Means for You

When you reach a phase with multiple outcomes, type `*swarm` instead of `*build`. The swarm builds all outcomes in the phase. You receive a verification checklist for each one. Verify each outcome individually, in any order. Describe failures in domain language. Confirm when all outcomes pass.

The swarm does not change what you do. It changes how much gets done in each session.

Next: Chapter 14 addresses the parts of building software that feel intimidating — security, authentication, and data protection — and shows they are just outcomes like any other.
