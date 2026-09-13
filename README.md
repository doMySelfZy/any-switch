<p align="left">
  <strong>中文</strong> · <a href="./README_EN.md">English</a>
</p>

<p align="center">
  <img src="assets/brand/app-icon-1024.png" alt="XiaoBaiSwitch Plus" width="160" height="160">
</p>

# XiaoBaiSwitch Plus

> **本项目是 [Licoy](https://github.com/Licoy) 的 [XiaoBaiSwitch](https://github.com/Licoy/xiaobai-switch) 的 fork。**
> 原作者、原始项目与 MIT 协议归属原作者所有；本仓库只做增强（真同步等），并只从我们自己的 GitHub 发布下载与自动更新。

## 下载与更新

- **下载与自动更新只走 GitHub Releases**：<https://github.com/doMySelfZy/xiaobai-switch-plus/releases>
- 自动更新端点即该 Release 的 `latest.json`。
- **本项目没有官网，也不在 Gitee 发布。**

## 功能（精简）

- 站点优先：Base URL + API Key → 模型 → 目标预设 → 应用到目标。
- 目标：Claude Code、Codex、Pi、Prime，各有独立表单。
- 本地备份 + WebDAV 真同步：多台机器共享一份数据，改完即同步、换机即拉取。
- `xiaobaiswitchplus://` 深链一键导入站点；旧的 `anyswitch://`、`xiaobaiswitch://` 链接仍可识别。
- API Key 在应用内加密存储，应用前后自动备份，可恢复官方配置。

更完整的说明与细节请见 [上游项目](https://github.com/Licoy/xiaobai-switch)。

## 数据目录

- 当前目录：`~/.xiaobai-switch/`（`xiaobai-switch.db`、`master.key`、`backups/`）。
- 从 AnySwitch 版本升级时，首次启动会**自动接管** `~/.any-switch/` 中的数据：按数据库修改时间判断哪份更新，复制迁移并做校验，然后使用新目录。
- 旧目录 `~/.any-switch/` **原样保留**作为回滚点，不会删除；确认稳定后可自行清理。

## 开发

```bash
pnpm install
pnpm tauri dev        # 开发运行
pnpm typecheck        # 类型检查
pnpm test:run         # 前端测试
cd src-tauri && cargo test   # Rust 测试
pnpm tauri build      # 打包
```

## 许可

MIT。原始版权、作者署名与归属见上游 [Licoy/xiaobai-switch](https://github.com/Licoy/xiaobai-switch)。
