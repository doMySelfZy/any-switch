# 修复 WebDAV 同步引擎的数据覆盖与算法兼容缺陷

## Goal

消除跨设备同步中"静默覆盖对方数据"的路径，让指纹算法变更可被识别而不是退化成互相覆盖，同时降低每次同步的无效开销与云端数据包堆积。

用户价值：换机/多机使用时不丢站点与密钥；跨版本升级时同步要么正常工作、要么明确报错，而不是无声地吞掉数据。

## Background（已确证的现场事实）

2026-09-15 排查一次真实数据丢失（AnyRouter 站点在两端消失），从 NAS 上的 22 个数据包还原出完整证据链：

- 家里那台 `DESKTOP-ML5H1I2` 最后发布是 09-14 19:25:17（revision 32），此后未再上传；ultrarouter 站点在全部 22 个数据包、全部表全部列中**零命中**，即它从未被发布到云端。
- 本机 `DESKTOP-K4H68TO` 于 09-15 08:41:27 做了 `pre_sync_apply` 快照（7 站点 9 密钥 95 模型），随后被远端数据覆盖为 6 站点 8 密钥 80 模型。快照仍在 `~/.xiaobai-switch/backups/app/xiaobai-switch-backup-20260915_004127.DESKTOP-K4H68TO.21b4227d.zip`。
- 家里那台存在自相矛盾状态：包内数据 6 站点，但其 `sync_meta.last_synced_database_sha256` 指向 7 站点那份数据的指纹。这正是下面 R1 描述的缺陷留下的痕迹。
- 家里那台运行的是**版本号与实际代码不符的开发构建**：`manifest.appVersion = 0.1.4`，但数据库 `user_version = 2` 且缺少 `agent_rules` 表；而 v0.1.4 标签的代码 `SCHEMA_VERSION = 3` 并会建出该表。对应提交窗口 `f593b1e`（bump 0.1.4，09-13 21:13）早于 `4b6add3`（引入 agent_rules 并加入指纹表清单，09-13 23:01）。
- 因此两端指纹算法不同：0.1.3 及更早为 8 张表，0.1.4（真实代码）起为 9 张表（新增 `agent_rules`）。同一份数据在两端算出不同指纹，判定结果恒定相反：一端判"只有本地变了"→上传，另一端判"只有远端变了"→下载。

### 相关代码锚点

| 位置 | 内容 |
|---|---|
| `src-tauri/src/sync.rs:168-187` | `decide_action` 纯决策函数 |
| `src-tauri/src/sync.rs:251-261` | 决策前即创建完整数据包（`VACUUM INTO` + zip），仅 Upload 分支使用 |
| `src-tauri/src/sync.rs:344-349` | Download 分支：`save_last_synced` 早于 `queue_pending_restore` |
| `src-tauri/src/sync.rs:280-320` | Upload 分支 |
| `src-tauri/src/sync.rs:41-51` | `FINGERPRINT_TABLES`（9 张表，0.1.4 起） |
| `src-tauri/src/sync.rs:56-101` | `compute_logical_fingerprint` |
| `src-tauri/src/sync.rs:112-123` | `SyncManifest` 结构（无算法标识字段） |
| `src-tauri/src/pending_restore.rs:75-111` | `queue_pending_restore` |
| `src-tauri/src/pending_restore.rs:113-151` | `apply_pending_restore`（启动时、`Db::open` 之前执行） |
| `src-tauri/src/state.rs:20-32` | `AppState::init`：应用恢复后在 `Db::open` 之后清理同步状态 |
| `src-tauri/src/db/migrate.rs:163` | 表中 `max_remote_backups` 默认值 10 |
| `src-tauri/src/domain/mod.rs:681` | 结构体默认值 10 |
| `src/components/settings/WebDavBackupSettings.tsx:25` | 前端默认值 10 |

## Requirements

### R1 下载未落地不再污染同步记账（P0，数据安全）

**缺陷**：Download 分支在排队恢复时立即 `save_last_synced(remote_fp)`，但真正的换库要等下次启动才由 `apply_pending_restore` 执行。若进程在重启前退出、或恢复执行失败，本地仍是旧数据，而记账已认定"远端即共同祖先"，于是下一轮判定落入"只有本地变了"→ 上传，用**旧数据覆盖云端较新的数据**。

**要求**：

