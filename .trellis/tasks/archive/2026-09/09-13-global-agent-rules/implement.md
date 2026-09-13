# 实施计划：全局约束统一注入

> 顺序执行；每组结束都跑一次「验证」再进下一组。禁止跳步合并提交。

## 前置

- 基线检查必须全绿再动手：`cargo test --manifest-path src-tauri/Cargo.toml`、`pnpm test:run`、`pnpm typecheck`。
- 工作树里已有其他会话的改动（`src-tauri/src/commands/mcp.rs`、`src/pages/McpPage.tsx` 等）。本任务只改下面列出的文件；提交时只 `git add` 本任务文件，不要 `git add -A`。

## 步骤 1：数据库与同步（R7.1、R7.2）

1. `src-tauri/src/db/migrate.rs`
   - `V1_SCHEMA` 增加 `agent_rules` 建表语句。
   - 新增 `ensure_agent_rules_schema(conn)`，并入 `ensure_incremental_schema()`。
   - 追加两个回归测试：`v1_database_reaches_current_schema_with_agent_rules`（user_version=1 存量库）、`legacy_database_keeps_newapi_columns_and_gains_agent_rules`（legacy 分支），断言 `table_exists(&conn, "agent_rules")`。
2. `src-tauri/src/db/mod.rs`：`SCHEMA_VERSION` 2 → 3。
3. `src-tauri/src/sync.rs`：`FINGERPRINT_TABLES` 加 `"agent_rules"`（8 → 9 项）；照抄 `sync.rs:650-670` 增补一个「改 agent_rules 行 → 指纹变化」的测试。

**验证**：`cargo test --manifest-path src-tauri/Cargo.toml db::` 与 `cargo test --manifest-path src-tauri/Cargo.toml sync::`。
**回滚点**：本组可独立回退，不影响其余代码。

## 步骤 2：repo 层（R1b）

`src-tauri/src/repo/rules.rs`（新建，在 `repo/mod.rs` 注册 `pub mod rules;`）

- `get(conn) -> AppResult<AgentRules>`：无行返回 `{ body: "", targets: [], updated_at: 0 }`。
- `save(conn, body, targets) -> AppResult<AgentRules>`：`INSERT … ON CONFLICT(id) DO UPDATE`。
- 目标去重 + 保持 `TargetKind` 顺序稳定（claude_code, codex, pi, prime）。
- 另有 `agent_rules_applied_targets` 的读写吗？不——直接复用 `repo::sync_meta`，在命令层拼 key（与 `repo/mcp.rs:11` 同一写法）。

**验证**：`cargo test --manifest-path src-tauri/Cargo.toml repo::`；新增内存库测试（`Connection::open_in_memory()` + `db::apply_schema`）。

## 步骤 3：适配器（R2、R4、R5）

`src-tauri/src/adapters/agent_rules.rs`（新建，在 `adapters/mod.rs` 注册 `pub mod agent_rules;`）

1. 常量：`RULE_BEGIN` / `RULE_END` / `RULE_NOTICE`。
2. 纯函数：`validate_body`、`render_block`、`strip_blocks`、`upsert_block`、`should_delete`（契约见 design.md §3.2）。
3. 路径：`claude_rules_path`、`codex_rules_path`（含 `codex_override_shadow`）、`pi_rules_path`、`prime_rules_path`（含 AGENTS.md / CLAUDE.md 选择）。
4. 落盘：`apply_to_target(target, body: Option<&str>, override, backup_root) -> AppResult<AgentRulesTargetResult>`
   - `FileLock::acquire(&path)` → 读（BOM 识别、UTF-8 校验、路径是目录则报错）→ `upsert_block` → 无变更直接返回 → `backup_file`（文件存在才备份）→ `atomic_write`（写回 BOM）或 `fs::remove_file`（空白剩余）。
5. 单测（`tempfile::tempdir()`）覆盖 PRD 的 AC2–AC6、AC8：
   - 空文件 / 不存在 → 只有托管块；
   - 用户内容（含 BOM、无末尾换行、末尾多空行）→ 追加后用户内容字节不变；
   - 重复保存幂等；块内正文变更 → 只替换块；
   - 手工放两个块 → 收敛为一个；
   - 只有 BEGIN 无 END → 报错且文件字节不变；
   - 清空正文 → 块被移除、用户内容保留；文件只剩空白 → 被删除；
   - 正文含标记字面量 → `validation_failed`；
   - Pi/Prime：只有 `CLAUDE.md` → 写进它且不新建 `AGENTS.md`；两者都有 → 用 `AGENTS.md`；
   - Claude：`CLAUDE_CONFIG_DIR` 生效、设置 override 优先于环境变量（环境变量在测试里用 `std::env::set_var`，注意 Rust 2024 里它是 `unsafe`——若编译报错就改为把环境变量读取抽成参数化函数再测）。

