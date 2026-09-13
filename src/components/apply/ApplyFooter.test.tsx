import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { App as AntdApp, ConfigProvider } from "antd";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { resetBrowserMock, seedBackups } from "@/lib/browserMock";
import { useApplyStore } from "@/stores";
import type { BackupInfo } from "@/types/domain";
import { ApplyFooter } from "./ApplyFooter";
import "@/i18n";

function Wrapper({ children }: { children: ReactNode }) {
  return (
    <ConfigProvider>
      <AntdApp>{children}</AntdApp>
    </ConfigProvider>
  );
}

const backup: BackupInfo = {
  id: "claude_code-1710000000000",
  target: "claude_code",
  dir: "/tmp/backups/claude_code/1710000000000",
  createdAt: 1710000000000,
  files: ["settings.json"],
  applyRecordId: "rec-1",
  siteNameSnapshot: "shuai",
  modelId: "gpt-5.6",
};

describe("ApplyFooter", () => {
  beforeEach(() => {
    resetBrowserMock();
    seedBackups([backup]);
    useApplyStore.setState({ backups: [] });
  });

  it("puts backup records beside apply and opens them in a modal", async () => {
    const onApply = vi.fn();
    render(
      <Wrapper>
        <ApplyFooter
          target="claude_code"
          loading={false}
          disabled={false}
          onApply={onApply}
          onRestoreOfficial={vi.fn()}
        />
      </Wrapper>,
    );

    const footer = screen.getByTestId("apply-footer");
    expect(footer).toHaveTextContent("配置备份记录");
    expect(footer).toHaveTextContent("应用配置");
    expect(footer).toHaveTextContent("还原官方配置");
    expect(screen.getByRole("button", { name: /配置备份记录/ }).querySelector("svg.lucide")).toBeTruthy();
    expect(screen.getByRole("button", { name: "应用配置" }).querySelector("svg.lucide")).toBeTruthy();
    expect(screen.getByRole("button", { name: "还原官方配置" }).querySelector("svg.lucide")).toBeTruthy();
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /配置备份记录/ }));
    const dialog = await screen.findByRole("dialog");
    expect(dialog).toHaveTextContent("配置备份记录");
    expect(await screen.findByText("shuai")).toBeInTheDocument();
    expect(screen.getByText("（gpt-5.6）")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "应用配置" }));
    expect(onApply).toHaveBeenCalledTimes(1);
  });

  it("confirms before restoring official config", async () => {
    const onRestoreOfficial = vi.fn().mockResolvedValue(undefined);
    render(
      <Wrapper>
        <ApplyFooter
          target="claude_code"
          loading={false}
          disabled={false}
          onApply={() => {}}
          onRestoreOfficial={onRestoreOfficial}
        />
      </Wrapper>,
    );

    fireEvent.click(screen.getByRole("button", { name: "还原官方配置" }));
    const dialog = await screen.findByRole("dialog");
    expect(dialog).toHaveTextContent("还原为官方配置？");
    expect(dialog).toHaveTextContent("ANTHROPIC_BASE_URL");
    expect(onRestoreOfficial).not.toHaveBeenCalled();

    fireEvent.click(within(dialog).getByRole("button", { name: "还原官方配置" }));
    await waitFor(() => {
      expect(onRestoreOfficial).toHaveBeenCalledTimes(1);
    });
    await waitFor(() => {
      expect(screen.getAllByText("已还原官方配置").length).toBeGreaterThan(0);
    });
    expect(screen.getByText("已清除 Claude Code 的中转配置，可使用官方账号登录。")).toBeInTheDocument();
  });

  it("uses Codex-specific official restore copy", async () => {
    render(
      <Wrapper>
        <ApplyFooter
          target="codex"
          loading={false}
          disabled
          onApply={() => {}}
          onRestoreOfficial={vi.fn()}
        />
      </Wrapper>,
    );

    expect(screen.getByRole("button", { name: "应用配置" })).toBeDisabled();
    const restore = screen.getByRole("button", { name: "还原官方配置" });
    expect(restore).toBeEnabled();
    fireEvent.click(restore);
    const dialog = await screen.findByRole("dialog");
    expect(dialog).toHaveTextContent("openai_base_url");
    expect(dialog).toHaveTextContent("ChatGPT");
  });

  it("uses a scoped remove action for Pi", async () => {
    const remove = vi.fn().mockResolvedValue(undefined);
    render(
      <Wrapper>
        <ApplyFooter
          target="pi"
          loading={false}
          disabled={false}
          onApply={() => {}}
          onRestoreOfficial={remove}
        />
      </Wrapper>,
    );

    fireEvent.click(screen.getByRole("button", { name: "移除 AnySwitch 配置" }));
    const dialog = await screen.findByRole("dialog");
    expect(dialog).toHaveTextContent("其他自定义 Provider、OAuth 登录与未知设置都会保留");
    fireEvent.click(within(dialog).getByRole("button", { name: "移除配置" }));
    await waitFor(() => expect(remove).toHaveBeenCalledTimes(1));
  });

  it("uses a scoped remove action for Prime", async () => {
    const remove = vi.fn().mockResolvedValue(undefined);
    render(
      <Wrapper>
        <ApplyFooter
          target="prime"
          loading={false}
          disabled={false}
          onApply={() => {}}
          onRestoreOfficial={remove}
        />
      </Wrapper>,
    );

    fireEvent.click(screen.getByRole("button", { name: "移除 AnySwitch 配置" }));
    const dialog = await screen.findByRole("dialog");
    expect(dialog).toHaveTextContent("~/.prime/agent/auth.json");
    fireEvent.click(within(dialog).getByRole("button", { name: "移除配置" }));
    await waitFor(() => expect(remove).toHaveBeenCalledTimes(1));
  });

  it("does not list backups until the modal is opened", () => {
    render(
      <Wrapper>
        <ApplyFooter
          target="claude_code"
          loading={false}
          disabled={false}
          onApply={() => {}}
          onRestoreOfficial={() => {}}
        />
      </Wrapper>,
    );
    expect(screen.queryByText("shuai")).not.toBeInTheDocument();
    expect(screen.queryByText("暂无备份")).not.toBeInTheDocument();
  });
});
