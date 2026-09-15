# 执行计划：WebDAV 同步引擎完整性修复

## 前置

- 工作目录：仓库根
- Rust：`cd src-tauri && cargo test`
- 前端：`pnpm typecheck`、`pnpm test:run`
- 参考：本任务 `prd.md`（需求与验收）、`design.md`（技术设计）

## 实施步骤

### S1 指纹算法版本（R2）

- [ ] S1.1 `src-tauri/src/sync.rs`：新增 `pub const FINGERPRINT_ALGORITHM_VERSION: u32 = 1;`，注释说明"表清单变更必须递增"。
- [ ] S1.2 `SyncManifest` 增加 `#[serde(skip_serializing_if = "Option::is_none")] pub fingerprint_algorithm: Option<u32>`。
- [ ] S1.3 `SyncManifest::validate()` 增加：`Some(v)` 时要求 `v >= 1`；不因未知版本拒绝。
- [ ] S1.4 新增 `SyncManifest::algorithm_matches_current()`（`None => true`）。
- [ ] S1.5 `SyncAction` 增加 `Incompatible`；`decide_action` 在 `Some(remote)` 分支**首先**做算法检查。
- [ ] S1.6 `run_sync_inner` 的 `match` 增加 `Incompatible` 分支 → `Err(AppError::new("sync_algorithm_mismatch", …))`，消息含双方版本号。
- [ ] S1.7 Upload 分支构造 manifest 时填 `fingerprint_algorithm: Some(FINGERPRINT_ALGORITHM_VERSION)`。

### S2 记账时机（R1）

- [ ] S2.1 `pending_restore.rs`：`queue_pending_restore(archive, app_dir, expected: Option<DataFingerprint>)`，在 staging 目录写入 `expected-sync.json`（`DataFingerprint` 序列化）。
- [ ] S2.2 新增 `pub struct RestoreStartupOutcome { pub result: RestoreStartupResult, pub synced_fingerprint: Option<DataFingerprint> }`；`apply_pending_restore` 返回 `Option<RestoreStartupOutcome>`。
- [ ] S2.3 `apply_pending_restore_inner` 成功后读取 `expected-sync.json` 填入 `synced_fingerprint`；失败路径返回 `None`（且清理/隔离时一并移除该文件）。
- [ ] S2.4 已 commit 的补齐路径（`COMMITTED_NAME` 存在时直接 cleanup 返回）也要正确回传或明确置 `None`——按"pending 已清理则无需提交"处理，并在注释中说明。
- [ ] S2.5 `sync.rs`：`save_last_synced` 改签名为 `(conn: &Connection, fp: &DataFingerprint)`；Upload / InSync 分支改为 `state.db.with_conn(|c| save_last_synced(c, &fp))`。
- [ ] S2.6 `sync.rs` Download 分支：删除 `save_last_synced(state, &remote.fingerprint())`，改为把 `Some(remote.fingerprint())` 传给 `queue_pending_restore`。
- [ ] S2.7 `sync.rs`：新增 `pub(crate) fn save_last_synced_connection(...)` 或直接 `pub` 化 `save_last_synced`，供 `state.rs` 调用。
- [ ] S2.8 `state.rs`：`AppState::init` 改造为——先 `apply_pending_restore`，再 `Db::open`，再对 `applied` 且 `synced_fingerprint.is_some()` 的情况提交 `save_last_synced`；保留原有 `clear_sync_status` 行为。
- [ ] S2.9 `commands/webdav.rs`：`restore_local_backup` / `restore_webdav_backup` 调用点补 `None`（手动恢复不携带同步目标）。

### S3 打包时机（R3）

- [ ] S3.1 `run_sync_inner`：把 `temp_dir` 创建、`create_backup_in`、`parse_device_from_filename` 从决策前移入 `Upload` 分支。
- [ ] S3.2 Download 分支单独创建自己的临时目录（下载归档 + 解包），不依赖 S3.1 的包。
- [ ] S3.3 确认 `InSync` / `Incompatible` 路径不再创建任何 `tempdir`。
- [ ] S3.4 保留 Download 分支的 `create_local_backup(conn, "pre_sync_apply", settings.max_backup_copies)`。

### S4 默认保留数（R4）

