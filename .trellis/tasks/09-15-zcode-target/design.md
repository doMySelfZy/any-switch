# 技术设计

## 关键决策

### 1. 协议必须让用户选，`site.protocol` 只做默认值
调研发现用户在同站点上的选择与本站 `protocol` **无一致规律**（AgentRouter 我们记 `anthropic`、他在 ZCode 选 `openai-compatible`；SHUAI API 我们记 `openai_compatible`、他选 `anthropic`）。且 ZCode 的 `api.type` 有三种，比我们的两种 protocol 更细。

因此新增列 `sites.zcode_api_type TEXT`，取值 `anthropic-messages` / `openai-responses` / `openai-chat-completions`。应用中心的 ZCode 面板让用户选；未设置时按 `site.protocol` 推断（`anthropic` → `anthropic-messages`，否则 → `openai-responses`）。

`kind` 与 `api.type` 必须配套写入两处，映射固定：

| 用户选（api.type） | config.json 的 kind | provider_config 的 api.type |
|--------------------|---------------------|------------------------------|
| `anthropic-messages` | `anthropic` | `anthropic-messages` |
| `openai-responses` | `openai-compatible` | `openai-responses` |
| `openai-chat-completions` | `openai-compatible` | `openai-chat-completions` |

**不需要升 schema 版本号**：只加列，走 `ensure_incremental_schema` 的 `ensure_column`；`version >= SCHEMA_VERSION` 的早退分支会跑到它。

### 2. providerId 派生：`xiaobai_<site_id 前 16 位去横线>`
与 Codex provider id 同一套做法，保证：
- 可识别归属（`xiaobai_` 前缀是既有兼容红线的一部分）
- 幂等（同站点重复应用得同一 id，不会越写越多）
- 不与用户自带 provider 冲突

ZCode 的 providerId 是任意字符串（`providerOrder` 里观察到非 UUID 的 `new-provider`），所以非 UUID 的 id 可用。

### 3. 两处配置的写入顺序与一致性
两个文件（`v2/config.json`、`v2/provider_config.json`）都必须改。策略：

1. 各自**先备份**到 `backups/zcode/`
2. 计算两份新内容（纯函数，便于单测）
3. 依次原子替换
4. 任一失败 → 报错；此时可能只有一处已更新

**为什么不做跨文件事务**：写入是幂等的（同样的站点状态产出同样的两份内容），所以用户**重新应用一次即可收敛到一致状态**。报错信息会明确这一点。加跨文件回滚的复杂度与收益不成正比。

### 4. 清理与保留
- 只动 `xiaobai_` 前缀的 provider（两处的 `providerRules`、`personalModelIds`、`providerModelRules` 都按 providerId 过滤）
- `providerOrder` 里的非 `xiaobai_` 项（含 `new-provider` 这类占位）**原样保留并保持相对顺序**
- 用户其它 provider 的 `models`、`options`、未知字段一律不动
- 形状不合法（如 `provider` 不是对象、`providerRules` 不是数组）→ **报错并原样保留文件**

### 5. MCP 独立于站点应用
ZCode 的 MCP 在 `cli/config.json`，与 provider 无关。适配器复用既有 `merge_servers_into_json`（ZCode 的 `mcp.servers` 是标准 JSON 对象），只多一层「根对象是 `mcp.servers`」的路径处理。

### 6. 路径解析
`ZCODE_HOME`（环境变量）→ 应用设置 `zcode_home_override` → `~/.zcode`，与既有四端同样的三级优先级。
- provider：`<home>/v2/{config.json,provider_config.json}`
- MCP：`<home>/cli/config.json`

## 数据流
```
应用：apply_site(ZCode) → adapters::zcode::apply_*（备份 → 两份内容 → 原子替换）
扫描：adapters::mcp_scan（新增 ZCode）读 provider 与 mcp.servers，标 managed
纳管：走既有 load_entry_for_import 的同一套（MCP）；provider 纳管见「不做」
```

## 兼容与安全
- apiKey 明文写进 ZCode 配置（ZCode 自己就这么存）→ UI 须提示这一点，与既有四端的说明一致
- 不改 ZCode 的协议字段语义；不改 `credentials.json`
- MCP 的托管前缀、清理规则、锁与原子写全部复用现有实现

## 分阶段
1. **后端**：`TargetKind::ZCode` + paths + domain 字段 + 迁移加列 + 既有 match 分支补齐（编译驱动）
2. **adapters/zcode.rs**：两份内容的构造（纯函数）+ 备份 + 原子替换 + 单测
3. **MCP**：zcode 适配器 + 扫描/纳管接入
4. **前端**：目标列表 + ZCode 面板 + 协议选择 + i18n + mock
5. **验证**：单测 / 全量测试 / 真机写入后 ZCode 能读到
