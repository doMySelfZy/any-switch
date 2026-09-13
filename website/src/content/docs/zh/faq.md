---
title: 常见问题
description: macOS 打不开、配置不生效、密钥展示和导入行为。
order: 14
---

## macOS 说应用已损坏

见 [安装](../install/)。执行 `xattr -cr /Applications/AnySwitch.app` 后右键打开。

## 应用成功了，CLI 还是旧地址

重启终端，或完全退出再打开 Claude Code / Codex / Pi。应用只写配置文件，不会热替换已经启动的进程；Pi 可打开 `/model` 重新加载模型列表。

## 如何查看完整 API Key？

站点列表与详情只显示前缀。打开站点编辑弹窗，等待完整密钥加载后，点击输入框右侧的眼睛即可查看。密钥仍加密保存在应用数据库中。

## 导入链接会不会直接改 Claude / Codex？

不会。导入只创建或更新站点，必须在应用中心手动应用。

## 支持 Linux 吗？

当前发布包是 macOS 与 Windows。

## 升级后需要卸载旧版 XiaoBaiSwitch 吗？

需要。旧版可能仍被开机自启拉起，两版并行会写同一套目标 CLI 配置且各自持有独立同步记账，可能出现「旧版把过期数据上传、新版又拉回」的反复覆盖。请卸载旧版或至少关闭它的开机自启。另外回滚不再对称：新版备份前缀是 `any-switch-backup-*`，旧版只认旧前缀、看不到新版备份，要回滚新版备份请在 AnySwitch 内操作。

## 官网地址是什么？

[https://any-switch.example.com](https://any-switch.example.com)（GitHub Pages 自定义域名）。文档在 `/docs/`，英文在 `/en/`。
