import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { App as AntdApp, ConfigProvider } from "antd";
import type { ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import i18n from "@/i18n";
import { resetBrowserMock, seedWebDavMock } from "@/lib/browserMock";
import * as invokeMod from "@/lib/invoke";
import { WebDavBackupSettings } from "./WebDavBackupSettings";

function Wrapper({ children }: { children: ReactNode }) {
  return (
    <ConfigProvider theme={{ token: { motion: false } }}>
      <AntdApp>{children}</AntdApp>
    </ConfigProvider>
  );
}

/** 只在 sync_now 上抛错，其余命令继续走浏览器 mock，避免测试依赖真实后端。 */
function failSyncWith(error: { code: string; message: string }) {
  const realInvoke = invokeMod.invoke;
  vi.spyOn(invokeMod, "invoke").mockImplementation(async (cmd, args) => {
    if (cmd === "sync_now") throw error;
    return realInvoke(cmd, args);
  });
}

describe("WebDavBackupSettings", () => {
  beforeEach(() => {
    resetBrowserMock();
    seedWebDavMock(
      { baseUrl: "https://dav.example.com/", username: "alice", hasPassword: true },
      [],
    );
  });

  afterEach(async () => {
    resetBrowserMock();
    vi.restoreAllMocks();
    await i18n.changeLanguage("zh-CN");
  });

  it("localizes the fingerprint algorithm mismatch error for both languages", async () => {
    failSyncWith({
      code: "sync_algorithm_mismatch",
      message: "remote fingerprint algorithm 2 does not match local 1; upgrade the other device",
    });
    render(
      <Wrapper>
        <WebDavBackupSettings />
      </Wrapper>,
    );

    const syncButton = await screen.findByRole("button", { name: /立即同步/ });
    await waitFor(() => expect(syncButton).toBeEnabled());
    fireEvent.click(syncButton);
    expect(await screen.findByText(/同步已暂停：对端设备的数据指纹算法/)).toBeInTheDocument();
    expect(screen.queryByText(/upgrade the other device/)).toBeNull();
    await waitFor(() => expect(syncButton).toBeEnabled());

    await i18n.changeLanguage("en-US");
    fireEvent.click(await screen.findByRole("button", { name: /Sync now/ }));
    expect(await screen.findByText(/Sync paused: the other device/)).toBeInTheDocument();
  });

  it("keeps showing the backend message for other sync errors", async () => {
    failSyncWith({ code: "network", message: "network error: connection refused" });
    render(
      <Wrapper>
        <WebDavBackupSettings />
      </Wrapper>,
    );

    const syncButton = await screen.findByRole("button", { name: /立即同步/ });
    await waitFor(() => expect(syncButton).toBeEnabled());
    fireEvent.click(syncButton);

    expect(await screen.findByText(/connection refused/)).toBeInTheDocument();
  });
});
