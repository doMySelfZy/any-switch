# AnySwitch — Agent / 自动化约束

本文档约束人类与编码 Agent 如何改动本仓库。

## 产品规则

- 产品名：**AnySwitch**（`com.domyselfzy.any-switch.app`）
- 领域单一事实来源（SSOT）是 **站点优先**：Base URL + API key → 模型 → 目标私有能力预设 → 应用到目标
- 目标：Claude Code（`~/.claude/settings.json`）、Codex（`~/.codex/` + 环境变量注入）、Pi（`~/.pi/agent/{models,auth,settings}.json`）与 Prime（`~/.prime/agent/{models,auth,settings}.json`）
- 应用数据根目录：`~/.any-switch/`（不是 Tauri 的 `app_data_dir`）

## 目录布局

```
~/.any-switch/
├── any-switch.db       # SQLite 应用状态
├── master.key          # AES-256-GCM 主密钥（Unix 上权限 0600）
└── backups/            # 应用前备份
```

- **禁止**把用户数据路径写死成 Tauri `app_data_dir`、bundle id 版本字符串，或平台应用支持目录。
- **数据目录迁移**：启动时若 `~/.any-switch` 不存在而 `~/.xiaobai-switch` 存在，会复制迁移并校验数据库 sha256 / `master.key`；**旧目录必须保留**作为回滚点，旧目录名只在迁移识别中使用，不要删除或改名。数据目录覆盖变量优先 `ANY_SWITCH_DATA_DIR`，并兼容旧的 `XIAOBAI_SWITCH_DATA_DIR`。
- **兼容红线（改了会破坏既有数据，一律保持旧值，只做“新旧双识别”）**：
  - WebDAV 同步 manifest 文件名 `xiaobai-switch-sync.json`；
  - WebDAV 远端目录默认值 `xiaobai-switch`（与 manifest 是同一套跨机协议路径）；
  - 旧本地备份前缀 `xiaobai-switch-backup-`（列表/恢复/清理/远端保留都要接受新旧两种前缀，排序需先剥离前缀再按时间戳比较）；
  - 备份包内数据库条目名 `xiaobai-switch.db`（内部归档协议，保留才能旧备份可恢复 + 新备份可被旧版读取）；
  - 已应用配置命名空间 `xiaobai_`：Pi/Prime provider id、Codex provider id 派生、`XIAOBAI_SITE_*` 环境变量、`__xiaobai_missing__`；
  - 应用/备份痕迹识别文件名：`xiaobai-model-catalog.json`（Codex 模型目录）、`xiaobai-backup.json`（目标备份元数据）、`.xiaobai-skill.json` 与 `.xiaobai-skill-` 临时前缀（技能安装清单）、`atomic.rs` 的 `.xiaobai-` 临时文件前缀；
  - `restore_official` / 孤儿清理的识别逻辑不得改动。
- API key 在数据库中加密存储；**应用（Apply）** 之后，它们可能以明文出现在 `~/.claude` / `~/.codex` / `~/.pi/agent/auth.json` / `~/.prime/agent/auth.json` / `codex.env` / shell rc 中 —— 须在 UI 文案中说明这一点。

## UI Shell

### 标题栏

- 高度 **36px**（对齐 macOS 红绿灯）。
- 自定义标题栏：`titleBarStyle: Overlay` + `hiddenTitle: true`。
- 拖拽区域：
  - CSS：`.title-bar-drag` → `-webkit-app-region: drag`
  - 可交互子元素：`.title-bar-nodrag` + 按钮 / `.ant-dropdown-trigger` → `no-drag`
  - macOS：标题栏还需设置 `data-tauri-drag-region`
  - 始终在 mousedown 时调用 `getCurrentWindow().startDragging()`（回退方案；CSS 拖拽单独失效时必须有它）
- Windows：左侧内边距约 12；macOS：左侧内边距约 72，给红绿灯留位
- 设置开关放在标题栏；设置页打开时，显示关闭（XCircle）状态

### 主侧栏

- 宽度 **48px**；仅图标的圆形按钮 **36×36**，`borderRadius: 50%`
- 标签用 antd `Tooltip`，`placement="right"` —— **图标下方不要文字**
- 激活态：`token.colorPrimaryBg` + `token.colorPrimary`
- 进入设置页时整栏隐藏

### 设置页

- 左侧 **设置侧栏**（`w-56`）+ 右侧内容（`colorBgElevated`）
- 侧栏：返回行（ArrowLeft + Esc 提示）+ antd `Menu` `mode="inline"`，带分区图标
- 内容：用 `SettingsGroup` 分组（Card、柔和边框、分区标题在卡片上方）
- Esc 退出设置，回到主页面
- 在 UX 允许时，优先在开关/选择时即时 `saveSettings`，而不是一个统一的批量保存表单

### 应用中心

