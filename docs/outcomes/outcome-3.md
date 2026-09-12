<!-- Trellis managed file. Remove this marker to take manual ownership. -->
# 同步不丢数据

## Actor
- 多机个人用户

## Trigger
两台机器同时修改配置，或同步过程中途失败

## Walkthrough
- 每次应用远端数据前自动生成本地快照
- 检测到冲突时按设备名+时间戳保留双方版本供追溯
- 任何时刻可从快照一键回滚

## Success State
任何同步操作可追溯、可回滚，最坏情况回到应用前快照

## Failure State
冲突未确认前不覆盖本地未同步的变更

## Verification Steps
- 模拟双机同时修改，双方变更均可追溯
- 回滚快照后本地数据恢复到同步前状态

## Sensitivity
- Level: high
- High stakes: Yes
