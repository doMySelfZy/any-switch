# 把 ZCode 加为第五个应用目标

## 目标
像 Claude Code / Codex / Pi / Prime 一样，让站点与 MCP 能应用到 **ZCode**，并支持扫描/纳管 ZCode 里已有的配置。完成后本应用统一管控五个 Agent 客户端。

## 调研结论（已在本机核实，2026-09-15）

配置根目录 `~/.zcode/`（本机为 `C:\Users\<user>\.zcode`）。

### MCP：`~/.zcode/cli/config.json` 的 `mcp.servers`
```json
{ "mcp": { "servers": {
  "exa": { "type": "stdio", "command": "npx", "args": [...], "enabled": true }
}}}
```
**与现有四端的格式基本一致**（`command`/`args`/`type`/`enabled`），改造成本低。本机现有 3 个（`sequential-thinking` / `ssh-mcp-server` / `exa`）。

### 站点（供应商）：需要**同时写两处**
ZCode 把 provider 拆成两份表示，两处都含明文 apiKey：

1. **`~/.zcode/v2/config.json` → `provider.<providerId>`**
```json
{ "<providerId>": {
  "name": "shuai-claude",
  "kind": "anthropic | openai | openai-compatible",
  "options": { "apiKey": "...", "baseURL": "https://api.shuaiapi.com/v1", "apiKeyRequired": true },
  "source": "custom",
  "models": { "<modelId>": { "limit": {"context": N, "output": N},
                            "modalities": {"input": [...], "output": [...]},
                            "zcode": {"modalitiesConfigured": true} } }
}}
```

2. **`~/.zcode/v2/provider_config.json` → `config`**
```json
{ "schemaVersion": 1, "config": {
  "providerOrder": ["<providerId>", ...],
  "providerConfigRules": { "providerRules": [{
      "providerId": "...", "providerName": "shuai-claude",
      "config": { "group": "standard-personal",
                  "access": { "type": "api-key", "apiKey": "..." },
                  "api": { "type": "anthropic-messages", "baseUrl": "..." },
                  "personalModelIds": [...], "modelOrder": [...] } }] },
  "modelConfigRules": { "providerModelRules": [{ "modelId": "...", "providerId": "...",
                        "config": { "properties": { "contextWindow": N } } }],
                        "manualProviderModelRules": [...] }
}}
```

`providerOrder` 里还观察到非 UUID 的占位项 `"new-provider"`，写入时**必须原样保留**。

### 协议映射（关键：不能靠推断）
`kind` 与 `api.type` 需配套，但**与本站点的 `protocol` 不对应**：

| 站点 | 本站 protocol | 用户手工在 ZCode 选的 kind / api.type |
|------|---------------|--------------------------------------|
| SHUAI API | openai_compatible | anthropic / anthropic-messages |
| AiHub | openai_compatible | openai / openai-responses |
| JustWoker | openai_compatible | anthropic / anthropic-messages |
| AgentRouter | **anthropic** | openai-compatible / openai-responses |
| OpenCode | openai_compatible | openai-compatible / openai-chat-completions |
| ZzzCoding | openai_compatible | anthropic / anthropic-messages |

两个观察：
- **同一 `kind` 可对应不同 `api.type`**（`openai-compatible` 既有 `openai-responses` 也有 `openai-chat-completions`）。
- 用户的选择与本站 protocol **无一致规律**（AgentRouter 我们记 anthropic、他选 openai）。

因此 **ZCode 目标必须让用户自己选协议**，不能只按 `site.protocol` 推断；`site.protocol` 只作为初始默认值。

### 现状价值
本机 ZCode 里已手工配了 6 个 provider，正好对应 6 个站点——**这正是本应用该替他做的事**，需求真实。

## 需求

### 后端
1. 新增目标类型 `TargetKind::ZCode`（第 5 个），贯穿 domain / paths / adapters / commands / 前端目标列表。
2. **站点应用**：写 ZCode 的两处配置，只管理 `xiaobai_` 命名空间派生的 providerId；保留用户其它 provider、`providerOrder` 占位项与未知字段。
   - 协议由用户在 ZCode 面板选择（3 选 1：`anthropic-messages` / `openai-responses` / `openai-chat-completions`），默认按 `site.protocol` 推断。
   - 模型目录：把站点模型写入 `config.json` 的 `models` 与 `provider_config.json` 的 `personalModelIds`/`modelOrder`/`providerModelRules`。
3. **MCP**：写 `~/.zcode/cli/config.json` 的 `mcp.servers`，复用现有托管前缀与合并逻辑。
4. **扫描/纳管**：读 ZCode 的 provider 与 MCP，纳入现有扫描结果（自产的 `xiaobai_` provider 标记为已托管）。
5. 路径解析：`ZCODE_HOME` 环境变量 → 应用设置覆盖 → `~/.zcode`，与现有四端一致的优先级。
6. 写入前备份 + 原子替换 + `.lock` 互斥；形状不合法时报错且原样保留。

### 前端
7. 应用中心新增 **ZCode** 目标页（左侧目标列表第 5 项），含协议选择、默认模型、写入模型目录开关。
8. 站点表单的"应用到 ZCode"语义与其它目标一致；扫描/纳管列表出现 ZCode 条目。

## 验收标准
- [ ] 站点可应用到 ZCode，两处配置都被正确写入，且 `providerOrder` 中的占位项与其它 provider 原样保留。
- [ ] 用户选择的协议如实落到 `kind` 与 `api.type`（两处一致）。
- [ ] MCP 可应用到 ZCode 的 `mcp.servers`，托管前缀与清理行为与其它四端一致。
- [ ] 扫描能列出 ZCode 里已有的 provider 与 MCP；自产的标记为已托管，不提供纳管。
- [ ] 既有四个目标行为不变；备份、原子写、`.lock`、失败不覆盖等既有约定全部保持。
- [ ] `cargo test`、`pnpm typecheck`、`pnpm test:run` 全绿；真机验证一次写入后 ZCode 能读到。

## 明确不做
- 不改 ZCode 自己的配置协议（只读写既有字段）。
- 不动 `credentials.json`（OAuth token 等与本应用无关）。
- 不做 ZCode 的会话/项目数据（`setting.json` 的 recentProjects 等）。
