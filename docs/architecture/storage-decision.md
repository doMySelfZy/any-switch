<!-- Trellis managed file. Remove this marker to take manual ownership. -->
# Storage Decision

- Storage mode: local-plus-user-webdav
- Aggregation boundary: 仅设备级，无服务端聚合
- Recovery assumptions:
- 每次应用远端数据前自动生成本地快照
- 远端数据包损坏或校验失败时回退本地现状
- 快照保留数量可配置并自动清理
