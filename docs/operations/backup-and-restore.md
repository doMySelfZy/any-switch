<!-- Trellis managed file. Remove this marker to take manual ownership. -->
# Backup And Restore

- Backup approach: 同步前自动本地快照 + WebDAV 版本化数据包（变更触发，定时仅兜底）
- Restore approach: 快照一键回滚；远端数据包校验后原子应用