- 与设置页同一套壳：左侧 **目标侧栏**（`w-56`）+ 右侧内容（`colorBgElevated`）
- 当前目标为 **Claude Code**、**Codex**、**Pi** 与 **Prime**；Claude/Codex/Pi 图标来自 `@lobehub/icons`，Prime 使用 lucide `Sparkles`（`@lobehub/icons` 无 Prime 图标）
- 每个目标有各自的专用表单（不是共享的双目标复选框面板）
- Claude 关键字段：鉴权 key 风格、默认模型、opus/sonnet/haiku 别名映射、effort 等级
- Codex 关键字段：默认模型、写入全部模型目录开关、reasoning effort；平台能力默认跟随站点 `codex-compact` / `codex-vision` / `codex-imagegen` / `codex-search`，也可在应用中心自定义覆盖
- Pi 关键字段：默认模型、写入站点全部模型开关、协议说明、逐模型思考能力与默认思考等级；思考预设按 `(site_id, target)` 记忆，不根据模型名称猜测
- Prime 关键字段：默认模型、写入站点全部模型开关、协议说明、逐模型思考能力与默认思考等级；配置目录默认 `~/.prime/agent`，不要写到 `~/.pi/agent`
- 站点编辑含默认收起的「高级配置」（连接协议、备注）与「Codex私有能力」；`anyswitch://sites` 用同一套 kebab 键导入预设
- 分区卡片复用 `SettingsGroup`，保持视觉语言一致

## Ant Design 约定

### ConfigProvider

始终用 antd `ConfigProvider` + `App`（`AntdApp`）包裹应用。全局 Modal 默认值：

```tsx
<ConfigProvider
  modal={{
    centered: true,
    styles: {
      mask: { backdropFilter: "blur(4px)" },
      container: {
        maxHeight: "calc(100vh - 32px)",
        display: "flex",
        flexDirection: "column",
        overflow: "hidden",
      },
      body: { overflowY: "auto", overflowX: "hidden", minHeight: 0 },
    },
  }}
>
  <AntdApp className="h-full">…</AntdApp>
</ConfigProvider>
```

### Message / Modal / notification

- **必须**使用 `App.useApp()` → `const { message, modal, notification } = App.useApp()`
- **禁止**：从 `antd` 静态调用 `message.success()`、`Modal.confirm()`（会破坏 `App` 下的主题 / 上下文）
- 确认框：`modal.confirm({ centered: true, … })`

### Modal 组件

每个表单/对话框 `Modal` 都应：

| 属性 | 值 |
|------|--------|
| `centered` | `true`（或依赖 ConfigProvider） |
| `destroyOnHidden` | `true`（antd 5.23+ / 6.x；优先于已弃用的 `destroyOnClose`） |
| `mask` | `{ enabled: true, blur: true }`（在支持时） |
| `width` | 表单优先 `520`–`560` |
| 高度 | 容器 `maxHeight: calc(100vh - 32px)`；标题 / 底栏固定，body 内部滚动 |

```tsx
<Modal
  open={open}
  centered
  destroyOnHidden
  mask={{ enabled: true, blur: true }}
  width={560}
  onCancel={onClose}
  onOk={handleOk}
>
  …
</Modal>
```

### Modal 内的表单

- 优先使用 antd `Form` + `Form.Item`，`layout="vertical"`
- 必填字段用 `rules={[{ required: true }]}` —— 不要自造校验 UI
- 密码字段：`Input.Password`
- 自由文本输入在安全的情况下使用 `allowClear`

### 主题 Token

- 颜色优先用 `theme.useToken()`，不要写死 hex（品牌色 / Windows 关闭按钮红 `#e81123` 除外）
- 将常用 token 同步到 `documentElement` 上的 CSS 变量（`--border-color`、`--color-bg-*`、`--color-text*`、`--color-primary`）

### 图标

- Shell / 导航使用 Lucide 图标；标题栏尺寸 14，侧栏 16–18
- CSS：`svg.lucide { display: inline-block; vertical-align: -0.125em; }`，以便与 antd 对齐

## 国际化（i18n）

- 所有用户可见文案走 `react-i18next`（`zh-CN` + `en-US`）
- 组件中不要硬编码中文/英文，技术标识除外（模型 id、环境变量名、路径）

## 安全 / 脱敏

- 永远不要记录原始 API key；任何可能回显密钥的日志或错误展示前，先走脱敏辅助函数
- 站点列表与详情展示密钥材料时只用前缀（`keyPrefix`）
- 编辑站点时可通过 `get_site_api_key` 按需解密到密码输入框；默认隐藏，禁止写入日志或长期前端状态

## 前端技术栈

- Tauri 2 + React + TypeScript + Vite + antd + Tailwind v4 + zustand + lucide-react
- 路径别名 `@/` → `src/`
- 非 Tauri 开发用 `src/lib/browserMock.ts` 作为浏览器 mock 层；invoke 契约须与 Rust command 保持同步

## 后端（Rust）

- Command 是唯一的 UI→宿主边界；保持 `#[tauri::command]` 表面小且有类型
- 改写目标 CLI 配置前，先原子写入 + 备份
- Codex 的 `wire_api` 保持为 `responses`；provider id 由 `site.id` 派生
- Pi 直接合并 `models.json`、`auth.json`、`settings.json`：保留注释、其他 Provider、OAuth 与未知字段；只管理 `xiaobai_` 命名空间
- Pi 配置目录按应用设置 → `PI_CODING_AGENT_DIR` → `~/.pi/agent` 解析；`auth.json` 权限为 `0600`
- Prime 直接合并 `~/.prime/agent/{models,auth,settings}.json`：规则与 Pi 相同，只管理 `xiaobai_` 命名空间
- Prime 配置目录按应用设置 → `PRIME_AGENT_CODING_AGENT_DIR` → `~/.prime/agent` 解析；`auth.json` 权限为 `0600`
- 环境注入矩阵（shell rc / user env / file_only）必须与设置保持一致

## 测试

- 前端：`pnpm test:run`、`pnpm typecheck`
- Rust：在 `src-tauri` 中执行 `cargo test`
- UI 或适配器改动后，未跑完相关检查，不得声称「已完成」
