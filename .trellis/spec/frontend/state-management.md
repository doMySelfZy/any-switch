# State Management

> zustand store 与后端调用的边界。

---

## 基本约定

- 一个领域一个 store：`src/stores/<域>Store.ts`，并从 `stores/index.ts` 统一导出。
- store 里封装 `invoke` 调用；**组件通常不直接 invoke**。
  例外：一次性的、与全局状态无关的查询（如页面打开时取一次路径列表）可以直接调。
- 组件用选择器订阅，避免整个 store 引起重渲染：

```tsx
const servers = useMcpStore((s) => s.servers);
const loadServers = useMcpStore((s) => s.loadServers);
```

- 测试里重置 store 用 `useXxxStore.setState({ ... })`（见 `McpPage.test.tsx`）。

## store 里做「写后回读」

写完数据后重新拉一次列表，保证 UI 与后端一致，而不是在前端手改数组：

```tsx
saveServer: async (input) => {
  const result = await invoke<McpSaveResult>("save_mcp_server", { input });
  const servers = await invoke<McpServerSummary[]>("list_mcp_servers");
  set({ servers });
  return result;
},
```

## 组件内部状态

纯 UI 状态（弹窗开关、当前编辑项、搜索词）用 `useState`，不要塞进 store。

## 跨窗口

**不同 webview 之间不共享 store**。悬浮窗是独立窗口，主窗口的 zustand 状态它读不到。
需要共享时：后端持久化 + Tauri 事件通知（见 `hook-guidelines.md`）。

## 与后端一致性的陷阱

前端字段名必须与 Rust 的 serde 命名**逐字对齐**（Rust 端统一
`#[serde(rename_all = "camelCase")]`）。写错名字不会报错——serde 忽略未知字段，
表现为「设置改了但没生效」。

真实案例：前端 `autoRefreshSeconds` vs 后端 `auto_refresh_minutes`，刷新间隔设置
静默失效了很久。**改这类字段时两边一起改，并跑一次集成验证**。

## 浏览器开发模式

`src/lib/browserMock.ts` 提供非 Tauri 环境的 mock：每个命令一个 `case`。
新增命令时同步补 mock，否则浏览器里会报「Unknown command in browser mock」。
mock 的行为要与真实 command 的契约一致（校验、返回形状），否则开发时看不出问题、
真机上才炸。
