# Chapter 17: The Swarm in Depth

The odd-flow swarm is the Build Protocol run in parallel by four specialised agents sharing a common specification. When the swarm produces wrong output, the cause is almost always a specification problem — incomplete contract map, vague walkthrough, ambiguous verification steps.

This chapter is for when the swarm produced wrong output. The instinct is to blame the tool. In almost every case, the root cause is the specification, not the agents.

## Key Principles

- **Specification quality determines swarm quality.** The swarm reads your outcomes, your contract map, and your verification steps. It builds from what you wrote. If what you wrote is vague, the swarm produces vague results. If the contract map has gaps, the agents make different assumptions about the missing data. Precise specifications produce precise builds.

- **The four agents have clear boundaries.** The coordinator manages sequencing and conflict resolution. The backend agent builds data and logic. The UI agent builds the interface. The QA agent runs automated verification. These boundaries are fixed.

- **Agent coordination follows the contract map.** The agents do not negotiate with each other. They read the contract map. If it says a booking record contains a customer name, event date, and ticket count, both backend and UI agents build to that specification. Conflicts come from ambiguous or missing contracts.

## Three Failure Sources

- **Incomplete contract map.** If a contract between two outcomes is not explicitly mapped, the agents producing and consuming that data have nothing to align on. They will each make assumptions. Those assumptions will conflict. The fix is not agent configuration — it is completing the contract map.

- **Vague outcome specification.** If the walkthrough says "the user sees their information," what information? The backend agent guesses one set of fields. The UI agent guesses another. The result is a mismatch. The fix is making the walkthrough specific.

- **Ambiguous verification steps.** If the verification says "check that the page displays correctly," the QA agent cannot write meaningful automated checks, and you cannot verify meaningfully either. Specify what "correctly" means: which elements, which data, which state.

## Red Flags

- Blaming agent configuration for incorrect output. If the output is wrong, the specification is the first place to look. Fix the specification first.

- Manually coordinating between agents. The contract map is the single source of truth. Bypassing it creates conflicts.

- Increasing the number of agents to solve quality problems. More agents do not fix specification gaps. They amplify them.

## What This Means for You

When a swarm build fails or produces unexpected results, resist the urge to troubleshoot the agents. Go back to the specification. Check the contract map for gaps. Check the walkthroughs for vagueness. Check the verification steps for ambiguity. Fix those, run the swarm again.

Next: Chapter 18 wraps up — the fundamental problem, what has changed, and where you go from here.
