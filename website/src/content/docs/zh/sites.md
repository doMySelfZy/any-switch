---
title: 站点中心
description: 管理上游名称、线路、协议、API Key 和 Codex 私有能力预设。
order: 4
---

站点代表一个上游中转：一组 Base URL + 一把 API Key，再挂模型与目标预设。

## 基本字段

| 字段 | 说明 |
|------|------|
| 显示名称 | 最长 128 |
| 线路 / Base URL | 可多条，最多 20 条；第一项为当前 / 默认线路 |
| API Key | 加密存入应用数据库；列表只展示前缀，编辑时按需解密到默认隐藏的密码输入框 |
| 连接协议 | `OpenAI 兼容`（默认）或 `Anthropic` |
| 备注 | 可选，最长 2000 |

高级配置（协议、备注）默认收起。

## API Key 额度

打开站点详情时，会用当前线路和已保存的 API Key 探测上游是否实现 OpenAI 兼容的计费接口：

- `GET /v1/dashboard/billing/credit_grants`
- `GET /v1/dashboard/billing/subscription`
- `GET /v1/dashboard/billing/usage`
- `GET /api/usage/token`（New API / 同类中转的 Key 额度，可能是 CNY）

解析成功才显示剩余 / 已用 / 总额；可手动刷新。官方 OpenAI 或 Anthropic 的用户 Key 通常查不到余额，此时该行会隐藏，不算错误。

## 多线路

第一项是当前默认线路。可测速、切换；切换行为见 [线路切换](../routes/)。

写入预览会分别显示拉模型 URL、Claude Base URL、Codex `base_url`，方便核对路径是否带 `/v1` 等后缀。

## Codex 私有能力预设

站点编辑里有默认收起的「Codex私有能力」，kebab 键与导入链接一致：

- `codex-compact` 远程压缩
- `codex-vision` 识图
- `codex-imagegen` 生图
- `codex-search` 内置搜索

多数中转并不具备这些能力，请按实际上游来开。应用中心可以跟随站点预设，也可以仅本次覆盖。

## 启用与停用

停用已应用到工具的站点时，可选择是否一并清除这些目标里的配置。不清除的话，目标会显示为「遗留」。
