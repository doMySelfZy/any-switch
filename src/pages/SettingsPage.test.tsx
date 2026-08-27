import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { App as AntdApp, ConfigProvider } from "antd";
import type { ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { resetBrowserMock } from "@/lib/browserMock";
import {
  GITHUB_ISSUES_URL,
  GITHUB_RELEASES_URL,
  GITHUB_REPO_URL,
} from "@/lib/constants";
import { useSettingsStore, useUIStore } from "@/stores";
import { resetUpdateCheckerForTests } from "@/hooks/useUpdateChecker";
import { SettingsPage } from "./SettingsPage";
import "@/i18n";

function Wrapper({ children }: { children: ReactNode }) {
  return (
    <ConfigProvider>
      <AntdApp>{children}</AntdApp>
    </ConfigProvider>
  );
}

const pkg = JSON.parse(readFileSync(resolve(import.meta.dirname, "../../package.json"), "utf8")) as {
  version: string;
};

describe("SettingsPage network", () => {
  beforeEach(() => {
    resetBrowserMock();
    useUIStore.setState({ settingsTab: "network" });
    useSettingsStore.setState({
      settings: { ...useSettingsStore.getState().settings, proxyMode: "system" },
      loaded: false,
      loading: false,
    });
  });

  afterEach(() => {
    resetBrowserMock();
    useUIStore.setState({ settingsTab: "general" });
  });

  it("shows proxy modes and custom fields only when custom is selected", async () => {
    render(
      <Wrapper>
        <SettingsPage />
      </Wrapper>,
    );

    await waitFor(() => {
      expect(screen.getByText("代理模式")).toBeInTheDocument();
    });
    expect(screen.getByText("系统代理")).toBeInTheDocument();
    expect(screen.queryByText("主机 / IP")).toBeNull();

    fireEvent.mouseDown(screen.getByRole("combobox"));
    fireEvent.click(await screen.findByTitle("自定义"));

    await waitFor(() => {
      expect(screen.getByText("主机 / IP")).toBeInTheDocument();
      expect(screen.getByText("端口")).toBeInTheDocument();
    });
    expect(useSettingsStore.getState().settings.proxyMode).toBe("custom");
    expect(screen.getByText("测速结果有效期")).toBeInTheDocument();
    expect(screen.queryByText("设置已保存")).toBeNull();
  });
});

describe("SettingsPage tray", () => {
  beforeEach(() => {
    resetBrowserMock();
    useUIStore.setState({ settingsTab: "general" });
    useSettingsStore.setState({
      settings: useSettingsStore.getState().settings,
      loaded: false,
      loading: false,
    });
  });

  afterEach(() => {
    resetBrowserMock();
    useUIStore.setState({ settingsTab: "general" });
  });

  it("shows close-to-tray and disables start-in-tray when close-to-tray is off", async () => {
    render(
      <Wrapper>
        <SettingsPage />
      </Wrapper>,
    );

    await waitFor(() => {
      expect(screen.getByText("关闭窗口时最小化到托盘")).toBeInTheDocument();
    });
    expect(useSettingsStore.getState().settings.closeToTray).toBe(true);

    const switches = screen.getAllByRole("switch");
    const closeToTray = switches[2];
    const startInTray = switches[3];
    expect(startInTray).not.toBeDisabled();

    fireEvent.click(closeToTray);
    await waitFor(() => {
      expect(useSettingsStore.getState().settings.closeToTray).toBe(false);
    });
    expect(useSettingsStore.getState().settings.startInTray).toBe(false);
    expect(startInTray).toBeDisabled();
  });

  it("persists launch-at-login when the auto-start switch is clicked", async () => {
    render(
      <Wrapper>
        <SettingsPage />
      </Wrapper>,
    );

    await waitFor(() => {
      expect(screen.getByText("开机启动")).toBeInTheDocument();
    });
    expect(useSettingsStore.getState().settings.autoStart).toBe(false);

    fireEvent.click(screen.getAllByRole("switch")[1]);
    await waitFor(() => {
      expect(useSettingsStore.getState().settings.autoStart).toBe(true);
    });
  });
});

describe("SettingsPage about", () => {
  beforeEach(() => {
    resetBrowserMock();
    resetUpdateCheckerForTests();
    useUIStore.setState({ settingsTab: "about" });
    useSettingsStore.setState({
      settings: useSettingsStore.getState().settings,
      loaded: false,
      loading: false,
    });
  });

  afterEach(() => {
    resetBrowserMock();
    useUIStore.setState({ settingsTab: "general" });
  });

  it("shows the actual package version instead of a hardcoded placeholder", async () => {
    render(
      <Wrapper>
        <SettingsPage />
      </Wrapper>,
    );

    await waitFor(() => {
      expect(useSettingsStore.getState().loaded).toBe(true);
      expect(screen.getByText(pkg.version)).toBeInTheDocument();
      expect(screen.getByText("~/.xiaobai-switch")).toBeInTheDocument();
    });
  });

  it("shows the app logo and GitHub link entries", async () => {
    const open = vi.spyOn(window, "open").mockImplementation(() => null);

    render(
      <Wrapper>
        <SettingsPage />
      </Wrapper>,
    );

    await waitFor(() => {
      expect(screen.getByRole("img", { name: "XiaoBaiSwitch" })).toBeInTheDocument();
    });
    expect(screen.getByRole("button", { name: /GitHub 仓库/ })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /问题反馈/ })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /版本发布/ })).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /GitHub 仓库/ }));
    expect(open).toHaveBeenCalledWith(GITHUB_REPO_URL, "_blank", "noopener,noreferrer");

    fireEvent.click(screen.getByRole("button", { name: /问题反馈/ }));
    expect(open).toHaveBeenCalledWith(GITHUB_ISSUES_URL, "_blank", "noopener,noreferrer");

    fireEvent.click(screen.getByRole("button", { name: /版本发布/ }));
    expect(open).toHaveBeenCalledWith(GITHUB_RELEASES_URL, "_blank", "noopener,noreferrer");

    open.mockRestore();
  });

  it("shows update check controls and persists auto-check", async () => {
    render(
      <Wrapper>
        <SettingsPage />
      </Wrapper>,
    );

    await waitFor(() => {
      expect(screen.getByText("自动检查更新")).toBeInTheDocument();
    });
    expect(screen.getByRole("button", { name: "检查更新" })).toBeInTheDocument();
    expect(useSettingsStore.getState().settings.autoCheckUpdate).toBe(true);

    fireEvent.click(screen.getByRole("switch"));
    await waitFor(() => {
      expect(useSettingsStore.getState().settings.autoCheckUpdate).toBe(false);
    });
  });

  it("tells the user a manual check needs the desktop app", async () => {
    render(
      <Wrapper>
        <SettingsPage />
      </Wrapper>,
    );

    await waitFor(() => {
      expect(screen.getByRole("button", { name: "检查更新" })).toBeInTheDocument();
    });
    fireEvent.click(screen.getByRole("button", { name: "检查更新" }));
    await waitFor(() => {
      expect(screen.getByText("请在桌面客户端中检查更新")).toBeInTheDocument();
    });
  });
});

describe("SettingsPage paths", () => {
  beforeEach(() => {
    resetBrowserMock();
    useUIStore.setState({ settingsTab: "paths" });
    useSettingsStore.setState({ loaded: false, loading: false });
  });

  afterEach(() => {
    resetBrowserMock();
    useUIStore.setState({ settingsTab: "general" });
  });

  it("shows and persists the Pi Agent directory override", async () => {
    render(
      <Wrapper>
        <SettingsPage />
      </Wrapper>,
    );

    const input = await screen.findByPlaceholderText(
      "留空按 PI_CODING_AGENT_DIR → ~/.pi/agent 解析",
    );
    fireEvent.change(input, { target: { value: "/tmp/custom-pi" } });
    fireEvent.click(screen.getByRole("button", { name: "保存设置" }));
    await waitFor(() => {
      expect(useSettingsStore.getState().settings.piAgentDirOverride).toBe("/tmp/custom-pi");
    });
  });
});
