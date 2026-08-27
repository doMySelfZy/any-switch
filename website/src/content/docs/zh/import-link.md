---
title: 链接导入
description: "用 xiaobaiswitch:// 导入站点；导入不会自动应用到 Claude Code、Codex 或 Pi。"
order: 10
---

安装桌面端后，浏览器或其它应用可以打开 `xiaobaiswitch://` 链接，拉起 XiaoBaiSwitch 并导入上游站点。**导入不会自动应用到工具**，需在应用中心手动确认。

1. 安装并打开桌面端
2. 点击导入链接，应用会切到站点页并弹出确认框
3. 核对名称、线路、协议、备注和 API Key 前缀后确认
4. 若链接没有 `apikey`，确认后会打开预填的添加站点表单，补全密钥再保存

<div class="not-prose">
<div role="alert" class="alert alert-warning my-4">
<p>URL 中携带 API Key 可能被浏览器历史、扩展或系统日志记录，不要把真实密钥写在公开页面。推荐登录后的私有后台按需生成，或省略 <code>apikey</code> 让用户在应用内补全。</p>
</div>
</div>

## 何时视为同一站点

同一协议且线路集合相同（顺序无关）视为同一站点：密钥相同则复用，密钥不同则更新密钥；线路多一条或少一条会新建站点，不会自动合并。

## 链接格式

```text
xiaobaiswitch://sites?name=<name>&baseurls=<url>[&baseurls=<url>…][&apikey=<key>][&protocol=openai_compatible|anthropic][&notes=<notes>][&codex-compact=1][&codex-vision=1][&codex-imagegen=1][&codex-search=1]
```

| 参数 | 必填 | 说明 |
|------|------|------|
| `name` | 是 | 站点名称，最长 128 |
| `baseurls` | 是 | 线路 Base URL，可写多条，最多 20 条 |
| `apikey` | 否 | API Key；省略则确认后打开预填表单 |
| `protocol` | 否 | `openai_compatible`（默认）或 `anthropic` |
| `notes` | 否 | 备注，最长 2000 |
| `codex-compact` | 否 | Codex 远程压缩预设，`1` / `true` / `on` / `yes` 为开 |
| `codex-vision` | 否 | Codex 识图预设 |
| `codex-imagegen` | 否 | Codex 生图预设 |
| `codex-search` | 否 | Codex 内置搜索预设 |

别名：`baseurl` = `baseurls`，`type=openai` / `type=anthropic` = `protocol`。其它符合 `平台-能力` 的 kebab 键会原样存入站点，当前界面只展示 Codex 四个。链接里只要出现任一能力参数，即视为一套完整 Codex 预设（未写的已知键为关）；老链接不加这些参数，不会覆盖站点里已有的预设。

## 多条线路怎么写

第一项是当前 / 默认线路。推荐重复写 `baseurls`，避免 URL 本身带逗号时被拆错：

```text
xiaobaiswitch://sites?name=Example%20Relay&baseurls=https://a.example.com/v1&baseurls=https://b.example.com/v1&protocol=openai_compatible
```

也接受写在同一个参数里，用逗号或 `|` 分隔。`baseurl` 和 `baseurls` 可以混用，按查询串出现顺序合并。
