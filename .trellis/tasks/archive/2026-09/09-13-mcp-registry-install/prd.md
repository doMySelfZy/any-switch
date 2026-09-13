# MCP 官方仓库搜索安装与表单简化

## 目标
让用户不必理解 `command`/`args`/`env`/`headers` 就能装上 MCP：搜索官方 MCP Registry，选中后一键安装并自动回填；手动添加保留但弱化。

## 结论：只接官方源（决策记录）
调研了 Glama / Smithery / PulseMCP / mcp.so 后确认**只有官方源可程序化接入**：其余为网页站（无稳定公开 API），Smithery 需注册 key。官方源免鉴权、接口稳定、安装信息完整，且治理方可信（MCP 官方 Registry 工作组，成员来自 Anthropic / GitHub / PulseMCP / Microsoft）。

风险已知：官方明示处于 preview 阶段，可能有行为变更或数据重置。为此对解析做了防御（见下）。

## 需求

### 搜索与安装
1. 后端命令 `search_mcp_registry(query, cursor, local_only)`：搜索官方源并返回归一化候选（含安装草稿）。
2. 条目换算成应用内字段：`packages` 型 → `command`+`args`（顺序为「运行时参数 → 包名 → 包参数」）；`remotes` 型 → `url` + transport 类型。
3. 识别必填字段（`isRequired` 且无 `default`），标注是否敏感（`isSecret`）与填到 env 还是 header。
4. 前端搜索列表可选中安装，表单自动回填，用户只需补必填项。
5. 保存复用既有 `save_mcp_server`，不新增写入路径。

### 「不要挂别人机子」的落实
6. 搜索默认带 `version=latest`（实测不带会让同一服务按历史版本重复返回，100 条只对应 20 个服务）。
7. 结果按「本地运行优先」排序；`local_only` 开关默认开启，只返回 npx/uvx 这类跑在用户自己机器上的条目。
8. 远程条目如实标注**具体域名**（如「请求将发送到 server.smithery.ai」），不含糊地说「远程服务」。

### 表单简化与手动添加弱化
9. 表单分两层：默认只问名称、类型、启动命令或服务地址、以及仓库声明的必填密钥；`env`/`headers`/额外字段收进「高级配置」折叠区。
10. 主入口为「从官方仓库安装」，手动添加降为次级按钮并附提示。

## 验收标准（已核对）
- [x] 搜索官方源返回唯一结果、本地优先排序；`local_only` 过滤生效。
- [x] packages 型条目生成正确的 `command`+`args`；remotes 型生成 `url`。
- [x] 必填项缺失时阻止保存并明确提示；填完保存后值落到 env/header。
- [x] 远程条目显示具体域名；本地条目显示「将运行：<命令>」。
- [x] 名称归一化（`io.github.x/y` → 合法键）有测试覆盖；非法/空名回退。
- [x] 网络失败有可读错误，手动添加路径不受影响。
- [x] Rust 21 个 registry 单测 + 前端 14 个页面测试；`cargo test` 420 绿、`pnpm test:run` 320 绿（2 个 updater 用例为既有 shebang 问题）。
- [x] 实测官方源真实响应验证解析（见下）。

## 明确不做
- 不内置精选预设清单。
- 不接入第三方聚合市场（无稳定 API）。
- 不替用户安装 npm 包（只写配置）。
- 不改既有 MCP 写入路径、`xiaobai_` 托管约定与同步协议，不升 schema（草稿不落库）。

## 实测记录
- `search=github&limit=20&version=latest`：20 条全部唯一，其中 4 条可本地运行。
- 不带 `version=latest`：100 条仅 20 个唯一服务（重复是历史版本）——已修并加回归。
- 远程托管方分布抽样：`server.smithery.ai`、`a2awire.com`、个人 Cloudflare Workers 等，印证「默认只看本地」的必要性。
