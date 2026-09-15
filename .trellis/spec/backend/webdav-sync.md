# WebDAV 同步引擎

> 本项目的同步引擎约定。**目标是可直接照着改代码**，不是原则宣讲。
>
> 权威源是仓库根的 `AGENTS.md`（兼容红线在那里）。本文档是按层提炼的操作性契约，
> 两处冲突时以 `AGENTS.md` 为准。

涉及文件：`src-tauri/src/sync.rs`（决策与记账）、`src-tauri/src/pending_restore.rs`
（启动恢复协议）、`src-tauri/src/webdav.rs`（传输）、`src-tauri/src/state.rs`（提交点）。

---

## 1. 不变量（改代码前先读这一节）

这四条各自对应一次真实故障，破坏任何一条都会导致**静默丢数据**。

### 不变量 1：下载未落地，不得提交 `last_synced`

`last_synced`（`sync_meta.last_synced_database_sha256` / `..._master_key_sha256`）记录的是
"本地与远端的共同祖先"。判定"只有本地变了 → 上传"的唯一依据就是它。

换库要等**下次启动**（`apply_pending_restore` 在 `Db::open` 之前执行），所以下载时数据尚未
落地。此刻提交 `last_synced` 会留下"远端即共同祖先"的假账：一旦进程在重启前退出、或恢复
失败回滚，下一轮就会判定"只有本地变了"，**用旧数据覆盖云端较新的数据**。

- 提交点唯一：`AppState::init` 中 `Db::open()` 成功之后。
- 传递路径：`queue_pending_restore(…, Some(remote.fingerprint()))` → pending 目录内的
  `expected-sync.json` → `RestoreStartupOutcome.synced_fingerprint` → `save_last_synced`。
- 手动恢复（`restore_local_backup` / `restore_webdav_backup`）传 `None`：不涉及同步记账。

### 不变量 2：提交的是**远端声明的**指纹，不是本地重算值

恢复后的库会在启动时跑 schema 迁移（`ensure_incremental_schema`），可能给旧库补出新表，
从而改变本地重算指纹。此时：

- 记**远端声明值** → `remote == last` → 判定 Upload → 把升级后的数据重新发布。正确。
- 记本地重算值 → `last == local` → 判定 Download → 反复重下。错误。

### 不变量 3：`FINGERPRINT_TABLES` 变更必须递增算法版本

指纹 = 遍历 `FINGERPRINT_TABLES` 的业务行 + `master.key` 哈希。**表清单本身是算法的一部分**：
两端清单不同，同一份数据会算出不同指纹，判定结果恒定相反（一端判"只有本地变了"→上传，
另一端判"只有远端变了"→下载），于是无限互相覆盖。0.1.3（8 表）与 0.1.4+（9 表）真实
发生过这个循环。

- 改 `FINGERPRINT_TABLES` ⇒ 同时递增 `FINGERPRINT_ALGORITHM_VERSION`。
- 护栏测试 `fingerprint_algorithm_version_is_pinned_to_the_table_list` 会拦住漏改。
- 远端算法与本地不符 ⇒ `SyncAction::Incompatible` ⇒ 报错 `sync_algorithm_mismatch`，
  **禁止任何上传与下载**。不兼容不是"冲突"，不要降级成冲突处理。

### 不变量 4：全库打包只在 Upload 分支

`app_backup::create_backup_in` 是 `VACUUM INTO` + zip 全库复制。它只在确定上传时才需要；
放在决策前会让"已同步"和"下载"两种结果白付一次全库复制 + 压缩（窗口每次聚焦都会触发一轮）。
Download 分支的 `create_local_backup(…, "pre_sync_apply", …)` 必须保留——那才是下载路径的兜底。

---

## 2. 契约

### SyncManifest（远端 `xiaobai-switch-sync.json`）

```rust
pub struct SyncManifest {
    pub format_version: u32,                    // == SYNC_FORMAT_VERSION，不得为新增字段而递增
    pub revision: u64,                          // 仅用于展示，不参与新旧判断
    pub device_name: String,
    pub updated_at: i64,
    pub bundle_file_name: String,
    pub database_sha256: String,
    pub master_key_sha256: String,
    pub app_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fingerprint_algorithm: Option<u32>,     // 新增字段，必须可被旧版本忽略
}
```

新增字段的三条硬约束：

1. **字段可选**，且不得引入 `deny_unknown_fields` —— 否则 0.1.5 读新 manifest 会报错，回滚不安全。
2. **不得为此递增 `format_version`** —— 旧版本会因 `format_version != SYNC_FORMAT_VERSION`
   直接拒绝，等于用一次格式升级换掉可回滚性。
3. 缺字段（`None`）语义**固定**为 `LEGACY_FINGERPRINT_ALGORITHM_VERSION`（=1，9 表实现），
   **不是**"等价于当前算法"。后者在今天（当前也是 1）行为相同，但版本号一升到 2，
   旧对端又会被误判为兼容，不变量 3 的循环原样复现。

