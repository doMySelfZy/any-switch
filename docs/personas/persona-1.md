<!-- Trellis managed file. Remove this marker to take manual ownership. -->
# 多机个人用户 — Persona

> Role: XiaoBaiSwitch 使用者（家庭与公司多台电脑）

## Goals
- 在任何一台机器打开 XiaoBaiSwitch 都直接是最新配置
- 配置改动后无需任何手动备份或恢复操作

## Constraints
- 经常在两台以上机器之间切换
- 不愿手动干预同步过程

## Environment
Windows / macOS 桌面客户端，通过同一个 WebDAV 账户互通

## Data Sensitivity Exposure
- high

## Decision Authority
- decides

## Pain Points
- 换机器必须手动去 WebDAV 里做恢复
- 60 分钟定时备份不是真同步，切机后数据可能是旧的
