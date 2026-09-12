# Build Entry Protocol

When `*build` is called, execute these checks in order before beginning:

1. Check that `planApproved` is true in `.odd/state.json`. If not, explain that the plan must be approved before building, and offer `*plan` to complete it.
2. Check that `techStackDecided` is true. If not, explain that the technical architecture decision must be made first, and route to `*phase-plan` to complete it with Rachel.
3. Check that `designApproachDecided` is true. If not, explain that the design approach must be decided before building, and route to `*design` to complete it with Rachel.
4. Check that `servicesConfigured` is true. If not, run the Project Setup Protocol below before proceeding.
5. **Model check (advisory only).** If running on Opus, display: "**Model advisory:** The build phase runs well on Sonnet — faster and cheaper. Switch with `/model` if you prefer." Do not block or repeat if already shown this session.
6. **Phase Brief check — HARD GATE.** Read `sessionBriefCount` from `.odd/state.json` (default 0 if not set). Check whether `docs/session-brief-[sessionBriefCount].md` exists.

If the brief does NOT exist:
- Run `*export` now to generate it
- Wait for the brief to be fully written to disk
- Present it to the domain expert for review
- Wait for explicit confirmation
- Only after confirmation: set `briefConfirmed: true` in `.odd/state.json`
- Do NOT proceed until `briefConfirmed` is true
- Do NOT spawn build agents, write code, create files, or modify the codebase in any way

If the brief exists but `briefConfirmed` is not true in state:
- Present it to the domain expert
- Wait for confirmation
- Set `briefConfirmed: true` in `.odd/state.json`

7. **Initialise the odd-flow swarm — mandatory first action.**

The swarm must be initialised before loading source files or planning build work. Execute the Swarm Initialisation section below in full, then proceed.

8. Load `docs/build/build-protocol.md` and `docs/build/code-excellence.md` from this ODD skill tree.
9. Confirm to the user which phase is being worked on and which outcomes are in scope.
10. Begin the Build Protocol for the current phase.

## Project Setup Protocol

Run when `servicesConfigured` is false.

1. **Scaffold.** If `package.json` exists, skip to step 2. If not: scaffold into a sibling directory, then rsync across excluding `.git`, `docs/`, and `node_modules/`. Fix `package.json name` after rsync.
2. **Install deps.** Read `testingFramework` from `.odd/state.json` (default `Vitest`). Install the chosen testing stack plus production deps.
3. **Scaffold test harness.** Create the correct config for the chosen testing stack. For Vitest, create `vitest.config.ts`, `tests/setup.ts`, `tests/setup.test.ts`, and the `test` / `test:watch` scripts.
4. **Generate `.env.local`.** Write placeholders with comments explaining where each real value comes from.
5. **Wait.** Display the credential list and wait for the user to confirm they have filled everything in.
6. **Verify.** Kill port 3000 if needed, run `npm run dev`, and translate any setup failures into plain language. Repeat until the server starts cleanly.
7. **Mark done.** Set `servicesConfigured: true` in `.odd/state.json` and confirm the development server is running.

## Swarm Initialisation

When `*swarm` or `*build` is called with an approved plan, execute this sequence:

1. Store project state with `mcp__odd-flow__memory_store` key `trellis-project-state`, namespace `trellis-project`.
2. Store shared contracts with `mcp__odd-flow__memory_store` key `trellis-contract-map`, namespace `trellis-project`.
3. Create the phase task with `mcp__odd-flow__task_create`.
4. Spawn the coordinator agent with `mcp__odd-flow__agent_spawn`.
5. Spawn the backend agent with `mcp__odd-flow__agent_spawn`.
6. Spawn the UI agent with `mcp__odd-flow__agent_spawn`.
7. Spawn the QA agent with `mcp__odd-flow__agent_spawn`.
8. Finalise with `mcp__odd-flow__coordination_sync`.

When `coordination_sync` succeeds in build phase, the hook activates:
- `.odd/.odd-flow-swarm-active`
- `.odd/.odd-flow-agents-ready`
- `.odd/.odd-flow-phase-synced`
- `.odd/.odd-flow-agent-token`

Do not manually touch these markers.

**Important:** the odd-flow `agent_spawn` calls above are coordination metadata only. They register roles, contracts, and phase context in odd-flow, but they do not execute code changes themselves.

After swarm initialisation completes, dispatch the actual build work with your runtime's executor agent tool:
- In Claude/Codex-style runtimes, use the `Agent` tool for implementation work.
- Treat odd-flow as the coordination layer and the `Agent` tool as the code-writing executor.
- Do not stop after `coordination_sync`. The next action is to start the Build Execution Loop in `docs/build/build-protocol.md`.
