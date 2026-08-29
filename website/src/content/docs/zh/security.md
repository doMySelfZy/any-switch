---
title: 安全说明
description: 密钥在应用内加密；Apply 之后可能以明文出现在 Claude / Codex / Pi / Prime 配置和 shell 环境中。
order: 13
---

API Key 在应用数据库中 **加密存储**（`~/.xiaobai-switch/xiaobai-switch.db` + `master.key`）。站点列表与详情只展示前缀；打开编辑弹窗时会按需解密到默认隐藏的密码输入框，点击眼睛后可查看完整值。日志不会回显原始密钥。

<div class="not-prose">
<div role="alert" class="alert alert-warning my-4">
<p><strong>应用（Apply）之后</strong>，密钥可能以明文出现在 <code>~/.claude</code>、<code>~/.codex</code>、<code>~/.pi/agent/auth.json</code>、<code>~/.prime/agent/auth.json</code>、<code>codex.env</code> 或 shell rc 中。请勿把这些目录同步到不可信云盘。</p>
</div>
</div>

## 数据根目录

只使用 `~/.xiaobai-switch/`，不要把它理解成系统应用支持目录或 bundle id 路径。

Unix 上 `master.key` 权限为 `0600`。丢失主密钥将无法解密库里的密钥材料。

## 公开链接

不要把真实 API Key 放进 `xiaobaiswitch://` 公开页面或聊天记录。省略 `apikey`，让用户在应用内补全。详见 [链接导入](../import-link/)。
