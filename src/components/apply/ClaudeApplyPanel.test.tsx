import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { App as AntdApp, ConfigProvider } from "antd";
import type { ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { resetBrowserMock } from "@/lib/browserMock";
import { useApplyStore, useSiteStore, useUIStore } from "@/stores";
import type { TargetLiveStatus } from "@/types/domain";
import { ClaudeApplyPanel } from "./ClaudeApplyPanel";
import "@/i18n";

function Wrapper({ children }: { children: ReactNode }) {
  return (
    <ConfigProvider>
      <AntdApp>{children}</AntdApp>
    </ConfigProvider>
  );
}

function resetStores() {
  resetBrowserMock();
  useSiteStore.setState({
    sites: [],
    modelsBySite: {},
    modelsLoadingBySite: {},
    loading: false,
    hydrated: false,
    fetchingModels: false,
    error: null,
  });
  useUIStore.setState({
    selectedSiteId: null,
    applyPrefillSiteId: null,
    activePage: "apply",
    applyTab: "claude_code",
  });
  useApplyStore.setState({
    statuses: [],
    tools: [],
    records: [],
    backups: [],
    applying: false,
    loading: false,
    statusHydrated: true,
    lastResult: null,
  });
}

describe("ClaudeApplyPanel", () => {
  beforeEach(() => {
    resetStores();
  });

  afterEach(() => {
    resetStores();
  });

  it("defaults form fields to the live tool config, not the site primary model", async () => {
    const created = await useSiteStore.getState().createSite({
      name: "shuai",
      baseUrl: "https://api.example.com",
      apiKey: "sk-test",
    });
    await useSiteStore.getState().fetchModels(created.id);
    await useSiteStore.getState().setSelectedModel(created.id, "gpt-4.1");
    const site = useSiteStore.getState().sites[0]!;
    useUIStore.getState().setSelectedSiteId(site.id);

    const live: TargetLiveStatus = {
      kind: "claude_code",
      installed: true,
      version: "2.1.197 (Claude Code)",
      configPath: "/Users/lmini/.claude/settings.json",
      status: "applied",
      appliedSiteId: site.id,
      appliedSiteName: site.name,
      appliedModelId: "codex-auto-review",
      providerId: null,
      orphan: false,
      liveSummary: {
        ANTHROPIC_MODEL: "codex-auto-review[1m]",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "opus-live[1m]",
        ANTHROPIC_DEFAULT_FABLE_MODEL: "fable-live",
        ANTHROPIC_AUTH_TOKEN: "sk-live",
        CLAUDE_CODE_EFFORT_LEVEL: "max",
      },
      lastAppliedAt: 42,
      staleReason: null,
    };
    useApplyStore.setState({
      statuses: [live],
      tools: [{ kind: "claude_code", installed: true, version: live.version, path: null }],
      statusHydrated: true,
      loading: false,
    });

    await act(async () => {
      render(
        <Wrapper>
          <div style={{ height: 800 }}>
            <ClaudeApplyPanel />
          </div>
        </Wrapper>,
      );
    });

    await waitFor(() => {
      expect(document.querySelector('.ant-select-content[title="codex-auto-review"]')).toBeTruthy();
    });
    expect(document.querySelector('.ant-select-content[title="opus-live"]')).toBeTruthy();
    expect(document.querySelector('.ant-select-content[title="fable-live"]')).toBeTruthy();
    expect(document.querySelector('.ant-select-content[title="Extra High"]')).toBeTruthy();
    expect(
      document.querySelector('.ant-select-content[title="ANTHROPIC_AUTH_TOKEN（推荐）"]'),
    ).toBeTruthy();
    expect(screen.getByRole("switch", { name: "1M 上下文" })).toBeChecked();

    const footer = screen.getByTestId("apply-footer");
    expect(footer).toBeInTheDocument();
    expect(footer.compareDocumentPosition(screen.getByText("状态")) & Node.DOCUMENT_POSITION_PRECEDING).toBeTruthy();
    expect(screen.getByRole("button", { name: "应用配置" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "还原官方配置" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /配置备份记录/ })).toBeInTheDocument();
    expect(screen.getByTestId("apply-footer").compareDocumentPosition(screen.getByRole("button", { name: /配置备份记录/ })) & Node.DOCUMENT_POSITION_CONTAINED_BY).toBeTruthy();
    expect(footer).not.toHaveTextContent("请重启终端");
    expect(screen.queryByText(/settings.json updated/i)).not.toBeInTheDocument();

    expect(screen.getByText("2.1.197")).toBeInTheDocument();
    expect(screen.getByText("站点模型")).toBeInTheDocument();
    expect(document.querySelector(".ant-collapse-item-active")).toBeNull();

    const siteSelected = document.querySelector(".ant-select-content");
    expect(siteSelected?.textContent ?? "").toContain("shuai");
    expect(siteSelected?.querySelector(".ant-avatar")).toBeTruthy();

    fireEvent.mouseDown(screen.getAllByRole("combobox")[0]!);
    await waitFor(() => {
      const dropdown = document.querySelector(".ant-select-dropdown");
      expect(dropdown).toBeTruthy();
      expect(dropdown?.querySelectorAll(".ant-avatar").length).toBeGreaterThan(0);
    });
  });

  it("opens a localized result modal after apply instead of pinning English copy", async () => {
    const created = await useSiteStore.getState().createSite({
      name: "shuai",
      baseUrl: "https://api.example.com",
      apiKey: "sk-test",
    });
    await useSiteStore.getState().fetchModels(created.id);
    await useSiteStore.getState().setSelectedModel(created.id, "gpt-4.1");
    const site = useSiteStore.getState().sites[0]!;
    useUIStore.getState().setSelectedSiteId(site.id);
    useApplyStore.setState({
      statuses: [
        {
          kind: "claude_code",
          installed: true,
          version: "2.1.197",
          configPath: "/tmp/settings.json",
          status: "applied",
          appliedSiteId: site.id,
          appliedSiteName: site.name,
          appliedModelId: "gpt-4.1",
          providerId: null,
          orphan: false,
          liveSummary: {
            model: "gpt-4.1",
            ANTHROPIC_DEFAULT_FABLE_MODEL: "gpt-4.1",
            effortLevel: "xhigh",
          },
          lastAppliedAt: null,
          staleReason: null,
        },
      ],
      statusHydrated: true,
      loading: false,
    });

    await act(async () => {
      render(
        <Wrapper>
          <div style={{ height: 800 }}>
            <ClaudeApplyPanel />
          </div>
        </Wrapper>,
      );
    });

    await waitFor(() => {
      expect(screen.getByRole("button", { name: "应用配置" })).toBeEnabled();
    });

    fireEvent.click(screen.getByRole("switch", { name: "1M 上下文" }));

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "应用配置" }));
    });

    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getAllByText("应用成功").length).toBeGreaterThan(0);
    expect(within(dialog).getByText("已写入 Claude Code 的 settings.json。")).toBeInTheDocument();
    expect(
      within(dialog).getByText("请重启终端或重新打开对应 CLI 工具使配置生效。"),
    ).toBeInTheDocument();
    expect(within(dialog).queryByText(/settings.json updated/i)).not.toBeInTheDocument();
    expect(within(dialog).queryByText(/Browser mock/i)).not.toBeInTheDocument();
    expect(
      useApplyStore.getState().statuses.find((row) => row.kind === "claude_code")?.liveSummary
        .model,
    ).toBe("gpt-4.1[1m]");
    expect(
      useApplyStore.getState().statuses.find((row) => row.kind === "claude_code")?.liveSummary
        .ANTHROPIC_DEFAULT_FABLE_MODEL,
    ).toBe("gpt-4.1");
    expect(
      useApplyStore.getState().statuses.find((row) => row.kind === "claude_code")?.liveSummary
        .effortLevel,
    ).toBe("xhigh");
    expect(
      useApplyStore.getState().statuses.find((row) => row.kind === "claude_code")?.liveSummary
        .CLAUDE_CODE_EFFORT_LEVEL,
    ).toBeUndefined();
  });

  it("groups disabled sites as unselectable in the apply picker", async () => {
    const enabled = await useSiteStore.getState().createSite({
      name: "Enabled Relay",
      baseUrl: "https://api.example.com",
      apiKey: "sk-test",
    });
    const disabled = await useSiteStore.getState().createSite({
      name: "Disabled Relay",
      baseUrl: "https://api2.example.com",
      apiKey: "sk-test-2",
    });
    await useSiteStore.getState().updateSite(disabled.id, { enabled: false });
    useUIStore.getState().setSelectedSiteId(enabled.id);

    await act(async () => {
      render(
        <Wrapper>
          <div style={{ height: 800 }}>
            <ClaudeApplyPanel />
          </div>
        </Wrapper>,
      );
    });

    fireEvent.mouseDown(screen.getAllByRole("combobox")[0]!);
    const dropdown = await waitFor(() => {
      const el = document.querySelector(".ant-select-dropdown");
      expect(el).toBeTruthy();
      return el as HTMLElement;
    });
    expect(dropdown).toHaveTextContent("已启用");
    expect(dropdown).toHaveTextContent("未启用");
    expect(dropdown).toHaveTextContent("Enabled Relay");
    expect(dropdown).toHaveTextContent("Disabled Relay");

    const disabledOption = Array.from(dropdown.querySelectorAll(".ant-select-item-option")).find(
      (el) => el.textContent?.includes("Disabled Relay"),
    );
    expect(disabledOption).toHaveClass("ant-select-item-option-disabled");

    fireEvent.click(disabledOption!);
    const siteSelected = document.querySelector(".ant-select-content");
    expect(siteSelected?.textContent ?? "").toContain("Enabled Relay");
    expect(siteSelected?.textContent ?? "").not.toContain("Disabled Relay");
  });
});
