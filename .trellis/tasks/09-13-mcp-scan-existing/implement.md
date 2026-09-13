# 实现计划

## 进度

- [x] **1. 后端读取层**（`src-tauri/src/adapters/mcp_scan.rs`）
  - 四个客户端只读解析器；Codex 用 `toml_edit` 处理 `[mcp_servers.<name>.env]` 嵌套子表。
  - `ScannedMcp` 只含元数据与**密钥键名**，`env`/`headers` 的值一律不返回。
  - `load_entry_for_import` 是独立入口，读完整内容（含密钥）供加密入库，与扫描严格分开。
  - `entry_fingerprint` 对规范化内容算 sha256（键序无关、值敏感），供接管比对。
  - **11 个单测全部通过**（在 HEAD 的隔离 worktree 中验证，因主树被其它会话的
    floating-window 改动阻塞编译）。含安全断言 `scan_never_returns_secret_values`，
    并已用变异测试确认该断言真的能抓到泄漏。
- [ ] **2. 命令层**：`scan_existing_mcp` / `import_scanned_mcp` + 与 DB 比对标注 `alreadyImported`。
- [ ] **3. 接管逻辑**：加进四个 `apply_to_*`（同名未托管条目指纹一致则替换，不一致则跳过报错）。
- [ ] **4. 前端**：扫描结果列表（区分托管/自有/已纳管）、单个与批量纳管、i18n、browser mock。
- [ ] **5. 验证**：全量 `cargo test` / `pnpm typecheck` / `pnpm test:run`；真机扫描本机 Codex 5 个条目 → 纳管 → 应用确认不重复。

## 阻塞说明

主工作树当前**无法编译**（12 个错误全在并行会话的 `commands/floating.rs` 与
`floating_window.rs`）——那是另一个未完成的功能，与本任务无关，不应由我改动。

因此第 2 步起需要在主树恢复可编译后再继续。第 1 步已用 `git worktree` 在 HEAD 的
干净副本中完整验证，未受主树影响。

## 验证命令
- `cd src-tauri && cargo test`、`pnpm typecheck`、`pnpm test:run`
- 真机只读扫描：确认能列出本机 Codex 的 5 个条目与其中的 `.env` 子表。
