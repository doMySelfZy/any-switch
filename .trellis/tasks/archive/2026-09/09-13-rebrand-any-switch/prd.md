# 品牌更名：XiaoBaiSwitch → AnySwitch

## Goal
把产品从「XiaoBaiSwitch / 小白Switch」全面更名为「AnySwitch」，覆盖应用、文档、仓库、发布物与更新链路；对既有用户数据/备份/已应用配置保持兼容，不丢数据。

## Requirements

### 命名映射
| 项 | 旧 | 新 |
|---|---|---|
| 产品名/二进制 | XiaoBaiSwitch | AnySwitch |
| 标识符 | com.domyselfzy.xiaobai-switch.app | com.domyselfzy.any-switch.app |
| 数据目录 | ~/.xiaobai-switch | ~/.any-switch（带迁移） |
| 数据库文件 | xiaobai-switch.db | any-switch.db（迁移时重命名） |
| 深链 | xiaobaiswitch:// | anyswitch://（旧链接失效，用户已知悉） |
| 安装包 | XiaoBaiSwitch_x.y.z_x64-setup.exe | AnySwitch_x.y.z_x64-setup.exe |
| 仓库 | doMySelfZy/xiaobai-switch | doMySelfZy/any-switch（GitHub 改名 + Gitee 新建） |
| 测试数据目录覆盖 | XIAOBAI_SWITCH_DATA_DIR | ANY_SWITCH_DATA_DIR |
| 版本 | 0.0.11 | 0.1.0（AnySwitch 首个版本） |

### 兼容红线（改错会破坏既有数据）
1. **本地备份文件名前缀**：新前缀 `any-switch-backup-`，但必须**双前缀识别**（列表/恢复/清理都接受新旧），否则用户历史备份不可见。
2. **WebDAV 同步 manifest 文件名**：`xiaobai-switch-sync.json` **保持不变**（内部协议标识）。改名会导致新旧版本机器互相读不到版本指针 → 误判后相互覆盖。
3. **已应用配置命名空间 `xiaobai_`**（Pi/Prime provider、Codex provider id）：**保持不变**，`restore_official` 识别逻辑不动，否则旧 Apply 痕迹无法识别/清理。
4. 迁移**不删除**旧目录 `~/.xiaobai-switch`（作为回滚点保留）。

### 数据迁移设计
启动时（Db::open 之前，paths 层）：
1. `~/.any-switch` 不存在且 `~/.xiaobai-switch` 存在 → 同盘 `rename`；跨盘失败则 copy + 校验（db sha256、master.key 32 字节）后保留旧目录。
2. 两者都存在 → 使用新目录，不覆盖，日志提示旧目录仍在。
3. 新目录内 `xiaobai-switch.db` 存在且 `any-switch.db` 不存在 → rename。
4. 迁移失败 → 回退使用旧目录（应用仍可用），记录错误。

### 实施批次
- 批次 1｜代码层命名：tauri.conf.json、Rust（paths/深链/通知/托盘/文案）、前端（i18n/appDisplayName/常量/标题）、AGENTS.md。
- 批次 2｜兼容层：备份前缀双识别、目录迁移逻辑 + 单测（新目录已存在不覆盖 / 跨盘回退 / 迁移失败回退旧目录）。
- 批次 3｜文档与站点：README 中英、website/、cliff.toml、Release 说明模板。
- 批次 4｜仓库与发布：GitHub 改名（旧 URL 自动跳转）、remote 更新、Gitee 建仓 + 推送 main/tags、updater 端点更新。
- 批次 5｜验证：cargo test + pnpm typecheck/test 全绿；本地装包验证迁移；旧备份可列出/恢复；`anyswitch://` 可导入。

## Acceptance Criteria
- [ ] 全新安装：数据在 `~/.any-switch`；带旧数据的安装：自动迁移且站点/密钥/历史备份可见可恢复。
- [ ] 应用名、窗口标题、托盘、安装包、关于页、文档无残留 "小白/XiaoBaiSwitch"。
- [ ] GitHub 与 Gitee 仓库均为 `any-switch`，更新端点指向新地址。
- [ ] 旧命名空间 `xiaobai_` 的已应用配置仍能识别，官方配置可正常恢复。
- [ ] 令牌不被写入任何文件/提交，用后提示用户重置。

## Notes
- 仓库操作由 AI 使用用户提供的令牌执行；令牌仅用于单次命令，不落盘。
- 回滚：改动集中提交；迁移不删旧目录；GitHub 改名可再改回。
