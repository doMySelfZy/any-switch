# Bootstrap Task: Fill Project Development Guidelines

**You (the AI) are running this task. The developer does not read this file.**

The developer just ran `trellis init` on this project for the first time.
`.trellis/` now exists with empty spec scaffolding, and this bootstrap task
exists under `.trellis/tasks/`. When they want to work on it, they should start
this task from a session that provides Trellis session identity.

**Your job**: help them populate `.trellis/spec/` with the team's real
coding conventions. Every future AI session — this project's
`trellis-implement` and `trellis-check` sub-agents — auto-loads spec files
listed in per-task jsonl manifests. Empty spec = sub-agents write generic
code. Real spec = sub-agents match the team's actual patterns.

Don't dump instructions. Open with a short greeting, figure out if the repo
has any existing convention docs (CLAUDE.md, .cursorrules, etc.), and drive
the rest conversationally.

---

## Status (update the checkboxes as you complete each item)

- [x] Fill frontend guidelines
- [x] Add code examples

### 完成说明（2026-09-14）

按 PRD 指引的 Step 1（优先从既有约定文件导入）执行：本仓库的权威约定文档是根目录
`AGENTS.md`（产品规则、兼容红线、UI Shell、Ant Design、i18n、后端约定、测试要求），
已按 spec 的分类结构提炼成六个文件，并补充了 Step 2 的「扫描真实代码」得到的内容
（真实文件路径、真实反例）。

写入的六个文件：

| 文件 | 内容要点 |
|------|----------|
| `index.md` | 声明 `AGENTS.md` 是权威源、本目录是操作性索引；三条最常踩的坑 |
| `directory-structure.md` | 前后端目录表；新增命令要动的三处；新增业务表必须进 `FINGERPRINT_TABLES` |
| `component-guidelines.md` | Modal 属性表；`App.useApp()`；图标按钮 `aria-label`；异步 `onOk` 必须吞异常 |
| `hook-guidelines.md` | 清理函数、防「卸载后才拿到句柄」、`useCallback` 依赖、纯函数放组件外（TDZ）、拖动不在 `mousemove` 里写库、跨窗口状态 |
| `state-management.md` | store 边界、写后回读、前后端字段名对齐（附 `autoRefreshSeconds` 真实事故）、browserMock 契约 |
| `type-safety.md` | `invoke<T>` 契约、少用 `as`、联合类型穷尽映射、Rust↔TS 类型对照表 |
| `quality-guidelines.md` | i18n 双花括号与中英成对更新、测试命令、**断言质量与变异验证**、本机已知的 2 个既有失败 |

全部内容基于真实代码与真实事故（不是理想化描述）：例如 Modal 异步 `onOk` 的两个
反例、刷新间隔字段名不一致导致设置静默失效、悬浮窗每帧写库、`useMemo` 踩 TDZ。

**未做**：`.trellis/spec/guides/` 是脚手架预置的通用思考指南，与本项目不冲突，
按 PRD 说明「仅在不适用时定制」，故保持原样。

---

## Spec files to populate


### Frontend guidelines

| File | What to document |
|------|------------------|
| `.trellis/spec/frontend/directory-structure.md` | Component/page/hook organization |
| `.trellis/spec/frontend/component-guidelines.md` | Component patterns, props conventions |
| `.trellis/spec/frontend/hook-guidelines.md` | Custom hook naming, patterns |
| `.trellis/spec/frontend/state-management.md` | State library, patterns, what goes where |
| `.trellis/spec/frontend/type-safety.md` | TypeScript conventions, type organization |
| `.trellis/spec/frontend/quality-guidelines.md` | Linting, testing, accessibility |


### Thinking guides (already populated)

`.trellis/spec/guides/` contains general thinking guides pre-filled with
best practices. Customize only if something clearly doesn't fit this project.

---

## How to fill the spec

### Step 1: Import from existing convention files first (preferred)

Search the repo for existing convention docs. If any exist, read them and
extract the relevant rules into the matching `.trellis/spec/` files —
usually much faster than documenting from scratch.

| File / Directory | Tool |
|------|------|
| `CLAUDE.md` / `CLAUDE.local.md` | Claude Code |
| `AGENTS.md` | Codex / Claude Code / agent-compatible tools |
| `.cursorrules` | Cursor |
| `.cursor/rules/*.mdc` | Cursor (rules directory) |
| `.windsurfrules` | Windsurf |
| `.clinerules` | Cline |
| `.roomodes` | Roo Code |
| `.github/copilot-instructions.md` | GitHub Copilot |
| `.vscode/settings.json` → `github.copilot.chat.codeGeneration.instructions` | VS Code Copilot |
| `CONVENTIONS.md` / `.aider.conf.yml` | aider |
| `CONTRIBUTING.md` | General project conventions |
| `.editorconfig` | Editor formatting rules |

### Step 2: Analyze the codebase for anything not covered by existing docs

Scan real code to discover patterns. Before writing each spec file:
- Find 2-3 real examples of each pattern in the codebase.
- Reference real file paths (not hypothetical ones).
- Document anti-patterns the team clearly avoids.

### Step 3: Document reality, not ideals

**Critical**: write what the code *actually does*, not what it should do.
Sub-agents match the spec, so aspirational patterns that don't exist in the
codebase will cause sub-agents to write code that looks out of place.

If the team has known tech debt, document the current state — improvement
is a separate conversation, not a bootstrap concern.

---

## Quick explainer of the runtime (share when they ask "why do we need spec at all")

- Every AI coding task spawns two sub-agents: `trellis-implement` (writes
  code) and `trellis-check` (verifies quality).
- Each task has `implement.jsonl` / `check.jsonl` manifests listing which
  spec files to load.
- The platform hook auto-injects those spec files + the task's `prd.md`
  into every sub-agent prompt, so the sub-agent codes/reviews per team
  conventions without anyone pasting them manually.
- Source of truth: `.trellis/spec/`. That's why filling it well now pays
  off forever.

---

## Completion

When the developer confirms the checklist items above are done with real
examples (not placeholders), guide them to run:

```bash
python ./.trellis/scripts/task.py finish
python ./.trellis/scripts/task.py archive 00-bootstrap-guidelines
```

After archive, every new developer who joins this project will get a
`00-join-<slug>` onboarding task instead of this bootstrap task.

---

## Suggested opening line

"Welcome to Trellis! Your init just set me up to help you fill the project
spec — a one-time setup so every future AI session follows the team's
conventions instead of writing generic code. Before we start, do you have
any existing convention docs (CLAUDE.md, .cursorrules, CONTRIBUTING.md,
etc.) I can pull from, or should I scan the codebase from scratch?"
