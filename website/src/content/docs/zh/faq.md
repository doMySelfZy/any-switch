---
title: 常见问题
description: macOS 打不开、配置不生效、密钥展示和导入行为。
order: 14
---

## macOS 说应用已损坏

见 [安装](../install/)。执行 `xattr -cr /Applications/XiaoBaiSwitch.app` 后右键打开。

## 应用成功了，CLI 还是旧地址

重启终端，或完全退出再打开 Claude Code / Codex / Pi。应用只写配置文件，不会热替换已经启动的进程；Pi 可打开 `/model` 重新加载模型列表。

## 如何查看完整 API Key？

站点列表与详情只显示前缀。打开站点编辑弹窗，等待完整密钥加载后，点击输入框右侧的眼睛即可查看。密钥仍加密保存在应用数据库中。

## 导入链接会不会直接改 Claude / Codex？

不会。导入只创建或更新站点，必须在应用中心手动应用。

## 支持 Linux 吗？

当前发布包是 macOS 与 Windows。

## 官网地址是什么？

[https://xiaobaiswitch.com](https://xiaobaiswitch.com)（GitHub Pages 自定义域名）。文档在 `/docs/`，英文在 `/en/`。
