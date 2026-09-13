# MCP 统一管控与跨设备同步

## 目标
在 XiaoBaiSwitch Plus 中建立 MCP 配置单一事实来源，用户可维护 MCP 服务并选择应用到 Claude Code、Codex、Pi、Prime；应用后由各适配器合并写入客户端配置，配置纳入现有 WebDAV/备份同步。

## 最小行为缺口
当前应用中心只管理模型、鉴权和思考设置，没有 MCP 的持久化模型、统一编辑入口或四客户端写入逻辑；现有同步主要通过数据库与 master.key 备份，需保证 MCP 数据随数据库跨设备同步。

## 数据与边界
- MCP 类型支持 stdio 与 HTTP/SSE。
- 每条 MCP 有稳定 id、名称、启用状态、目标客户端集合、连接字段、环境变量/headers、created_at/updated_at。
- env 与 headers 进入现有 Crypto 加密存储；config 原样透传（因此不要把密钥写进 config）。
- 应用按目标客户端独立备份、原子合并并加文件锁，保留未知字段和其他 MCP；托管条目限定在 `xiaobai_` 命名空间，且只清理该命名空间内已不在当前清单里的条目。
- 同步沿用现有数据库备份协议与 manifest，不改变既有文件名、目录名、备份前缀或密钥。

## 各客户端落点
| 目标 | 文件 | 结构 |
|------|------|------|
| Claude Code | `~/.claude.json`（`CLAUDE_CONFIG_DIR` 生效时为其目录内同名文件） | 顶层 `mcpServers` |
| Codex | `~/.codex/config.toml` | `[mcp_servers.*]`，headers 映射到 `http_headers` |
| Pi | `<pi agent dir>/mcp.json` | `mcpServers` |
| Prime | `<prime agent dir>/settings.json` | `mcpServers` |

## 验收标准（已核对）
- [x] 可创建、编辑、启用/禁用、删除 MCP，并选择任意目标组合。
- [x] 应用到四个目标后写入各自原生位置；已有非托管内容保持不变（有测试覆盖，含 Codex 注释与 inline table）。
- [x] 应用先备份再原子写入；不可解析的既有配置报错且不覆盖原文件；并发写入由 `.lock` 串行化。
- [x] 改名、禁用、删除、改绑目标后，旧客户端里的托管条目会被清理；写失败的目标会保留以便重试。
- [x] WebDAV/备份同步覆盖 MCP（`mcp_servers` 参与内容指纹，有回归测试）；旧备份仍可恢复。
- [x] Rust 覆盖迁移升级路径、加密、四适配器合并/清理、锁定、命令层清理与路径解析；前端覆盖表单校验与应用流程。
- [x] `cargo test`（397 passed）、`pnpm typecheck`、`pnpm test:run`（310 passed；2 个 updater 脚本用例为既有 shebang 解析问题，与本次改动无关）通过。

## 明确不做
不实现 MCP 服务运行时、工具调用代理、远端市场/自动发现，也不改变既有站点同步协议和 Agent 的非 MCP 配置语义。
