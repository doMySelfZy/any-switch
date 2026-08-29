---
title: 简介
description: XiaoBaiSwitch 以站点为中心，把上游 API 接到 Claude Code、Codex、Pi 和 Prime。
order: 1
---

**XiaoBaiSwitch** 是一个桌面应用：小白也能上手，用「上游站点」驱动 Claude Code、Codex、Pi 和 Prime 的配置。

领域上只有一条主线：

**Base URL + API Key → 模型 → 目标私有能力预设 → 应用到目标**

站点是单一事实来源（SSOT）。先把上游填对，再分别写入目标 CLI，避免地址和模型互相漂移。

## 当前目标

- **Claude Code**：写入 `~/.claude/settings.json`
- **Codex**：写入 `~/.codex/`，并按设置注入环境变量
- **Pi**：合并 `~/.pi/agent/models.json`、`auth.json` 与 `settings.json`
- **Prime**：合并 `~/.prime/agent/models.json`、`auth.json` 与 `settings.json`

应用数据在 **`~/.xiaobai-switch/`**（不是系统「应用支持」目录，也不是 Tauri 的 `app_data_dir`）：

```text
~/.xiaobai-switch/
├── xiaobai-switch.db   # 应用状态
├── master.key          # AES-256-GCM 主密钥（Unix 上权限 0600）
└── backups/            # 应用前备份
```

## 你会用到的界面

1. **站点中心**：上游、线路、模型、协议、Codex 私有能力预设
2. **应用中心**：Claude Code、Codex、Pi 与 Prime 各有专用表单
3. **设置**：语言、主题、托盘、备份份数、更新、路径覆盖

下一步：[安装](../install/)，然后走 [快速开始](../quick-start/)。