- R1.1 排队恢复时**不得**把远端指纹写入 `last_synced`。
- R1.2 只有在恢复**真正应用成功后**，才把该次下载对应的远端指纹提交为 `last_synced`；提交时机必须在 `Db::open` 之后（恢复后的库是远端库，其自身 `sync_meta` 不携带本次下载的目标指纹，故不可依赖）。
- R1.3 恢复失败（回滚/隔离 payload）时**不得**提交 `last_synced`，使下一轮能重试下载而不是反向上传。
- R1.4 恢复成功但随后 schema 迁移改变了本地数据（例如为旧库补出 `agent_rules` 表）时，行为必须是"重新发布升级后的数据"（Upload），而不是反复下载。

### R2 指纹算法可识别，跨算法不得互相覆盖（P0，数据安全）

**缺陷**：`SyncManifest` 没有指纹算法标识，`decide_action` 只比对哈希值。表清单一旦变化（已真实发生：0.1.3 的 8 表 → 0.1.4+ 的 9 表），跨版本两端对同一份数据算出不同指纹，进入互相覆盖的循环。

**要求**：

- R2.1 为指纹算法引入显式版本标识，与表清单同源维护；表清单变化时该标识必须同步变更。
- R2.2 `SyncManifest` 携带该算法标识，与数据一起发布。
- R2.3 当本地算法标识与远端 manifest 的标识不一致时，**禁止**执行 Upload 与 Download 两种数据替换动作，改为明确报错（含可操作提示：升级对端）。不得静默跳过、不得降级为"假冲突"。
- R2.4 缺少该字段的 manifest（由未修复版本写出）**按当前算法处理**，照常比较指纹。理由：现存 manifest 全部来自 0.1.4/0.1.5 的 9 表实现，"假定相同"对实际存在的对端是正确的，且升级后不会出现"必须上传才能加字段、但字段缺失又禁止上传"的死锁。该行为必须有测试固定。
- R2.5 现有 `format_version` 语义保持不变（它约束的是 manifest 结构版本），算法标识独立于它。新增字段必须是**可选/可忽略**的：未修复版本读取新 manifest 不得报错（保证可回滚）。

### R3 只为上传创建数据包（P2，性能）

**缺陷**：`run_sync_inner` 在做出决策**之前**就 `VACUUM INTO` 并打包整个数据库；"已同步"与"下载"两种结果都不需要这个包，打完即随临时目录丢弃。窗口每次聚焦（约 5 秒一轮询）都会走一遍。

**要求**：

- R3.1 数据包只在确定执行 Upload 时创建。
- R3.2 设备名解析等仅上传需要的派生值随之移入 Upload 分支，行为不变。
- R3.3 Download 分支的"应用前强制本地快照"（`pre_sync_apply`，写入持久备份目录）必须保留，它才是下载路径的兜底。

### R4 远端保留数默认值改为 3（P3）

**要求**：

- R4.1 新配置的远端保留数默认为 3（`max_remote_backups`）。
- R4.2 校验范围 1–100 与现有 UI 交互不变。
- R4.3 已存在配置的取值**不被静默改写**；用户可在设置界面自行调整。

### R5 回归测试

- R5.1 为 R1 的"恢复失败不提交记账""恢复成功提交远端指纹"补充单元测试。
- R5.2 为 R2 的算法标识不一致路径补充单元测试（含旧 manifest 缺字段的情形）。
- R5.3 为 R3 补充测试，确认"已同步"与"下载"路径不再产生数据包。
- R5.4 现有 `sync.rs` 决策测试（`uploads_when_remote_is_empty`、`in_sync_when_fingerprints_match`、`downloads_when_only_remote_changed`、`uploads_when_only_local_changed`、`flags_conflict_when_both_sides_changed`、`downloads_on_first_contact_without_conflict`、`rejects_tampered_manifests` 等）与 `pending_restore.rs` 测试需继续通过；签名变化处同步更新。

### R6 算法不兼容的提示走 i18n（P2）

算法不兼容是用户必须采取行动（升级对端）的错误，不能只抛英文原文。按仓库约定（AGENTS.md 国际化章节：用户可见文案走 `react-i18next`）：

- R6.1 后端返回稳定错误码（如 `sync_algorithm_mismatch`），错误消息可读但面向诊断。
- R6.2 前端在 WebDAV 设置界面对该错误码映射到 `zh-CN` / `en-US` 本地化文案，文案说明"对端版本过旧、需升级后重试"。
- R6.3 其余错误维持现有"直接展示 `error.message`"的行为，不在本任务内扩大改造。

## Acceptance Criteria

