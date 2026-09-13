# 技术设计

## 关键决策

### 1. 仓库访问走后端，不走前端
与技能市场（`search_skill_marketplace`）保持一致：前端不直接发外部请求，统一经 `AppState` 的 HTTP 客户端（`http_client::build_client`），这样代理设置、超时、TLS 行为与全应用一致，也能统一做错误归一化与大小上限。

固定出站域名：`registry.modelcontextprotocol.io`（仅 https）。不做用户可控的仓库地址，避免变成 SSRF 面。

### 2. 归一化在 Rust 侧完成，前端只渲染
仓库条目结构（`packages`/`remotes` 两套形状、参数是带 `type` 的数组）不适合直接塞给前端拼装。后端新增一个纯函数把 `server` 条目转成「安装草稿」：

```
RegistryInstallDraft {
  name: String,              // 归一化后的托管名
  display_name: String,      // 仓库原名，仅展示
  kind: McpKind,             // stdio | http | sse
  config: Value,             // 直接可写入的 command/args 或 url
  env: Value,                // 已知默认值预填
  headers: Value,
  required_env: Vec<RegistryField>,    // 必填且无默认值
  required_headers: Vec<RegistryField>,
}
```

这样前端不重复实现一遍解析规则；未来仓库字段变化只改一处。

### 3. 手动添加保留为兜底，不删除
现有 `save_mcp_server` 与表单写入路径完全复用。仓库只是「多一个入口帮用户把字段填好」，不是唯一路径——仓库不可达、私有 MCP、内网服务都必须能手动配。

### 4. 名称归一化规则
仓库名形如 `io.github.upstash/context7-mcp` 不是合法 MCP 键（现有校验 `[A-Za-z0-9_-]+`）。规则：
1. 取 `/` 之后的最后一段作为主体（`context7-mcp`）；无 `/` 时用整体。
2. 非 `[A-Za-z0-9_-]` 字符替换为 `-`，合并连续 `-`，去首尾 `-`。
3. 结果为空则回退为 `mcp`。
4. 保存冲突时沿用现有「another MCP server is already named」报错，由用户改名——不自动加序号（避免用户看不懂名字从哪来）。

## 数据流
搜索页 → `search_mcp_registry(query, cursor?)` → HTTP → 解析归一化 → 前端列表 → 选中 → `resolve_mcp_registry_entry(name)`（或直接复用搜索结果）→ 预填表单 → 用户补必填 → `save_mcp_server` → 既有应用流程写入四客户端。

## 兼容与安全
- 不新增目标写入路径，不改 `xiaobai_` 托管约定、同步协议与既有命令签名。
- 仓库是第三方 metadata，UI 需标注来源仓库链接，不暗示安全背书；秘钥字段只在应用数据库加密存储，沿用既有能力。
- 搜索请求带上限与超时；响应体大小设上限，避免异常响应拖垮界面。
- 不改动 `mcp_servers` 表结构（草稿不落库，仅作为表单初值），因此无需 schema 升级。

## 分阶段
1. 后端：仓库客户端 + 条目解析/归一化纯函数 + 两个命令 + 单测。
2. 前端：搜索界面 + 自动回填 + 表单分层 + 手动入口弱化。
3. 联调与验证：真实仓库搜索、安装到四客户端、必填校验、错误分支。
