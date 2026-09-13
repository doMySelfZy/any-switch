import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { App as AntdApp, ConfigProvider } from "antd";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { resetBrowserMock, seedBackups } from "@/lib/browserMock";
import { revealInExplorer } from "@/lib/revealInExplorer";
import { useApplyStore } from "@/stores";
import type { BackupInfo } from "@/types/domain";
import { TargetBackupList } from "./TargetBackupList";
import "@/i18n";

vi.mock("@/lib/revealInExplorer", () => ({
  revealInExplorer: vi.fn().mockResolvedValue(undefined),
}));

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
  dir: "/Users/lmini/.xiaobai-switch/backups/claude_code/1710000000000",
  createdAt: 1710000000000,
  files: ["settings.json"],
  applyRecordId: "rec-1",
  siteNameSnapshot: "shuai",
  modelId: "gpt-5.6",
};

describe("TargetBackupList", () => {
  beforeEach(() => {
    resetBrowserMock();
    seedBackups([backup]);
    useApplyStore.setState({ backups: [] });
    vi.mocked(revealInExplorer).mockClear();
  });

  it("lists backups and shows a summary preview like the live config card", async () => {
    render(
      <Wrapper>
        <TargetBackupList target="claude_code" />
      </Wrapper>,
    );

    expect(await screen.findByText("shuai")).toBeInTheDocument();
    expect(screen.getByText("（gpt-5.6）")).toBeInTheDocument();

    fireEvent.click(screen.getByText("shuai"));
    expect(await screen.findByText("ANTHROPIC_MODEL")).toBeInTheDocument();
    expect(screen.getByText("gpt-5.6")).toBeInTheDocument();
    expect(screen.getByText("ANTHROPIC_BASE_URL")).toBeInTheDocument();
    expect(screen.getByText("settings.json")).toBeInTheDocument();
    expect(document.querySelector(".ant-collapse-ghost")).toBeNull();
  });

  it("reveals the source file in the explorer", async () => {
    render(
      <Wrapper>
        <TargetBackupList target="claude_code" />
      </Wrapper>,
    );
    fireEvent.click(await screen.findByText("shuai"));
    fireEvent.click(await screen.findByRole("button", { name: /查看源文件/ }));
    await waitFor(() => {
      expect(revealInExplorer).toHaveBeenCalledWith(`${backup.dir}/settings.json`);
    });
  });

  it("asks before restoring and overwriting the current files", async () => {
    const restoreBackup = vi.fn().mockResolvedValue(undefined);
    useApplyStore.setState({ restoreBackup });

    render(
      <Wrapper>
        <TargetBackupList target="claude_code" />
      </Wrapper>,
    );
    fireEvent.click(await screen.findByText("shuai"));
    fireEvent.click(await screen.findByRole("button", { name: /还\s*原/ }));

    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getAllByText("确定还原这份备份？").length).toBeGreaterThan(0);
    expect(within(dialog).getByText(/直接覆盖当前配置文件/)).toBeInTheDocument();

    fireEvent.click(within(dialog).getByRole("button", { name: /覆盖并还原/ }));
    await waitFor(() => expect(restoreBackup).toHaveBeenCalledWith(backup.id));
    expect(await screen.findByText("已还原备份，当前配置已被覆盖")).toBeInTheDocument();
  });

  it("asks before permanently deleting a backup", async () => {
    const deleteBackup = vi.fn().mockResolvedValue(undefined);
    useApplyStore.setState({ deleteBackup });

    render(
      <Wrapper>
        <TargetBackupList target="claude_code" />
      </Wrapper>,
    );
    fireEvent.click(await screen.findByText("shuai"));
    fireEvent.click(await screen.findByRole("button", { name: /删\s*除/ }));

    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getAllByText("确定永久删除这份备份？").length).toBeGreaterThan(0);
    expect(within(dialog).getByText(/无法恢复/)).toBeInTheDocument();

    fireEvent.click(within(dialog).getByRole("button", { name: /永久删除/ }));
    await waitFor(() => expect(deleteBackup).toHaveBeenCalledWith(backup.id));
    expect(await screen.findByText("已删除备份")).toBeInTheDocument();
  });

  it("does not list another target's backups", async () => {
    render(
      <Wrapper>
        <TargetBackupList target="codex" />
      </Wrapper>,
    );
    expect(await screen.findByText("暂无备份")).toBeInTheDocument();
    expect(screen.queryByText("shuai")).not.toBeInTheDocument();
  });
});
