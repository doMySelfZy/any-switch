<!-- Trellis managed file. Remove this marker to take manual ownership. -->
# 切机自动拉取最新数据

## Actor
- 多机个人用户

## Trigger
在另一台机器上打开或唤醒 XiaoBaiSwitch

## Walkthrough
- 应用启动/窗口聚焦时读取远端版本指针
- 远端版本比本地新时自动下载并校验数据包
- 应用前先做本地快照，再原子替换本地数据并刷新界面

## Success State
打开即最新，全程无需手动 WebDAV 恢复

## Failure State
拉取或校验失败时保持本地数据原样并明确报错，不出现半套数据

## Verification Steps
- A 机改配置后 B 机启动即见最新
- 远端数据包损坏时 B 机本地数据不受影响且给出提示

## Sensitivity
- Level: high
- High stakes: No
