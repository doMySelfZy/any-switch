import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { App as AntdApp, ConfigProvider } from "antd";
import type { ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { handleBrowserCommand, resetBrowserMock } from "@/lib/browserMock";
import { FloatingWindow, withAlpha } from "./FloatingWindow";
import "@/i18n";

describe("withAlpha", () => {
  it("applies the cap to opaque colors", () => {
    expect(withAlpha("#141414", 0.72)).toBe("rgba(20, 20, 20, 0.72)");
    expect(withAlpha("rgb(20, 20, 20)", 0.72)).toBe("rgba(20, 20, 20, 0.72)");
  });

  it("never raises an already-translucent color", () => {
    // 深色主题下 antd token 常是「很淡的 rgba」，放大透明度会把卡片变成亮白块
    // —— 这正是截图里看到的问题，用这条锁住。
    expect(withAlpha("rgba(255, 255, 255, 0.08)", 0.6)).toBe("rgba(255, 255, 255, 0.08)");
    expect(withAlpha("rgba(255, 255, 255, 0.9)", 0.6)).toBe("rgba(255, 255, 255, 0.6)");
  });

  it("passes through values it cannot parse", () => {
    expect(withAlpha("var(--x)", 0.5)).toBe("var(--x)");
  });
});

// 悬浮窗要操作真实窗口对象；jsdom 里没有，这里替换掉。
const setSize = vi.fn().mockResolvedValue(undefined);
const setPosition = vi.fn().mockResolvedValue(undefined);
const outerPosition = vi.fn().mockResolvedValue({ x: 100, y: 100 });

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({
    setSize,
    setPosition,
    outerPosition,
  }),
  LogicalPosition: class {
    constructor(
      public x: number,
      public y: number,
    ) {}
  },
  LogicalSize: class {
    constructor(
      public width: number,
      public height: number,
    ) {}
  },
  PhysicalPosition: class {
    constructor(
      public x: number,
      public y: number,
    ) {}
  },
}));

// 跨窗口事件：悬浮窗靠它跟随设置页的改动。
const listeners = new Map<string, () => void>();
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn((event: string, handler: () => void) => {
    listeners.set(event, handler);
    return Promise.resolve(() => listeners.delete(event));
  }),
  emit: vi.fn().mockResolvedValue(undefined),
}));

// 统计后端调用，用来验证自动刷新确实会发请求。
vi.mock("@/lib/invoke", async () => {
  const actual = await vi.importActual<typeof import("@/lib/invoke")>("@/lib/invoke");
  return {
    ...actual,
    invoke: vi.fn((cmd: string, args?: Record<string, unknown>) => actual.invoke(cmd, args)),
  };
});

function Wrapper({ children }: { children: ReactNode }) {
  return (
    <ConfigProvider theme={{ token: { motion: false } }}>
      <AntdApp>{children}</AntdApp>
    </ConfigProvider>
  );
}

async function invokeMock() {
  const mod = await import("@/lib/invoke");
  return mod.invoke as unknown as ReturnType<typeof vi.fn>;
}

