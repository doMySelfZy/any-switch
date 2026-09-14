# 实现计划

## 进度

- [x] **1. 后端读取层**（`src-tauri/src/adapters/mcp_scan.rs`）
  - 四个客户端只读解析器；Codex 用 `toml_edit` 处理 `[mcp_servers.<name>.env]` 嵌套子表。
  - `ScannedMcp` 只含元数据与**密钥键名**，`env`/`headers` 的值一律不返回。
  - `load_entry_for_import` 是独立入口，读完整内容（含密钥）供加密入库，与扫描严格分开。
  - 11 个单测，含安全断言 `scan_never_returns_secret_values`（已用变异测试确认有效）。
- [x] **2. 命令层**（`commands/mcp.rs`）
  - `scan_existing_mcp`：扫描 + 与库内记录比对标注 `alreadyImported`。
  - `import_scanned_mcp`：按 `(target, key)` 定位符纳管；密钥由后端读盘取得，**不经过前端**；
    只回元数据（`save` 的返回值含明文，不回传）。
- [x] **3. 接管逻辑**（`adapters/mcp.rs` 四个 `apply_to_*`）
  - 同名未托管条目若与库内记录等价（忽略 `type`/`transport`）→ 删除后写托管条目，净结果一条；
  - 不一致 → 报错跳过该目标，文件原样保留，绝不覆盖用户改动；
  - 无同名条目 → 行为不变。Codex 侧用 `codex_entry_to_json` / `codex_record_to_json`
    把 `http_headers` 与 `env` 子表规约成同一形态，两个方向共用。
  - 7 个接管用例；两个变异测试确认：去掉接管删除后客户端里会同时出现
    `demo` 与 `xiaobai_demo`（即重复加载），用例会失败。
- [x] **4. 前端**（`McpPage` + store + types + i18n + browser mock）
  - 打开页面即自动扫描一次；列出条目并标明来源客户端、启动方式、需要的密钥**键名**、
    以及「本工具管理 / 已纳管 / 可纳管」状态。
  - 支持单条纳管与「全部纳管」；纳管后重新扫描刷新标记。
  - 单客户端读取失败只提示该客户端，不影响其它客户端。
  - 5 个用例覆盖：打开即扫描、只列键名不列值、托管条目不给纳管入口、
    单条纳管后标记刷新、全部纳管后批量入口消失。
- [ ] **5. 真机验证**：打包安装后确认界面能列出本机 Codex 的 5 个条目
      （`ssh`/`context7`/`exa`/`tavily`/`sequential-thinking`，其中三个带 `.env` 子表）。

## 验证结果
- `cargo test`：512 passed / 0 failed / 1 ignored
- `pnpm typecheck`：通过
- `pnpm test:run`：352 passed（2 个 updater 脚本用例为本机既有 shebang 失败，与本任务无关）

## 提交
- `ae2ac6b` 命令层与安全接管逻辑
- `5045dd7` 界面展示并纳管已有 MCP

## 说明
本任务开始阶段主工作树被并行会话的 floating-window 改动阻塞编译，第 1 步曾在
`git worktree` 的 HEAD 干净副本中验证；该阻塞已解除，后续步骤均在主树完成。