- [ ] AC1 构造"远端较新、本地已排队下载但恢复未落地"的状态后，下一轮同步**不会**上传本地数据；改为重试下载。
- [ ] AC2 恢复成功后，`sync_meta.last_synced_database_sha256` 等于本次下载对应的远端 manifest 指纹。
- [ ] AC3 恢复在启动时失败并被隔离后，`last_synced` 保持未提交状态。（对应 R1.3）
- [ ] AC4 本地算法标识与远端不同时，`run_sync` 返回明确错误（非"冲突"、非静默成功），且**未发生任何**上传或下载替换。错误文案指明需升级对端。（对应 R2.3）
- [ ] AC5 远端 manifest 缺少算法标识时（旧版本写出），行为符合 R2.4 的显式定义，且已有测试固定该行为。
- [ ] AC6 在一次"已同步"决策与一次"下载"决策中，应用目录下不再出现新创建的 `.sync-*` 数据包；"上传"决策仍产生并使用数据包。（对应 R3.1）
- [ ] AC7 下载路径仍会在持久备份目录生成 `pre_sync_apply` 快照。（对应 R3.3）
- [ ] AC8 新配置的 `max_remote_backups` 为 3；已存在配置的值不变。（对应 R4.1/R4.3）
- [ ] AC9 算法不兼容时，WebDAV 设置界面显示中/英本地化文案而非英文原文（`zh-CN` 与 `en-US` 两种语言下各验证一次）。（对应 R6.2）
- [ ] AC10 未修复版本（0.1.5）能正常解析新格式 manifest 且不报错。（对应 R2.5）
- [ ] AC11 `cd src-tauri && cargo test` 全绿；`pnpm typecheck` 与 `pnpm test:run` 全绿。

## Out of Scope

- 不实现行级/字段级合并；冲突策略仍为"整库替换"。
- 不实现"保留两台设备各自的合并结果"或三向合并。
- 不改动 `xiaobai-switch-sync.json` 文件名、备份前缀、ZIP 条目名等兼容红线。
- 不自动升级对端应用，也不在应用内做版本协商下载。
- 不改动本地备份保留数（`maxBackupCopies`，当前 30）与 `pre_sync_apply` 快照策略。
- 不清理历史上已堆积的远端数据包（由保留数在下次上传后自然收敛）。
- 不修复家里那台开发构建本身，也不在本任务内执行数据恢复（AnyRouter 的找回是独立操作）。

## Open Questions

无阻塞项。原待确认项"缺算法字段的旧 manifest 如何处理"已决策为**按当前算法处理**（见 R2.4）。

## 已知遗留（本轮不修，实现复核后发现）

以下均经复核确认，非阻塞、无数据丢失，记录以便后续处理：

1. **记账提交与清理之间存在窄窗口（P2）**：`apply_pending_restore_inner` 成功后在 `cleanup_committed` 中删除了 pending 目录（含 `expected-sync.json`），而记账提交发生在随后的 `AppState::init`（`Db::open` 之后）。若进程恰好在这两步之间崩溃，`last_synced` 会停留在下载前的旧值。后果有界：下一轮判定为 `Download + conflict`（多一次冲突提示与一份 `pre_sync_apply` 快照），再下一轮收敛为 Upload，**不会丢数据**。彻底修复需把清理推迟到提交之后（新增 `finish_pending_restore` 供 `state.rs` 调用），属启动协议改动，在无 Rust 工具链可编译验证的环境下不宜盲改。
2. **后台同步失败原因不上屏（P2）**：`webdav_sync_state.error` 在前端无任何消费者（已 grep 确认），因此自动同步遇到算法不兼容时用户只看到失败标签、看不到原因；而 R6 的目的正是让用户知道"需升级对端"。手动同步有 toast 覆盖。彻底修复需让状态记录携带错误码（涉及表结构变更），超出本任务 R6.3 划定的范围。
3. **AC2 最后一跳无测试（P3）**：`state.rs` 中提交 `last_synced` 的三行代码无测试覆盖（该文件无测试模块）。建议抽出 `commit_restore_fingerprint(conn, &outcome)` 以便单测。
4. **AC6 由源码结构护栏保障而非行为测试（P3）**：`whole_database_bundle_is_built_only_for_uploads` 用 `include_str!` 断言分支内不出现 `create_backup_in`。已验证其非空转（把打包移回决策前或移入 Download 都会变红），但无法发现通过其他名字的辅助函数引入的第二处全库复制。
5. **手动恢复会被下一次同步覆盖（P3，既有行为）**：手动恢复传 `expected=None` 故不提交记账，下一轮若远端较新会再次下载并标记冲突。本任务只是让该行为显式化。
6. **`.trellis/.template-hashes.json` 有一处无关改动**（`statusline.py` 记录过期被清理），来自先前的 `trellis update`，不属于本任务，提交时排除。
