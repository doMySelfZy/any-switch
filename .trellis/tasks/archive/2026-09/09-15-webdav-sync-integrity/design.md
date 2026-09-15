# 技术设计：WebDAV 同步引擎完整性修复

## 1. 架构与边界

改动集中在同步引擎，不触碰应用层（应用/还原目标 CLI 配置的逻辑）与 UI Shell。

```
src-tauri/src/
├── sync.rs              ← 核心：指纹算法版本、manifest 契约、决策、记账时机、打包时机
├── pending_restore.rs   ← 恢复预期指纹的落盘与回传
├── state.rs             ← 启动时：恢复成功后提交 last_synced
├── webdav.rs            ← manifest 上传/下载（契约透传，无需逻辑改动）
├── domain/mod.rs        ← 默认值
├── db/migrate.rs        ← 建表默认值
└── commands/webdav.rs   ← 错误码透出、测试夹具默认值
src/
├── components/settings/WebDavBackupSettings.tsx  ← 错误码 → i18n
├── lib/browserMock.ts                            ← 默认值同步
└── i18n（zh-CN / en-US）                          ← 新增文案
```

**不引入新的持久化概念**，除一处：待恢复目录内多一个记录"本次下载目标指纹"的 JSON 文件。

## 2. 契约变更

### 2.1 指纹算法版本（新增常量）

`sync.rs`：

```rust
/// 逻辑指纹的算法版本。`FINGERPRINT_TABLES` 的任何增删都必须同步递增此值：
/// 两端算法不同会对同一份数据算出不同指纹，判定结果恒定相反，进而互相覆盖。
pub const FINGERPRINT_ALGORITHM_VERSION: u32 = 1;
```

取值从 1 开始（0.1.3 的 8 表实现视为"算法 0 之前"，无版本号，见 2.3）。

### 2.2 SyncManifest 新增可选字段

```rust
pub struct SyncManifest {
    // ...既有字段不变...
    /// 写出该 manifest 的机器的指纹算法版本。
    /// 缺省 = 未修复版本写出（0.1.4/0.1.5 的 9 表实现），按当前算法处理。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fingerprint_algorithm: Option<u32>,
}
```

- `#[serde(default)]`（`Option` 自带）保证旧 manifest 能解析。
- **不加 `deny_unknown_fields`**（现状即无），因此 0.1.5 读取新 manifest 会忽略未知字段而不报错 → 满足 R2.5 的"可回滚"。
- 新写出的 manifest 一律填 `Some(FINGERPRINT_ALGORITHM_VERSION)`。
- `validate()` 增加：若为 `Some(v)`，要求 `v >= 1`；**不**因未知版本而拒绝（不兼容是决策问题，不是有效性问题）。

### 2.3 兼容语义

| 远端 manifest 的 `fingerprintAlgorithm` | 处理 |
|---|---|
| 缺失（0.1.4/0.1.5 写出，实为 9 表） | 视为与本机相同 → 正常比较（R2.4 决策） |
| 等于本机 | 正常比较 |
| 存在且不等于本机 | `Incompatible` → 报错，禁止任何替换（R2.3） |

判定函数（`SyncManifest` 方法）：

```rust
fn algorithm_matches_current(&self) -> bool {
    match self.fingerprint_algorithm {
        None => true,                                   // 旧 manifest：按当前算法处理
        Some(version) => version == FINGERPRINT_ALGORITHM_VERSION,
    }
}
```

### 2.4 决策结果新增 `Incompatible`

```rust
pub enum SyncAction { Upload, Download, InSync, Incompatible }
```

`decide_action` 在拿到 `Some(remote)` 后**首先**做算法检查，早于任何指纹比较；远端无 manifest 时不受影响（仍为 Upload）。

调用方映射为错误码 `sync_algorithm_mismatch`，消息含双方版本，例如：
`remote fingerprint algorithm 2 does not match local 1; upgrade the other device`。

**不新增 `SyncOutcome.action` 取值**——不兼容走 `Err`，前端 `action === "upload"` 等既有判断不受影响。

## 3. 数据流

### 3.1 last_synced 记账时机（R1）

现状（有缺陷）：

```
Download: 下载 → 校验 → pre_sync_apply 快照 → save_last_synced(远端指纹) → 排队恢复 → 重启
                                                  ↑ 此时数据尚未替换
```

改为：**下载时只记录"本次下载目标指纹"到待恢复目录；恢复真正应用成功后，才由启动流程提交为 `last_synced`。**

```
Download: 下载 → 校验 → pre_sync_apply 快照 → queue_pending_restore(archive, app_dir, Some(远端指纹)) → 重启
                                                      │ 写入 .pending-restore/<...>/expected-sync.json
                                                      ▼
启动:  apply_pending_restore(app_dir)
         ├─ 成功 → 返回 { result: applied, synced_fingerprint: Some(fp) }
         │         → Db::open → 写入 sync_meta.last_synced = fp   ← 提交点
         └─ 失败 → 回滚 + 隔离 payload → 返回 synced_fingerprint: None（不提交）
```

关键点：**提交必须在 `Db::open` 之后**。恢复后的库是远端库，其自身 `sync_meta` 携带的是上传方**打包时**的旧记账（上传方先打包后上传，快照不含本次发布的新值），因此不能依赖它，必须显式提交。

记账值必须是**远端 manifest 声明的指纹**，而不是恢复后本地重算的指纹：

