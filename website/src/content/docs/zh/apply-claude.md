---
title: 应用到 Claude Code
description: 为 Claude Code 选择站点、模型、鉴权字段、别名映射和 effort，写入 settings.json。
order: 6
---

应用中心左侧选 **Claude Code**。每个目标是独立表单，不会和 Codex 共用勾选框。

写入路径默认是 `~/.claude/settings.json`（可在设置里覆盖 Claude 配置目录）。

## 站点与模型

- 选择已启用的站点
- **默认模型**写入顶层 `model`，之后仍可使用 Claude Code 的 `/model` 切换并保存
- 请先确认站点里已有模型

同一 model id 可能不被 Claude Code、Codex 与 Pi 同时接受；若提示多目标冲突，仍可强制应用，但通常应分开选。

## 鉴权字段

仅影响 Claude Code：

- `ANTHROPIC_AUTH_TOKEN`（推荐）
- `ANTHROPIC_API_KEY`

设置里可打开「Claude 强制独占鉴权键」：写入选定键时删除另一个字段。

## 模型别名映射

把 Claude Code 内置的 **fable / opus / sonnet / haiku** 别名映射到当前站点的模型 id。可与默认模型相同，也可以清空。使用自定义 model id 时，Claude Code 默认显示 `Custom <family> model`，这不代表映射失败。

## 思考等级

Effort 等级写入顶层 `effortLevel`，可选 Low / Medium / High / Extra High（`xhigh`）。这不会锁定 Claude Code 的交互式选择，仍可使用 `/effort` 修改并保存支持的等级；`max` 只用于当前会话，不会持久化。

重新应用旧配置时，XiaoBaiSwitch 会迁移旧 binding，并移除 `settings.json` 中会覆盖交互式选择的 `ANTHROPIC_MODEL` 与 `CLAUDE_CODE_EFFORT_LEVEL`。应用不会删除用户在 shell 或其他外部环境中设置的同名变量。

## 应用之后

状态卡片会显示已应用站点与摘要。请重启终端或重新打开 Claude Code。

可从应用中心 **还原官方配置**：移除中转相关的 Base URL、鉴权键和模型覆盖，使 Claude Code 回到官方 claude.ai 账号登录。不会删除已保存的官方登录凭证；当前文件会先备份。
