# Directory Structure

> 文件放哪儿。新增代码前先照这里对位置。

---

## 前端（`src/`）

| 目录 | 放什么 | 例子 |
|------|--------|------|
| `pages/` | 顶层页面，一个页面一个文件；页面级测试同目录同名 `.test.tsx` | `McpPage.tsx`、`SitesPage.tsx` |
| `components/<域>/` | 按业务域分组的组件 | `components/apply/ClaudeApplyPanel.tsx`、`components/settings/SettingsGroup.tsx` |
| `components/layout/` | 应用外壳（侧栏、标题栏） | `SideNav.tsx` |
| `stores/` | zustand store，一个领域一个文件，统一从 `stores/index.ts` 导出 | `mcpStore.ts`、`siteStore.ts` |
| `types/` | 与后端契约对应的类型 | `types/domain.ts`、`types/mcp.ts` |
| `lib/` | 无 UI 的工具与适配层 | `invoke.ts`、`browserMock.ts`、`quotaProbe.ts` |
| `i18n/locales/` | 文案，`zh-CN.json` 与 `en-US.json` **必须成对更新** | — |
| `hooks/` | 跨组件复用的 hook | `useResolvedDarkMode.ts` |
| `test/` | 测试环境初始化（`setup.ts`） | — |

路径别名 `@/` → `src/`，一律用别名，不用 `../../` 上溯。

---

## 后端（`src-tauri/src/`）

| 目录 | 放什么 |
|------|--------|
| `commands/` | Tauri 命令，UI→宿主的唯一边界；按领域分文件 |
| `adapters/` | 改写目标 CLI 配置的适配器（claude_code / codex / pi / prime / mcp） |
| `repo/` | 数据库读写 |
| `db/migrate.rs` | schema 定义与迁移 |
| `domain/` | 领域模型（命令出入参类型） |
| 顶层 `*.rs` | 单职责模块（`sync.rs`、`webdav.rs`、`paths.rs`、`floating_window.rs`…） |

### 新增一个命令要动的三处

1. `commands/<域>.rs` 写 `#[tauri::command]`
2. `lib.rs` 的 `generate_handler!` 里注册——**漏注册的表现是前端报「命令不存在」**
3. 前端 `lib/browserMock.ts` 补同名 mock 分支（非 Tauri 开发与单元测试都依赖它）

### 新增业务表

除 schema 外**必须**把表名加进 `sync.rs` 的 `FINGERPRINT_TABLES`，否则该表的数据变更
不会被判定为「数据变化」，跨设备同步永远不发。`mcp_servers` 踩过这个坑。

---

## 常见错误

- 组件因为「只在一个页面用」就塞进 `pages/`——可复用或可单测的都应进 `components/`。
- 加了 i18n key 只改 `zh-CN.json`——英文界面会回退显示 key 原文。
- 在 `adapters/` 里写 `#[tauri::command]`——那一层不做 UI 边界。
