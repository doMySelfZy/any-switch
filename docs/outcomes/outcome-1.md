<!-- Trellis managed file. Remove this marker to take manual ownership. -->
# 配置变更自动上传同步

## Actor
- 多机个人用户

## Trigger
用户在本机修改了任意配置（站点、目标、应用设置）

## Walkthrough
- 用户正常保存配置（无感知）
- 应用检测到数据版本变化，短暂防抖合并多次改动
- 生成新的数据包上传 WebDAV 并更新远端版本指针

## Success State
远端 WebDAV 上始终是最近一次变更的数据，无需手动备份

## Failure State
上传失败时本地数据不变，标记待重试并在 UI 提示，绝不静默丢包

## Verification Steps
- 改一个站点后不做任何操作，WebDAV 上出现新版本
- 断网时改配置本地正常，恢复网络后自动补传

## Sensitivity
- Level: high
- High stakes: No