- 两端算法相同 ⇒ 二者相等；
- 恢复后 schema 迁移补齐了新表（如旧库补出 `agent_rules`）⇒ 本地重算值 ≠ 远端值，此时 `last=远端值` 会让下一轮判定落入 `remote == last` → **Upload**，即"把升级后的数据重新发布"，符合 R1.4；若误记为本地重算值，则会落入 `last == local` → Download，造成反复下载。

### 3.2 打包时机（R3）

`create_backup_in`（`VACUUM INTO` + zip，全库复制）从"决策前"移入 `Upload` 分支；`device_name` 派生随之移入。`temp_dir` 仍按需创建（Upload / Download 各用一次），`InSync` 与 `Incompatible` 不再产生任何数据包。

Download 分支的 `create_local_backup(conn, "pre_sync_apply", …)`（写入持久备份目录）**保留不动**，它才是下载路径的兜底（R3.3）。

## 4. 接口签名变更

```rust
// sync.rs —— 变为接收 Connection，供启动路径复用
fn save_last_synced(conn: &Connection, fingerprint: &DataFingerprint) -> AppResult<()>;  // 原签名取 &AppState

// pending_restore.rs
pub struct RestoreStartupOutcome {          // 新增，仅进程内使用，不参与序列化
    pub result: RestoreStartupResult,       // 原公开类型不变（前端契约不变）
    pub synced_fingerprint: Option<DataFingerprint>,
}
pub fn queue_pending_restore(archive: &Path, app_dir: &Path, expected: Option<DataFingerprint>) -> AppResult<()>;
pub fn apply_pending_restore(app_dir: &Path) -> AppResult<Option<RestoreStartupOutcome>>;

// state.rs
let restore = apply_pending_restore(&app_dir)?;
let db = Db::open()?;
if let Some(outcome) = &restore {
    if outcome.result.status == "applied" {
        db.with_conn(repo::webdav::clear_sync_status)?;
        if let Some(fp) = &outcome.synced_fingerprint {
            db.with_conn(|c| crate::sync::save_last_synced(c, fp))?;
        }
    }
}
```

`DEFAULT_CONFIG` 等前端默认值：`maxRemoteBackups: 10 → 3`。

## 5. 兼容性与迁移

- **manifest 向后兼容**：新字段可选，旧按当前算法处理；新版本读旧、旧版本读新均不报错。
- **不加 `format_version`**：字段为纯增量且可忽略，升级 `format_version` 反而会让旧版本因 `format_version != SYNC_FORMAT_VERSION` 而直接拒绝，破坏可回滚性（R2.5）。
- **保留数默认值**：只改默认值，不改写任何已存在的 `webdav_config` 行（R4.3）。旧库的 `DEFAULT` 子句变更不影响已有行；UI 加载已有值原样回传。
- **兼容红线不动**：manifest 文件名、备份前缀、ZIP 条目名、`xiaobai_` 命名空间均不涉及。
- **数据目录内新增文件**：`.pending-restore/<uuid>.migrating/expected-sync.json`（排队期）+ 成功后随 pending 目录一并清理。失败隔离时随 payload 一起进 `.failed-restore-*`，不残留。

## 6. 权衡

| 决策 | 取舍 |
|---|---|
| 算法不兼容直接报错而非兼容处理 | 同步暂时中断，但绝不静默覆盖。兼容处理无法在算法不同时给出正确比对，任何"继续"都等价于赌运气。 |
| 缺字段按当前算法 | 对 ≤0.1.3 的 8 表对端仍可能覆盖（与今天行为相同，不会更差）；换取升级零死锁。这是**过渡期**让步，新 manifest 一律带字段。 |
| 记账值取远端声明值 | 依赖"发布方 manifest 指纹 == 其包内容指纹"，该不变量由 R5.1 的下载校验保证（下载后重算并比对 manifest，不一致直接报错）。 |
| 新增 `RestoreStartupOutcome` 而非扩 `RestoreStartupResult` | 避免把同步内部状态泄漏进前端可见的 `restore-result.json`。 |
| 用 `Option<u32>` 而非 `#[serde(default)] u32` | 显式区分"未修复版本写出"与"算法 1"，并让过渡策略在代码里可见、可测。 |

## 7. 运维与回滚

- **回滚**：本次改动不改变既有数据的存储结构（只多一个可忽略字段与一个进程内结构），回退到 0.1.5 二进制后可正常读写。已提交的 `last_synced` 语义与旧版一致。
- **上线顺序**：建议先升级一台，确认与另一台（仍未修复）能正常同步；再升级第二台。修复后双方都会在 manifest 里带上算法版本。
- **现场数据**：AnyRouter 的找回不在本任务内；`pre_sync_apply` 快照仍保留在 `~/.xiaobai-switch/backups/app/`。
- **可观测**：后台守护任务的失败仅 `tracing::warn`（现状）。算法不兼容会以 warn 反复出现，据此可诊断；不新增遥测。

## 8. 风险

| 风险 | 缓解 |
|---|---|
| 忘记在表清单变更时递增算法版本 | 固定清单的断言测试：改动 `FINGERPRINT_TABLES` 而未改版本号会导致测试失败 |
| 双进程竞态：重启前用户又触发一次同步 | 已有 `webdav_operation` 互斥锁串行化；且排队恢复期间 `.pending-restore` 存在时再次排队会返回 `restore_pending` |
| 恢复成功后立即崩溃，`last_synced` 未提交 | 下次启动 `pending` 已清理 → 不重复提交；此时本地 == 远端，`decide_action` 走 `remote_fp == local` → InSync，自愈 |
| 旧对端（8 表）在过渡期覆盖数据 | 无法在算法层面消除；通过上线顺序与"两台尽快升级"规避。已在本设计的权衡表中显式记录为接受的残余风险 |
