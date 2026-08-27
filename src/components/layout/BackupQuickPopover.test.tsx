import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { App as AntdApp, ConfigProvider } from "antd";
import type { ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { resetBrowserMock } from "@/lib/browserMock";
import { useUIStore } from "@/stores";
import { BackupQuickPopover } from "./BackupQuickPopover";
import "@/i18n";

function Wrapper({ children }: { children: ReactNode }) {
  return (
    <ConfigProvider theme={{ token: { motion: false } }}>
      <AntdApp>{children}</AntdApp>
    </ConfigProvider>
  );
}

describe("BackupQuickPopover", () => {
  beforeEach(() => {
    resetBrowserMock();
    useUIStore.setState({ activePage: "sites", settingsTab: "general" });
  });

  afterEach(() => {
    resetBrowserMock();
  });

  it("creates local snapshots and routes unconfigured WebDAV users to backup settings", async () => {
    render(
      <Wrapper>
        <BackupQuickPopover />
      </Wrapper>,
    );

    fireEvent.click(screen.getByRole("button", { name: "应用数据备份" }));
    expect(await screen.findByText("备份包含数据库和解密主密钥，请仅保存到可信位置")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /立即备份到 WebDAV/ })).toBeDisabled();

    fireEvent.click(screen.getByRole("button", { name: /立即创建本地备份/ }));
    expect(await screen.findByText("本地应用备份已创建")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /前往备份设置配置 WebDAV/ }));
    await waitFor(() => {
      expect(useUIStore.getState().activePage).toBe("settings");
      expect(useUIStore.getState().settingsTab).toBe("backup");
    });
  });
});
