# 全局约束：Agent 指令统一注入

## Goal

在 XiaoBaiSwitch Plus 中新增「全局约束」：用户在应用内以 Markdown 维护**用户级（全局）**约束，勾选目标后由应用自动写入各 CLI 自己的全局指令文件，取消勾选 / 禁用 / 删除时清理已写入的托管内容。用户自己写在那些文件里的内容必须原样保留。

对用户的价值：一次填写，四个 CLI 同时生效，不必再去记 `~/.claude/CLAUDE.md`、`~/.codex/AGENTS.md`、`~/.pi/agent/AGENTS.md`、`~/.prime/agent/AGENTS.md` 这一堆不同位置。

## Background（已调研确认的事实）

### 各 CLI 的全局指令文件落点

| 目标 | 文件 | 说明 | 来源 |
|------|------|------|------|
| Claude Code | `${CLAUDE_CONFIG_DIR:-~/.claude}/CLAUDE.md` | 官方用户级记忆文件；单文件上限 4 MiB，建议 < 200 行；支持 `@path` 导入 | code.claude.com/docs/en/memory、/claude-directory |
| Codex | `${CODEX_HOME:-~/.codex}/AGENTS.md` | 全局作用域；若同目录存在 `AGENTS.override.md`，该文件会**整体遮蔽** `AGENTS.md`；旧的 `instructions.md` 已不再读取；文档提到合并 32 KiB 上限 | developers.openai.com/codex/guides/agents-md |
| Pi | `${PI_CODING_AGENT_DIR:-~/.pi/agent}/AGENTS.md` | 全局上下文文件；同目录 `CLAUDE.md` 也被接受，但同一目录内 `AGENTS.md` 优先（`AGENTS.override.md` > `AGENTS.md` > `CLAUDE.md`） | github.com/earendil-works/pi（packages/coding-agent/docs/usage.md） |
| Prime | `${PRIME_AGENT_CODING_AGENT_DIR:-~/.prime/agent}/AGENTS.md` | Prime 是 pi-mono 的 fork，布局与 Pi 相同；`AGENTS.override.md` 在 Prime 文档中未出现，按不支持处理 | github.com/PrimeIntellect-ai/prime-agent（docs/usage.md、docs/sdk.md） |

四个 CLI 都只接受纯 Markdown，且都在**启动时**读取（Pi/Prime 支持 `/reload`，Claude Code 需重启）。

### 仓库现状（证据）

- 用户级 override 设置已存在：`AppSettings.claude_home_override / codex_home_override / pi_agent_dir_override / prime_agent_dir_override`（`src-tauri/src/domain/mod.rs:536-573`），解析入口 `resolve_claude_home / resolve_codex_home / resolve_pi_agent_dir / resolve_prime_agent_dir`（`src-tauri/src/paths.rs:470-521`）。
- Claude 的配置目录解析有一个**既有先例**：`claude_mcp_json_path()`（`src-tauri/src/paths.rs:479-494`）按 `override → CLAUDE_CONFIG_DIR → ~/.claude` 取值；而 `resolve_claude_home()` 目前只看 override，忽略 `CLAUDE_CONFIG_DIR`（`src-tauri/src/paths.rs:470-476`）。
- MCP 统一管控是本功能的直接参照：托管前缀 `xiaobai_`（`src-tauri/src/adapters/mcp.rs:15`）、"目标 = 本次请求 ∪ 上次应用目标" 的清理并集（`src-tauri/src/commands/mcp.rs:204-212`）、`sync_meta` 记录已应用目标（`src-tauri/src/repo/mcp.rs:11,209-222`）、备份根 `backups_dir()/mcp`（`src-tauri/src/commands/mcp.rs:258`）。
- 文件写入设施：`atomic_write` / `FileLock` / `backup_file`（`src-tauri/src/adapters/atomic.rs:9,82,120`）。
- 数据库：`SCHEMA_VERSION = 2`（`src-tauri/src/db/mod.rs:9`）；`ensure_incremental_schema()` 已被所有「库已存在」分支调用（`src-tauri/src/db/migrate.rs:203-207,215-238`），新表必须同时进 `V1_SCHEMA` 与该函数。
- 同步指纹表白名单 `FINGERPRINT_TABLES`（`src-tauri/src/sync.rs:41-50`）当前 8 张表；漏加新表会导致数据变更永不触发同步。
- 代码库里目前**完全没有**任何 `CLAUDE.md` / `AGENTS.md` / memory 处理逻辑（全仓检索无命中）。