- [ ] S4.1 `src-tauri/src/db/migrate.rs:163`：建表默认值 `DEFAULT 10` → `DEFAULT 3`。
- [ ] S4.2 `src-tauri/src/domain/mod.rs`：结构体默认值 10 → 3（约 681 行）。
- [ ] S4.3 `src/components/settings/WebDavBackupSettings.tsx:25`：`maxRemoteBackups: 10` → `3`。
- [ ] S4.4 `src/lib/browserMock.ts`：两处 10 → 3（约 221、415 行）。
- [ ] S4.5 `commands/webdav.rs` 测试夹具（约 436、464 行）与 `repo/webdav.rs` 测试夹具（约 143 行）：按测试意图决定是否同步改为 3；不改变断言语义则不强制。

### S5 i18n 错误提示（R6）

- [ ] S5.1 `WebDavBackupSettings.tsx`：`handleSyncNow` 与 `loadRemoteBackups` 的 catch 中，对 `sync_algorithm_mismatch` 映射到 `t("settings.webdav.algorithmMismatch")`。
- [ ] S5.2 i18n 资源（`zh-CN` / `en-US`）：新增 `settings.webdav.algorithmMismatch`，文案说明"对端应用版本过旧，请先升级对端后重试"。
- [ ] S5.3 确认其余错误码仍走 `error.message`（不扩大改造范围）。

### S6 测试（R5）

- [ ] S6.1 `sync.rs`：`decide_action` 新增算法不兼容用例（远端 `Some(2)` → `Incompatible`）。
- [ ] S6.2 `sync.rs`：缺字段用例（`None` + 指纹相同 → `InSync`；`None` + 只有远端变 → `Download`）。
- [ ] S6.3 `sync.rs`：`FINGERPRINT_TABLES` 长度与算法版本的固定断言（防"改表清单忘改版本号"）。
- [ ] S6.4 `sync.rs`：现有 `manifest()` 测试夹具补 `fingerprint_algorithm` 字段；`manifest_round_trips_and_validates` 覆盖新字段。
- [ ] S6.5 `sync.rs`：`rejects_tampered_manifests` 增加 `fingerprint_algorithm: Some(0)` 被拒的用例。
- [ ] S6.6 `pending_restore.rs`：`queue_pending_restore` 带 `expected` 时，成功应用后 `apply_pending_restore` 回传该指纹。
- [ ] S6.7 `pending_restore.rs`：恢复失败（注入 publish 失败）后回传 `synced_fingerprint: None`。
- [ ] S6.8 现有 `pending_restore.rs` 测试调用点补 `None` 参数与新的返回类型解构。
- [ ] S6.9 `state.rs`：如可行，补一个"恢复成功 → last_synced 被提交"的集成测试；不可行则在 `sync.rs` 用直接调用 `save_last_synced` 覆盖语义并说明。

## 验证命令

```bash
cd src-tauri && cargo test          # 后端全部
cd src-tauri && cargo clippy --all-targets  # 若仓库已启用
pnpm typecheck                       # 前端类型
pnpm test:run                        # 前端测试
```

## 风险点 / 回滚

- **高风险文件**：`src-tauri/src/sync.rs`（决策与记账）、`src-tauri/src/pending_restore.rs`（启动恢复链，失败会导致换库回滚）。
- **回滚点**：本任务为纯代码改动，`git revert` 单个提交即可；不涉及数据迁移，回退后 0.1.5 二进制可正常读写（manifest 新字段被忽略）。
- **不可回滚的动作**：无。不清理远端数据包、不改写既有 `webdav_config` 行。
- **注意**：S2 改动触及启动顺序，必须保证 `Db::open` 失败时不会误提交 `last_synced`（提交写在 `with_conn` 内，天然受其错误传播保护）。

## 提交前检查

- [ ] `cargo test` 全绿，且新增用例确实覆盖 AC1–AC5、AC9、AC10、AC11
- [ ] `pnpm typecheck`、`pnpm test:run` 全绿
- [ ] 无残留调试输出；未改动兼容红线（manifest 文件名、备份前缀、ZIP 条目名）
- [ ] 确认 `AGENTS.md` 中「数据库 schema 版本号变更时每个分支都要跑 `ensure_incremental_schema`」的约束未被破坏（本任务不改 schema 版本号）