**验证**：`cargo test --manifest-path src-tauri/Cargo.toml adapters::agent_rules`。

## 步骤 4：DTO 与命令层（R3）

1. `src-tauri/src/domain/rules.rs`（新建，`domain/mod.rs` 注册）— 四个 DTO，`#[serde(rename_all = "camelCase")]`。
2. `src-tauri/src/commands/rules.rs`（新建，`commands/mod.rs` 加 `pub mod rules;` + `pub use rules::*;`）
   - `get_agent_rules` / `save_agent_rules` / `agent_rules_target_paths`。
   - `save_agent_rules` 的并集与记录语义按 design.md §5；记录 key `agent_rules_applied_targets`。
   - 抽取纯函数 `merged_targets(requested, previously_applied)` 与 `targets_to_record(results, desired)` 并单测（照抄 `commands/mcp.rs:204-233` 的写法与用例）。
3. `src-tauri/src/lib.rs`：`generate_handler!` 里加三个命令（放在 MCP 命令之后保持可读性）。

**验证**：`cargo test --manifest-path src-tauri/Cargo.toml commands::rules`。
**检查点**：此时 `cargo clippy` 无新增警告（若仓库基线本来就有警告，不新增即可）。

## 步骤 5：前端（R6、R7.3、R8）

1. `src/types/rules.ts`、`src/stores/rulesStore.ts`。
2. `src/pages/RulesPage.tsx` — 结构见 design.md §6；`App.useApp()` 取 `message`；错误文案走既有 `errorText` 辅助（照抄 `McpPage.tsx` 的用法）。
3. `src/stores/uiStore.ts` 加 `"rules"`；`src/App.tsx` 加 KeepAlive 分支；`src/components/layout/SideNav.tsx` 加导航项（lucide `ScrollText`，14/18 尺寸约定）。
4. i18n：`zh-CN.json` / `en-US.json` 同时加 `nav.rules` 与 `rules.*`，键名对齐（不硬编码中文）。
5. `src/lib/browserMock.ts`：模块级 `agentRules` 状态 + 复位 + 三个命令 handler（`save_agent_rules` 返回逐目标结果，路径用 demo 路径）。
6. `src/pages/RulesPage.test.tsx`：渲染 → 勾选目标 → 输入正文 → 点保存 → mock 被调用且结果提示出现；再补一个「清空正文保存 → 提示已清理」的用例。

**验证**：`pnpm test:run` + `pnpm typecheck`。

## 步骤 6：全量验证与文档

1. 全量：`pnpm test:run`、`pnpm typecheck`、`cargo test --manifest-path src-tauri/Cargo.toml`（三条全绿才算完成；CI 就是这三条，见 `.github/workflows/ci.yml:20-35`）。
2. 浏览器目视：`pnpm dev` 打开 rules 页，确认文本框、目标勾选、落点路径折叠区、Codex 警告、保存结果提示、深色模式与中英文切换。
3. `AGENTS.md` 补一行：全局约束写的是用户级 `CLAUDE.md` / `AGENTS.md`，托管块标记前缀 `xiaobai-switch:`，与 MCP 的 `xiaobai_` 同属兼容红线（新表必须进 `FINGERPRINT_TABLES`）。
4. 若 `.trellis/spec/` 有前端相关条目需要同步（组件/状态约定），按 `trellis-update-spec` 更新。

## 复核门（每步结束自查）

- 有没有静默吞掉错误？失败必须返回 `ok: false` + 消息，UI 必须显示。
- 有没有在「无需变更」时仍然写盘 / 备份？（会污染备份目录并触发不必要的同步指纹变化）
- 有没有改动 `resolve_claude_home` 或其他目标的既有解析函数？（本任务只新增 `claude_rules_path`）
- 有没有碰用户文件里块外的内容？（唯一允许的变化是尾部空行归一，见 design.md §3.3）

## 命令与路径速查

- Rust 测试：`cargo test --manifest-path src-tauri/Cargo.toml`
- 前端测试：`pnpm test:run`；类型：`pnpm typecheck`
- 迁移与同步：`src-tauri/src/db/migrate.rs:203-238`、`src-tauri/src/sync.rs:41-50`
- 参照实现：`src-tauri/src/adapters/mcp.rs`（托管前缀 + 合并保留）、`src-tauri/src/commands/mcp.rs:204-318`（并集清理）