describe("FloatingWindow", () => {
  beforeEach(() => {
    resetBrowserMock();
    setSize.mockClear();
    setPosition.mockClear();
    outerPosition.mockClear();
    outerPosition.mockResolvedValue({ x: 100, y: 100 });
    Object.defineProperty(window, "devicePixelRatio", { value: 1, configurable: true });
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("lists site balances in the four display shapes", async () => {
    render(
      <Wrapper>
        <FloatingWindow />
      </Wrapper>,
    );

    // 「不可用」在多个站点上都会出现，断言按站点所在行限定。
    const cell = (name: string) =>
      within(screen.getByText(name).parentElement as HTMLElement);

    expect(await screen.findByText("Relay A")).toBeInTheDocument();
    expect(cell("Relay A").getByText("$42.50")).toBeInTheDocument();
    // 低余额仍显示金额（警示色由样式承担）
    expect(cell("Relay B").getByText("$1.25")).toBeInTheDocument();
    // 无限额
    expect(cell("Unlimited C").getByText("无限")).toBeInTheDocument();
    // 未知
    expect(cell("Unknown D").getByText("不可用")).toBeInTheDocument();
  });

  it("shows the last update time once balances are loaded", async () => {
    render(
      <Wrapper>
        <FloatingWindow />
      </Wrapper>,
    );

    expect(await screen.findByText("Relay A")).toBeInTheDocument();
    expect(screen.getByText(/最后更新/)).toBeInTheDocument();
  });

  it("collapses into a small orb and persists the state", async () => {
    const invoke = await invokeMock();
    render(
      <Wrapper>
        <FloatingWindow />
      </Wrapper>,
    );

    expect(await screen.findByText("Relay A")).toBeInTheDocument();

    // 收起：整块面板消失，只剩一个可点的小球；尺寸收成球、状态写库。
    fireEvent.click(screen.getByRole("button", { name: /收\s*起/ }));

    await waitFor(() => {
      expect(screen.queryByText("Relay A")).toBeNull();
    });
    expect(screen.getByRole("button", { name: /展\s*开/ })).toBeInTheDocument();
    await waitFor(() => {
      expect(setSize).toHaveBeenCalled();
    });
    expect(invoke).toHaveBeenCalledWith("set_floating_window_collapsed", { collapsed: true });
  });

  it("expands back into the panel when the orb is clicked", async () => {
    render(
      <Wrapper>
        <FloatingWindow />
      </Wrapper>,
    );

    expect(await screen.findByText("Relay A")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: /收\s*起/ }));
    await waitFor(() => {
      expect(screen.queryByText("Relay A")).toBeNull();
    });

    fireEvent.click(screen.getByRole("button", { name: /展\s*开/ }));
    await waitFor(() => {
      expect(screen.getByText("Relay A")).toBeInTheDocument();
    });
  });

  it("drags the window with physical-pixel math", async () => {
    // 回归：之前把「物理位置 + CSS 位移」当逻辑坐标交给 setPosition，
    // 在高 DPI 下窗口会以缩放倍数乱飞，表现就是拖不动。这里锁住换算。
    render(
      <Wrapper>
        <FloatingWindow />
      </Wrapper>,
    );

    const header = await screen.findByTestId("floating-header");
    outerPosition.mockResolvedValue({ x: 1000, y: 500 });
    Object.defineProperty(window, "devicePixelRatio", { value: 2, configurable: true });

    fireEvent.mouseDown(header, { button: 0, clientX: 10, clientY: 10 });
    await waitFor(() => expect(outerPosition).toHaveBeenCalled());
    fireEvent.mouseMove(document, { clientX: 40, clientY: 30 });
    fireEvent.mouseUp(document);

    // CSS 位移 (30, 20) × dpr 2 = 物理 (60, 40)，叠加起点 (1000, 500)。
    await waitFor(() => {
      expect(setPosition).toHaveBeenCalledWith(expect.objectContaining({ x: 1060, y: 540 }));
    });
  });

  it("refreshes on the configured interval and stops after unmount", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    const invoke = await invokeMock();
    const { unmount } = render(
      <Wrapper>
        <FloatingWindow />
      </Wrapper>,
    );

    await waitFor(() => {
      expect(screen.getByText("Relay A")).toBeInTheDocument();
    });

    const countRefreshes = () =>
      invoke.mock.calls.filter((call) => call[0] === "refresh_sites_quota").length;
    const before = countRefreshes();

    // 默认 5 分钟；推进 5 分钟应当触发一次，推进不足不触发。
    await vi.advanceTimersByTimeAsync(4 * 60_000);
    expect(countRefreshes()).toBe(before);

    await vi.advanceTimersByTimeAsync(60_000);
    expect(countRefreshes()).toBeGreaterThan(before);

    // 卸载后定时器必须清掉，否则窗口关掉还在后台请求。
    const afterUnmount = countRefreshes();
    unmount();
    await vi.advanceTimersByTimeAsync(10 * 60_000);
    expect(countRefreshes()).toBe(afterUnmount);
  });

  it("performs a manual refresh from the refresh button", async () => {
    const invoke = await invokeMock();
    render(
      <Wrapper>
        <FloatingWindow />
      </Wrapper>,
    );

    await waitFor(() => {
      expect(screen.getByText("Relay A")).toBeInTheDocument();
    });
    const before = invoke.mock.calls.filter((c) => c[0] === "refresh_sites_quota").length;

    fireEvent.click(screen.getByRole("button", { name: /刷\s*新/ }));

    await waitFor(() => {
      const after = invoke.mock.calls.filter((c) => c[0] === "refresh_sites_quota").length;
      expect(after).toBeGreaterThan(before);
    });
  });

  it("follows a settings change without reopening the window", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    const invoke = await invokeMock();
    render(
      <Wrapper>
        <FloatingWindow />
      </Wrapper>,
    );

    await waitFor(() => {
      expect(screen.getByText("Relay A")).toBeInTheDocument();
    });
    const countRefreshes = () =>
      invoke.mock.calls.filter((call) => call[0] === "refresh_sites_quota").length;

    // 设置页把间隔改成 1 分钟并广播事件。
    await handleBrowserCommand("save_settings", {
      partial: {
        floatingWindow: {
          enabled: true,
          autoRefreshMinutes: 1,
          positionX: 100,
          positionY: 100,
          collapsed: false,
        },
      },
    });
    const handler = listeners.get("floating-settings-changed");
    expect(handler, "悬浮窗应当订阅设置变更事件").toBeTruthy();
    handler?.();

    // 等重读设置完成，再推进 1 分钟：应当已按新间隔触发刷新。
    await vi.advanceTimersByTimeAsync(0);
    const before = countRefreshes();
    await vi.advanceTimersByTimeAsync(60_000);
    expect(countRefreshes()).toBeGreaterThan(before);
  });

  it("closes the window and turns the feature off", async () => {
    const invoke = await invokeMock();
    render(
      <Wrapper>
        <FloatingWindow />
      </Wrapper>,
    );

    await waitFor(() => {
      expect(screen.getByText("Relay A")).toBeInTheDocument();
    });

    // 「关闭」应当真正关掉功能——只 hide 的话下次启动它又冒出来，用户会以为关不掉。
    fireEvent.click(screen.getByRole("button", { name: /关\s*闭/ }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("set_floating_window_enabled", { enabled: false });
    });
  });

  it("shows the reason when a site's balance could not be read", async () => {
    render(
      <Wrapper>
        <FloatingWindow />
      </Wrapper>,
    );

    // 失败站点显示「不可用」，并挂上可查看的原因。
    expect(await screen.findByText("Failed E")).toBeInTheDocument();
    const cell = within(screen.getByText("Failed E").parentElement as HTMLElement);
    expect(cell.getByText("不可用")).toBeInTheDocument();
  });

  it("keeps the glass background translucent", async () => {
    render(
      <Wrapper>
        <FloatingWindow />
      </Wrapper>,
    );

    await waitFor(() => {
      expect(screen.getByText("Relay A")).toBeInTheDocument();
    });

    // 背景必须带透明度且启用 backdrop-filter：任一丢了毛玻璃就看不见。
    // （窗口侧还需 Rust 建窗时 transparent(true)，那部分在 floating_window.rs 的测试里。）
    const root = screen.getByTestId("floating-panel");
    const style = root.style;
    expect(style.backdropFilter).toContain("blur(");
    const bg = style.background;
    const alpha = Number(/rgba\([^)]*,\s*([\d.]+)\)/.exec(bg)?.[1] ?? "1");
    expect(alpha).toBeGreaterThan(0);
    expect(alpha).toBeLessThan(1);
  });
});
