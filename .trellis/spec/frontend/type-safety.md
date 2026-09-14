# Type Safety

> 前后端类型契约。跨语言边界的错误不会在编译期暴露，只能靠约定。

---

## 命令契约

`invoke` 用泛型标注返回类型，参数对象用后端要求的 camelCase 键名：

```tsx
const servers = await invoke<McpServerSummary[]>("list_mcp_servers");
const result = await invoke<McpSaveResult>("save_mcp_server", { input });
await invoke("set_floating_window_collapsed", { collapsed: next });
```

Rust 侧一律 `#[serde(rename_all = "camelCase")]`，前端类型逐字对应。
**字段名写错不会报错**（serde 忽略未知字段），表现为「传了但没生效」——
改这类字段时前后端一起改。

## 类型放哪

- `src/types/domain.ts`：与后端 `domain/mod.rs` 对应的核心模型
- `src/types/<域>.ts`：按域拆分（如 `types/mcp.ts`）
- 类型只在测试里用时也放同一处，不要就地定义

## 少用 as 断言

`as` 会绕过检查，字段改名后不会报错。优先：

- **类型守卫**：`isAppError(e): e is AppError`
- **可选链 + 兜底**：`settings.floatingWindow?.autoRefreshMinutes ?? 5`
- 确实无法收窄时才用 `as`，并写一句为什么

## 联合类型

领域枚举（目标客户端、MCP 类型等）用字符串联合而不是裸 `string`，并在切换分支用
穷尽映射表，新增取值时能一眼看出哪里要补：

```tsx
const TARGET_LABEL_KEYS: Record<TargetKind, string> = {
  claude_code: "mcp.targetClaudeCode",
  codex: "mcp.targetCodex",
  pi: "mcp.targetPi",
  prime: "mcp.targetPrime",
};
```

## 可选字段与「缺省即兼容」

后端新增字段一律带 `#[serde(default)]`，前端类型标可选并在读取处给默认值——
老数据库/老备份反序列化时不会炸。

## 与 Rust 的对应关系

| Rust | TypeScript |
|------|-----------|
| `Option<T>` | `T \| null`（serde 输出 `null`）或 `T \| undefined`（字段可能不存在） |
| `Vec<T>` | `T[]` |
| `i64`（时间戳毫秒） | `number` |
| `serde_json::Value` | `Record<string, unknown>` |
| `#[serde(rename_all = "camelCase")]` | 驼峰字段名 |

## 常见错误

- 在 store 外用 `as` 把 `unknown` 直接转成具体类型——错误形状会在运行时才炸。
- 类型写了可选但代码按必填用（`settings.floatingWindow.xxx`）→ 编译期不报，
  运行时空指针。加 `?.`。
