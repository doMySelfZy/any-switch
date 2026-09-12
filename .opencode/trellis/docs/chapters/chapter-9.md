# Chapter 9: Start from Zero

`npx trellis-cli init` sets up everything: the /trellis skill, six safety hooks, project scaffold, and odd-flow persistent memory. odd-flow stores all project state between sessions so you never have to reconstruct context from scratch.

This chapter is about the practical starting point. You have your outcomes, your contract map, and your phase plan. Now you set up the environment that will carry you through the build. One command does it. Everything after that command is building and verifying.

## Key Principles

- **odd-flow preserves the project state between sessions so you never have to reconstruct it.** Every outcome, every contract, every verification result, every decision recorded in CLAUDE.md — odd-flow keeps it all. When you start a new session, Trellis reads from odd-flow and picks up exactly where you left off. You do not re-explain your project.

- **The six safety hooks protect the build.** They run automatically: pre-commit checks, contract validation, verification gating. You do not manage them. They prevent the kinds of errors that silently corrupt a project — committing unverified outcomes, breaking existing contracts, skipping steps.

- **Git is non-negotiable.** Every verified outcome is committed. Every commit is a checkpoint you can return to. Without git, a failed build can destroy working code. With git, it cannot. Trellis handles the commits — you just verify and confirm.

- **The scaffold is a starting structure, not a constraint.** `trellis init` creates the directories and files your project needs. It reflects the methodology's structure — outcomes, contracts, phases. As your project grows, the structure grows with it.

## Red Flags

- Skipping git. Without version control, there is no safety net. A failed build overwrites working code with no way back. Trellis commits after every verified outcome specifically to prevent this. Do not bypass it.

- Attempting to manage project state manually. If you are keeping notes in a separate document, copying context into prompts, or re-explaining your project at the start of each session, you are doing odd-flow's job by hand. Let the tool manage state.

- Modifying safety hooks without understanding them. The six hooks exist for specific reasons. If a hook blocks something, it is usually catching a real problem. Understand why before overriding.

- Starting a build before init. If you begin building without running init, you have no hooks, no odd-flow memory, no project scaffold. You are building without a safety net or a state manager.

## What This Means for You

If you have not already, run `npx trellis-cli init` in your project directory. Confirm that git is initialised, the skill is loaded, and odd-flow is connected. Then move to your Phase A outcomes.

The setup is a one-time step. Everything after this is the repeating rhythm of build, verify, confirm — which Chapter 10 covers next.

Next: Chapter 10 introduces the Build Protocol — the session rhythm that carries you through every outcome.
