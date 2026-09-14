# 技术设计

## 关键决策

### 1. 扫描只读、只回元数据，密钥值不出后端
- 扫描函数只读文件、不写。
- 返回值**不含** `env`/`headers` 的值，只含其**键名**（例：「需要 `EXA_API_KEY`」）。
- `command`/`args`/`url` 原样返回：这是识别一个条目是什么的唯一依据（用户看到
  `npx -y exa-mcp-server` 才知道那是 exa），且内容本就来自用户自己可读的配置文件。
  明确记录这条边界：受保护的「密钥值」指 `env`/`headers` 的值。

### 2. 纳管时**按 (target, key) 重新读盘**，不接收前端传来的配置
`import_scanned_mcp` 的参数只是 `[(target, key)]` 这样的定位符，后端据此重新读取文件、
解析、加密入库。好处：
- 密钥值从不经过前端；
- 前端无法伪造/篡改要写入数据库的内容；
- 扫描与纳管之间文件有变化时，以纳管时刻的真实内容为准。

### 3. 用「内容比对」代替存快照，避免新增表/列
接管判断不依赖额外存储：拿**库里那条记录的完整内容**（`config` 明文 + 解密后的
`env`/`headers`）与客户端里那条未托管条目的内容做规范化指纹比较。
- 指纹 = 对规范化后的内容做 sha256 十六进制（沿用 `crypto::key_fingerprint` 的既有做法：
  只存哈希，不存明文）。
- 两者一致 → 是同一条目 → 替换（删除未托管条目 + 写入 `xiaobai_<name>`）。
- 两者不一致 → 用户事后改过 → **该目标整体跳过**，报明确错误列出冲突条目名，不写任何东西。
- 因此**不需要** schema 变更，也**不需要**在 `sync_meta` 里存快照。

选择按目标整体跳过而非单条跳过，是为了与既有「目标级原子性」一致：一个目标要么整体写成功，
要么整体不写，不留半完成状态。

### 4. 复用既有写入路径与托管约定
不新增写入路径。接管逻辑加在现有四个 `apply_to_*` 里，作为「清理阶段」的补充：
原来的清理只扫 `xiaobai_` 前缀，现在多一步「先判断同名未托管条目是否等价」。

## 数据流

```
扫描：commands::scan_existing_mcp
  → adapters::mcp::scan_{claude,codex,pi,prime}（只读解析）
  → 命令层与 DB 比对，标注 managed / already_imported
  → 前端列表（无密钥值）

纳管：commands::import_scanned_mcp([(target, key)])
  → 重新读盘解析该条目
  → repo::mcp::save（env/headers 走既有加密）
  → 返回纳管结果

应用：既有 apply_mcp_servers
  → 对每个目标：清理 xiaobai_ 旧条目 + 接管等价的未托管条目 + 写入托管条目
```

## 各客户端的读取要点
| 目标 | 读取位置 | 注意 |
|------|----------|------|
| Claude Code | `~/.claude.json` 顶层 `mcpServers` | 同写入路径（`claude_mcp_json_path`） |
| Codex | `~/.codex/config.toml` 的 `[mcp_servers.*]` | **必须处理 `[mcp_servers.<name>.env]` 嵌套子表**（本机实测形状）；用 `toml_edit` 遍历，不解析成强类型以免丢字段 |
| Pi | `<pi agent dir>/mcp.json` | `mcpServers` |
| Prime | `<prime agent dir>/settings.json` | `mcpServers` |

读取时对形状不合法的文件**报错但不修改**，与写入路径的既有约定一致。

## 归一化与冲突
- 客户端键名去掉 `xiaobai_` 前缀后，按既有规则校验；已是合法名则原样保留。
- 库里已有同名记录 → 扫描结果标 `already_imported`，界面不提供重复纳管。
- 纳管时重名 → 复用 `repo::mcp::save` 的既有报错（`another MCP server is already named`）。

## 安全与兼容
- 不记录任何密钥值到日志 / 错误信息；错误只提键名与条目名。
- 不改变 `xiaobai_` 托管约定、同步协议、既有命令签名；`mcp_servers` 表结构不变。
- 测试夹具一律用明显占位串，不出现可用凭据字面量。

## 分阶段
1. 后端读取层：四个 `scan_*` + 规范化 + 指纹计算 + 单元测试（含 Codex 嵌套 env）。
2. 命令层：`scan_existing_mcp` / `import_scanned_mcp` + 与 DB 的比对标注。
3. 接管逻辑：加进四个 `apply_to_*`，含冲突跳过与目标级报错 + 回归测试。
4. 前端：扫描结果列表（区分三类）、单个/批量纳管、i18n、browser mock。
5. 验证：真实配置扫描（只读）、纳管后应用确认不产生重复、既有测试不回归。