### 启动恢复协议

```
Download: 下载 → 校验（manifest 指纹 == 包内重算指纹）→ pre_sync_apply 快照
          → queue_pending_restore(archive, app_dir, Some(远端指纹))
              └─ 写入 .pending-restore.<uuid>.migrating/{payload,expected-sync.json}
              └─ 原子改名为 .pending-restore
          → 返回 pending_restart: true → 重启

启动: apply_pending_restore(app_dir)        [Db::open 之前]
        ├─ applied  → 返回 expected-sync.json 里的指纹，清理 pending
        ├─ failed   → 回滚 + 隔离到 .failed-restore-<ms>，synced_fingerprint = None
        └─ committed 分支（上次已换库、只剩清理）→ 同样返回该指纹，清理 pending
      Db::open()                             [此时数据才真正就位，schema 迁移在此发生]
      提交 last_synced = 上面返回的指纹        [仅 applied 且指纹为 Some]
```

- `committed` 标记只在发布成功后写入，**它的存在即代表数据已落地**，缺的只是记账；
  该分支必须回传指纹，否则 `last_synced` 可能永久停留在旧值。
- `expected-sync.json` 解析失败只记 warning 并当作 `None`：记账文件损坏不得阻断数据恢复。
- 恢复失败的路径**绝不**提交记账（否则下一轮反向上传覆盖云端）。

---

## 3. 校验与错误矩阵

| 条件 | 结果 |
|---|---|
| 远端无 manifest | `Upload`（revision = 1） |
| 远端算法与本机不符 | `Err("sync_algorithm_mismatch")`，零替换 |
| 指纹相同 | `InSync` |
| 无上次同步记录（首台/新机） | `Download`（不算冲突） |
| 只有本地变 | `Upload` |
| 只有远端变 | `Download` |
| 两侧都变 | `Download` + `conflict: true` |
| manifest `fingerprint_algorithm == Some(0)` | `Err("sync_manifest_invalid")` |
| manifest 未知算法版本（如 `Some(9)`） | 解析通过；由决策判为 `Incompatible` |
| 下载包内容指纹 ≠ manifest 声明指纹 | `Err("sync_manifest_invalid")` |
| 恢复失败并被隔离 | 不提交 `last_synced`，`clear_sync_status` 不执行 |

---

## 4. Good / Base / Bad

- **Good**：改 `FINGERPRINT_TABLES` 时同步递增 `FINGERPRINT_ALGORITHM_VERSION`，并补一条
  `VERSION_N_TABLES` 快照断言。
- **Base**：只改与指纹无关的逻辑（如清理时机、日志），不动表清单与算法版本。
- **Bad**：为了"让新字段生效"递增 `format_version`；或把不兼容路径写成"跳过同步 + 静默成功"。

---

## 5. 测试要求（断言点）

改同步引擎时至少覆盖：

- `decide_action` 七种组合各一条（现有 6 条 + `Incompatible`）。
- 算法不兼容：远端 `Some(本机+1)` ⇒ `Incompatible`，且**指纹相同也要拦下**（检查顺序在比较之前）。
- 缺字段：`None` + 指纹相同 ⇒ `InSync`；`None` + 只有远端变 ⇒ `Download`。
- `algorithm_matches(None, 2) == false` —— 钉死"缺字段 ≠ 等价于当前算法"。
- 表清单快照断言（防漏改版本号）。
- 恢复链：成功回传指纹、失败回传 `None` 且不动数据、`committed` 分支也回传指纹。
- 打包时机：`run_sync_inner` 分段源码断言（InSync / Incompatible / Download 不得出现
  `create_backup_in`，Download 必须保留 `create_local_backup` + `pre_sync_apply`）。

> **警告**：`whole_database_bundle_is_built_only_for_uploads` 是源码结构护栏，不是行为测试。
> 它能拦住"把打包移回决策前/移进 Download"，但拦不住通过别名的第二处全库复制。

---

## 6. Wrong vs Correct

#### Wrong —— 下载时立即记账

```rust
// 排队恢复时提交：数据还没落地，假账会让下一轮反向上传旧数据
save_last_synced(state, &remote.fingerprint())?;
crate::pending_restore::queue_pending_restore(&archive, &app_dir, None)?;
```

#### Correct —— 交给启动流程在落地后提交

```rust
// 只传目标指纹；提交点在 AppState::init 的 Db::open 之后
crate::pending_restore::queue_pending_restore(
    &archive,
    &app_dir,
    Some(remote.fingerprint()),
)?;
```

#### Wrong —— 缺字段等价于当前算法

```rust
// 版本升到 2 之后，0.1.5 的对端会被误判为兼容，互相覆盖的循环回归
self.fingerprint_algorithm.unwrap_or(FINGERPRINT_ALGORITHM_VERSION)
```

#### Correct —— 缺字段固定等价于旧版算法

```rust
self.fingerprint_algorithm.unwrap_or(LEGACY_FINGERPRINT_ALGORITHM_VERSION)
```