## Requirements

- **R1 数据模型：单条全局文本（OQ1 已由用户确认）**
  - R1.1 全部约束就是**一段** Markdown 正文 + **一组**目标勾选（Claude Code / Codex / Pi / Prime），没有多条条目、没有条目级启用开关、没有排序。
  - R1.2 单行表 `agent_rules`（`id = 1`）存 `body` / `targets_json` / `updated_at`，与 `settings` 表同构。不预插行，无行时读作「空正文 + 空目标」。
  - R1.3 空正文或空目标 = 不生效（走清理路径），不视为错误。
- **R2 写入策略：托管块，绝不吞用户内容**
  - R2.1 每个目标文件内只维护**一个**托管块，块外用成对标记界定，例如 `<!-- xiaobai-switch:begin global-rules -->` … `<!-- xiaobai-switch:end global-rules -->`；块外一切内容原样保留。
  - R2.2 文件不存在则创建；文件已存在则在末尾追加托管块（保证与既有内容之间有且仅有一个空行分隔）。
  - R2.3 该目标不再需要约束时，移除托管块；若移除后文件只剩空白（即文件内容全部是本应用写的），**删除该文件**——否则 Pi/Prime 会把我们留下的空 `AGENTS.md` 当作优先文件，反而遮蔽用户自己的 `CLAUDE.md`。清理会顺带把尾部多余空行归一为一个换行，这是唯一允许的字节变化。
  - R2.4 用户正文中若出现标记字面量，保存时拒绝并给出明确报错（否则无法保证幂等）。
  - R2.5 保留原文件的 UTF-8 BOM（Windows 常见）。
- **R3 目标选择与自动生效**
  - R3.1 保存条目后立即写入其目标集合（与 MCP 的 `save_mcp_server` 行为一致），无需再点一次应用。
  - R3.2 写入目标 = 本次目标集合 ∪ 上次应用过的目标集合（`sync_meta`），因此取消勾选 / 禁用 / 删除 / 改绑目标都能清理掉旧文件里的托管块。
  - R3.3 提供「应用全部」手动按钮与四目标落点路径展示（对齐 `mcp_target_paths`）。
- **R4 安全与失败姿态**
  - R4.1 每次写前 `backup_file` 到 `~/.xiaobai-switch/backups/agent-rules/`，再用 `atomic_write` 原子替换，读-改-写期间持 `<文件名>.lock`。
  - R4.2 以下情况必须**报错并原样保留文件**：路径是目录、文件不是合法 UTF-8、只有开始标记没有结束标记、文件读取失败。
  - R4.3 同一文件里出现多个托管块（异常情况）时收敛为一个，不报错。
- **R5 兼容红线**
  - R5.1 Claude 记忆文件按 `override → CLAUDE_CONFIG_DIR → ~/.claude` 解析（与 `claude_mcp_json_path` 同一套优先级）。
  - R5.2 Pi/Prime 的目标文件选择：`AGENTS.md` 存在 → 用它；否则若 `CLAUDE.md` 存在 → append 到它（不新建 `AGENTS.md`，避免改变 CLI 的选择优先级）；两者都不存在 → 新建 `AGENTS.md`。
  - R5.3 Codex 只写 `AGENTS.md`；检测到 `AGENTS.override.md` 时在 UI 明确警告该文件会遮蔽我们的内容，不自动改写 override 文件。
  - R5.4 不新增明文密钥；约束正文按普通业务数据存库，随既有备份 / WebDAV 同步。
