---
title: 安装
description: 从 GitHub Releases 下载 macOS 与 Windows 安装包，处理 macOS「已损坏」提示。
order: 2
---

支持 **macOS**（Apple Silicon / Intel）与 **Windows**（x64 / ARM64）。请到 [Releases](https://github.com/doMySelfZy/xiaobai-switch/releases) 下载对应安装包，或打开官网 [下载页](/download/)。

## 安装包形态

| 平台 | 安装包 |
|------|--------|
| macOS Apple Silicon | `.dmg`（aarch64） |
| macOS Intel | `.dmg`（x86_64） |
| Windows x64 | `.msi` / NSIS `.exe`，另有 portable zip |
| Windows ARM64 | 仅 NSIS `.exe`，另有 portable zip |

## macOS 提示「已损坏」

macOS 安装包是 **ad-hoc 签名**（未使用 Apple Developer ID，也未公证）。浏览器下载后，系统可能提示「已损坏」——这是隔离属性，不是文件坏了。**「隐私与安全性」不会出现「仍要打开」。**

把应用拖到「应用程序」后执行：

```bash
xattr -cr /Applications/XiaoBaiSwitch.app
```

然后右键点应用 → 打开。

## 自动更新

正式发布的桌面端会检查更新，并支持在应用内下载安装。可在 **设置 → 关于** 手动检查，或开启自动检查。

官网与文档地址是 [https://xiaobaiswitch.com](https://xiaobaiswitch.com)。
