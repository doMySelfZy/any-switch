---
title: 应用到 Prime
description: 使用 Prime 官方 models.json、auth.json 与 settings.json 接入站点，并保留其他 Provider 和 OAuth。
order: 8.5
---

应用中心左侧选 **Prime**。默认目录按「应用路径设置 → `PRIME_AGENT_CODING_AGENT_DIR` → `~/.prime/agent`」解析，不要写到 `~/.pi/agent`。

## 写入内容

XiaoBaiSwitch 使用 Prime 官方配置接口，不安装 extension：

- `models.json`：写入一个 `xiaobai_` Provider、Base URL、协议和模型列表
- `auth.json`：写入对应 API Key
- `settings.json`：为新会话设置默认 Provider 与模型

`openai_compatible` 映射为 `openai-completions`，Base URL 使用带 `/v1` 的形式；`anthropic` 映射为 `anthropic-messages`。

## 模型目录

关闭「将站点全部模型写入 Prime」时，只写当前默认模型；开启后写入站点模型列表，可在 Prime 中打开 `/model` 重新加载和切换。

首版只写可靠的模型 ID 与显示名称。`reasoning`、图像输入、上下文窗口、最大输出和兼容选项使用 Prime 官方默认值，不根据模型名称猜测。

## 保留与单活

Prime 中同时只保留一个 XiaoBaiSwitch 管理的 Provider。切换站点只替换 `xiaobai_` 命名空间；其他自定义 Provider、OAuth 登录、注释、尾逗号和未知设置都会保留。

全局默认值可能被项目级 `.prime/agent/settings.json`、CLI 参数或恢复会话覆盖，这不会让受管 Provider 变成过期状态。

## 移除 XiaoBai 配置

此操作只删除受管 Provider 和对应凭据。当 Prime 当前默认值仍指向它时，应用才恢复首次写入前保存的默认值，避免覆盖用户后来在 Prime 中做出的选择。

`auth.json` 含明文密钥，Unix 上权限设为 `0600`。详见 [安全说明](../security/)。