- **R6 UI**
  - R6.1 主侧栏新增入口（48px 图标按钮 + antd Tooltip，`placement="right"`），页面沿用现有壳体。
  - R6.2 编辑界面用 antd `Form` + `Input.TextArea`（等宽字体、可纵向拉伸），目标用 `Checkbox.Group`。
  - R6.3 保存后底部提示写入结果（逐目标成功/失败消息），失败不静默。
  - R6.4 页面展示四个目标的实际落点路径与存在状态，含 R5.3 的警告。
- **R7 数据与同步**
  - R7.1 新表进 `V1_SCHEMA` 且 `SCHEMA_VERSION` 升到 3，并在 `ensure_incremental_schema()` 里补建（存量库必须拿到，不得静默失败）。
  - R7.2 新表加入 `FINGERPRINT_TABLES`，否则约束变更不会触发同步。
  - R7.3 `browserMock.ts` 补齐新命令，保证非 Tauri 开发与 vitest 可跑。
- **R8 国际化**：所有用户可见文案进 `zh-CN.json` + `en-US.json`（新增 `rules` 命名空间与 `nav.rules`）。
- **R9 文档口径**：UI 需说明写入的是明文 Markdown，任何能读到该文件的程序都能看到内容（与 MCP 的明文提示同一模块）。

## Acceptance Criteria

- [ ] AC1 新建一条约束并勾选四个目标后，四个文件里各出现且仅出现一个托管块，块内容与该条目正文一致。
- [ ] AC2 目标文件里预先存在用户自己的内容（含 Markdown 标题、正文、BOM、无末尾换行）时，应用后用户内容字节级不变（BOM 保留），托管块追加在末尾。
- [ ] AC3 取消某个目标并保存后，该目标文件里的托管块被移除，块外内容不变；文件若因此只剩空白则被删除。
- [ ] AC4 禁用或删除全部条目后，四个文件都回到「无托管块」状态，且用户原有文件内容完好。
- [ ] AC5 目标文件只有开始标记时保存失败并返回明确错误，文件字节不变。
- [ ] AC6 Pi/Prime 场景：目录里只有用户的 `CLAUDE.md` 时，托管块写进 `CLAUDE.md` 且不新建 `AGENTS.md`。
- [ ] AC7 Codex 场景：目录里有 `AGENTS.override.md` 时，UI 落点区显示遮蔽警告。
- [ ] AC8 设置 `CLAUDE_CONFIG_DIR` 时，Claude 的托管块写进该目录下的 `CLAUDE.md`（override 设置优先于环境变量）。
- [ ] AC9 存量老库（user_version=2）升级后功能可用：`agent_rules` 表存在（`ensure_incremental_schema` 生效）。
- [ ] AC10 约束变更后 `compute_logical_fingerprint` 结果变化（新表进入指纹）。
- [ ] AC11 `pnpm test:run`、`pnpm typecheck`、`cargo test` 全绿；新增 Rust 单测覆盖 R2/R3/R5 的分支，前端至少覆盖页面保存与目标勾选。
- [ ] AC12 中英文文案齐全，无硬编码中文。

## Out of Scope

- 不管理项目级 `AGENTS.md` / `CLAUDE.md`（只做用户级全局文件）。
- 不做 Markdown 渲染预览与富文本编辑（v1 仅纯文本编辑）。
- 不接管 `~/.claude/rules/**/*.md`、`SYSTEM.md` / `APPEND_SYSTEM.md` 这类替换系统提示的机制。
- 不自动改写 Codex 的 `AGENTS.override.md`（只警告）。
- 不修复 `resolve_claude_home()` 对 `CLAUDE_CONFIG_DIR` 的既有盲区（影响 `settings.json` 与 restore_official，属独立风险，另开任务）。
- 不做技能 / 提示词级别的下发，不做条目排序拖拽。

## Notes

- 规划阶段已确认（用户 2026-09-13）：约束内容组织方式 = **单个 Markdown 文本框 + 一组目标勾选**（即 R1.1）。
- 本 PRD 是规划阶段产物；实现前需要 `design.md` + `implement.md`，评审通过后 `task.py start`。
