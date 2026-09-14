# Hook Guidelines

> effect、定时器、清理、闭包。这里的每条都是踩过的坑。

---

## 副作用必须可清理

定时器、事件监听、Tauri 事件订阅都要返回清理函数。**不清理的后果不是理论问题**：
悬浮窗关掉后仍在后台请求、监听器累积。

```tsx
useEffect(() => {
  const timer = window.setInterval(() => void refresh(false), intervalMs);
  return () => window.clearInterval(timer);
}, [intervalMs, refresh]);
```

异步订阅要防「已卸载才拿到句柄」——否则句柄永远退不掉，且卸载后再退订会报错：

```tsx
useEffect(() => {
  let unlisten: (() => void) | undefined;
  let disposed = false;
  void listen("some-event", handler).then((fn) => {
    if (disposed) fn();      // 已经卸载：立刻退订
    else unlisten = fn;
  });
  return () => { disposed = true; unlisten?.(); };
}, [handler]);
```

## 定义为 effect 依赖的函数用 useCallback

否则每次渲染都是新引用，effect 反复重跑（典型表现：定时器刚设就被清、出现「只在
某些交互后才开始刷新」这类怪现象）。

```tsx
const refresh = useCallback(async () => { ... }, [message, t]);
useEffect(() => { void refresh(); }, [refresh]);
```

## 纯函数放在组件外

**别在组件内把工具函数定义在使用点之后**——`useMemo` 在渲染期同步求值，会踩
TDZ（`Cannot access 'x' before initialization`）。放组件外最省事；需要 i18n 时把
`t` 当参数传入。

```tsx
// 组件外
function formatQuota(quota: SiteQuotaSummary["quota"], t: (k: string) => string) { ... }

// 组件内
const lowCount = useMemo(() => sites.filter((s) => isLowBalance(s.quota)).length, [sites]);
```

## 只跑一次的 effect

首屏加载用空依赖数组，并显式注释为什么只跑一次；ESLint 的 exhaustive-deps 用
局部 `eslint-disable-next-line` 放行（不要全局关规则）。

```tsx
// 只跑一次：后续变更由 xxx 事件驱动。
// eslint-disable-next-line react-hooks/exhaustive-deps
}, []);
```

## 拖拽这类高频交互

不要在 `mousemove` 里做重活（写库、发请求）。正确做法：移动过程只改本地/窗口位置，
**松手时落一次盘**。悬浮窗原先每帧 `save_floating_window_position`，直接打满数据库。

## 跨窗口状态

不同 webview 之间**不共享** React/zustand 内存。主窗口的 store 悬浮窗读不到，必须：
后端存一份（缓存/设置表）+ 用 Tauri 事件广播变更。参考
`floating-settings-changed` 的用法（设置页 emit，悬浮窗 listen 后重读设置）。

## 常见错误

- 依赖数组写 `[props]` 而不是 `[props.field]`——对象每次新建，effect 每次重跑。
- 在 effect 里直接 `async () => {}`——effect 不能返回 Promise，要包一层 `void (async () => {})()`。
- 清理函数里用未捕获的异步调用（`void close()`），失败静默——加 `.catch`。
