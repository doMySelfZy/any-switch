# Frontend Development Guidelines

> 本项目的实际前端约定。**目标是可直接照着写代码**，不是原则宣讲。

---

## 权威源与本文档的关系

`AGENTS.md`（仓库根）是**约束的唯一权威源**——产品规则、兼容红线、UI Shell、
Ant Design 约定、i18n 规则都在那里，改动必须以它为准。

本目录是按层提炼的**操作性索引**：写组件时看
[component-guidelines](./component-guidelines.md)，改状态看
[state-management](./state-management.md)，以此类推。两处冲突时以 `AGENTS.md` 为准。

---

## Guidelines Index

| Guide | Description | Status |
|-------|-------------|--------|
| [Directory Structure](./directory-structure.md) | 前后端文件放哪儿 | ✅ |
| [Component Guidelines](./component-guidelines.md) | 组件、弹窗、图标、可访问性 | ✅ |
| [Hook Guidelines](./hook-guidelines.md) | effect、定时器、清理、TDZ | ✅ |
| [State Management](./state-management.md) | zustand store 与后端调用边界 | ✅ |
| [Quality Guidelines](./quality-guidelines.md) | i18n、测试、断言质量 | ✅ |
| [Type Safety](./type-safety.md) | 前后端类型契约 | ✅ |

---

## 快速上手（最常踩的三条）

1. **文案一律走 i18n**：`t("settings.xxx")`，插值必须写 `{{count}}`——写成 `{count}`
   会把占位符原样显示到界面上。
2. **弹窗内的 message/modal 必须从 `App.useApp()` 取**，不要从 `antd` 静态导入：
   静态调用会丢主题与上下文。
3. **图标按钮要 `aria-label`**：否则读屏不可用，测试里
   `getByRole("button", { name })` 也查不到。

**Language**: 文档用中文写；代码标识符、路径、命令保持原文。
