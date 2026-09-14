import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { App as AntdApp, ConfigProvider } from "antd";
import type { ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { resetBrowserMock } from "@/lib/browserMock";
import { FloatingWindow } from "./FloatingWindow";
import "@/i18n";

// 悬浮窗要操作真实窗口对象；jsdom 里没有，这里替换掉。
const setSize = vi.fn().mockResolvedValue(undefined);
const hide = vi.fn().mockResolvedValue(undefined);

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({
    setSize,
    setPosition: vi.fn().mockResolvedValue(undefined),
    outerPosition: vi.fn().mockResolvedValue({ x: 100, y: 100 }),
    hide,
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
    hide.mockClear();
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

    // 正常余额
    expect(await screen.findByText("Relay A")).toBeInTheDocument();
    expect(screen.getByText("$42.50")).toBeInTheDocument();
    // 低余额仍显示金额（警示色由样式承担，这里只断言不隐藏）
    expect(screen.getByText("$1.25")).toBeInTheDocument();
    // 无限额
    expect(screen.getByText("无限")).toBeInTheDocument();
    // 未知
    expect(screen.getByText("不可用")).toBeInTheDocument();
    expect(screen.getByText("Unknown D")).toBeInTheDocument();
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

  it("collapses to just the title bar and persists the state", async () => {
    const invoke = await invokeMock();
    render(
      <Wrapper>
        <FloatingWindow />
      </Wrapper>,
    );

    expect(await screen.findByText("Relay A")).toBeInTheDocument();

    // 收起：内容区被标记为隐藏（过渡需要它留在 DOM 里），窗口高度收掉、状态写库。
    fireEvent.click(screen.getByRole("button", { name: /收\s*起/ }));

    await waitFor(() => {
      expect(screen.getByTestId("floating-content")).toHaveAttribute("aria-hidden", "true");
    });
    expect(screen.getByText("站点余额")).toBeInTheDocument();
    await waitFor(() => {
      expect(setSize).toHaveBeenCalled();
    });
    expect(invoke).toHaveBeenCalledWith("set_floating_window_collapsed", { collapsed: true });
  });

  it("expands back to the full list", async () => {
    render(
      <Wrapper>
        <FloatingWindow />
      </Wrapper>,
    );

    expect(await screen.findByText("Relay A")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: /收\s*起/ }));
    await waitFor(() => {
      expect(screen.getByTestId("floating-content")).toHaveAttribute("aria-hidden", "true");
    });

    fireEvent.click(screen.getByRole("button", { name: /展\s*开/ }));
    await waitFor(() => {
      expect(screen.getByTestId("floating-content")).toHaveAttribute("aria-hidden", "false");
    });
    expect(screen.getByText("Relay A")).toBeInTheDocument();
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

  it("hides the window instead of quitting on close", async () => {
    render(
      <Wrapper>
        <FloatingWindow />
      </Wrapper>,
    );

    await waitFor(() => {
      expect(screen.getByText("Relay A")).toBeInTheDocument();
    });
    fireEvent.click(screen.getByRole("button", { name: /关\s*闭/ }));
    expect(hide).toHaveBeenCalled();
  });
});
