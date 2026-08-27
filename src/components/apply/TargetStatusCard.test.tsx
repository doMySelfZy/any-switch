import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { App as AntdApp, ConfigProvider } from "antd";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { Site, TargetLiveStatus } from "@/types/domain";
import { useSiteStore } from "@/stores";
import {
  cliVersionLabel,
  isConfiguredStatus,
  TargetStatusCard,
  targetsAppliedForSite,
} from "./TargetStatusCard";
import { revealInExplorer } from "@/lib/revealInExplorer";
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

const site: Site = {
  id: "shuai",
  name: "shuai",
  baseUrl: "https://api.example.com",
  baseUrls: ["https://api.example.com"],
  keyPrefix: "sk-",
  quotaRevision: "rev-1",
  hasKey: true,
  protocol: "openai_compatible",
  claudeAuthKeyStyle: "anthropic_auth_token",
  notes: null,
  enabled: true,
  sortOrder: 0,
  selectedModelId: "gpt-5.6",
  lastModelFetchAt: null,
  lastModelFetchLatencyMs: null,
  lastModelFetchError: null,
  createdAt: 1,
  updatedAt: 1,
};

const status: TargetLiveStatus = {
  kind: "claude_code",
  installed: true,
  version: "2.1.197 (Claude Code)",
  configPath: "/Users/lmini/.claude/settings.json",
  status: "applied",
  appliedSiteId: "shuai",
  appliedSiteName: "shuai",
  appliedModelId: "gpt-5.6",
  providerId: null,
  orphan: false,
  liveSummary: {
    ANTHROPIC_MODEL: "gpt-5.6",
    ANTHROPIC_BASE_URL: "https://api.example.com",
  },
  lastAppliedAt: 1,
  staleReason: null,
};

function renderCard(overrides?: {
  status?: TargetLiveStatus;
  toolVersion?: string | null;
  onRefresh?: () => Promise<void>;
  onRevert?: () => Promise<void>;
  onCleanupOrphan?: () => Promise<void>;
}) {
  return render(
    <Wrapper>
      <TargetStatusCard
        status={overrides?.status ?? status}
        tool={{
          kind: "claude_code",
          installed: true,
          version: overrides?.toolVersion === undefined ? status.version : overrides.toolVersion,
          path: null,
        }}
        onRefresh={overrides?.onRefresh ?? vi.fn().mockResolvedValue(undefined)}
        onRevert={overrides?.onRevert ?? vi.fn().mockResolvedValue(undefined)}
        onCleanupOrphan={overrides?.onCleanupOrphan ?? vi.fn().mockResolvedValue(undefined)}
      />
    </Wrapper>,
  );
}

describe("cliVersionLabel", () => {
  it("extracts a numeric version from CLI --version output", () => {
    expect(cliVersionLabel("2.1.197 (Claude Code)")).toBe("2.1.197");
    expect(cliVersionLabel("codex-cli 0.42.0")).toBe("0.42.0");
    expect(cliVersionLabel("  ")).toBeNull();
  });
});

describe("TargetStatusCard", () => {
  beforeEach(() => {
    useSiteStore.setState({
      sites: [site],
      modelsBySite: {},
      modelsLoadingBySite: {},
      loading: false,
      hydrated: true,
      fetchingModels: false,
      error: null,
    });
    vi.mocked(revealInExplorer).mockClear();
  });

  it("puts install status next to the name and merges site + model", () => {
    renderCard();

    expect(screen.getByText("Claude Code")).toBeInTheDocument();
    const installed = screen.getByText("2.1.197");
    expect(installed.closest(".ant-tag")).toHaveClass("ant-tag-green");
    expect(screen.queryByText("已安装")).not.toBeInTheDocument();
    expect(screen.queryByText("安装状态")).not.toBeInTheDocument();

    expect(screen.getByText("站点模型")).toBeInTheDocument();
    expect(screen.getByText("shuai")).toBeInTheDocument();
    expect(screen.getByText("（gpt-5.6）")).toBeInTheDocument();
    expect(screen.queryByText("已应用站点")).not.toBeInTheDocument();
    expect(screen.queryByText(/^模型$/)).not.toBeInTheDocument();

    const siteModel = screen.getByText("站点模型").parentElement;
    expect(siteModel?.querySelector(".ant-avatar")).toBeTruthy();

    expect(screen.getByText("配置路径")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: status.configPath })).toBeInTheDocument();

    const revertBtn = screen.getByRole("button", { name: /清\s*除/ });
    const refreshBtn = screen.getByRole("button", { name: /刷\s*新/ });
    expect(revertBtn).toBeInTheDocument();
    expect(revertBtn.className).toMatch(/ant-btn-dangerous/);
    expect(revertBtn.querySelector("svg.lucide")).toBeTruthy();
    expect(refreshBtn).toBeInTheDocument();
    expect(refreshBtn.querySelector("svg.lucide")).toBeTruthy();
    expect(screen.queryByRole("button", { name: /从目标移除/ })).not.toBeInTheDocument();

    expect(screen.getByText("当前配置摘要")).toBeInTheDocument();
    expect(document.querySelector(".ant-collapse")).toHaveStyle({ marginTop: "24px" });
    expect(document.querySelector(".ant-collapse-ghost")).toBeNull();
    expect(document.querySelector(".ant-collapse-item-active")).toBeNull();
    expect(screen.queryByText("ANTHROPIC_BASE_URL")).not.toBeInTheDocument();
  });

  it("uses a red tag when the CLI is not installed", () => {
    renderCard({
      status: {
        ...status,
        installed: false,
        version: null,
        status: "not_applied",
        appliedSiteId: null,
        appliedSiteName: null,
        appliedModelId: null,
      },
    });

    const tag = screen.getByText("未检测到").closest(".ant-tag");
    expect(tag).toHaveClass("ant-tag-red");
    expect(screen.queryByRole("button", { name: /清\s*除/ })).not.toBeInTheDocument();
  });

  it("falls back to 已安装 when the CLI is present but version is unknown", () => {
    renderCard({
      status: { ...status, version: null },
      toolVersion: null,
    });

    const tag = screen.getByText("已安装").closest(".ant-tag");
    expect(tag).toHaveClass("ant-tag-green");
    expect(screen.queryByText("2.1.197")).not.toBeInTheDocument();
  });

  it("shows loading on refresh and a success message", async () => {
    let finish!: () => void;
    const onRefresh = vi.fn(
      () =>
        new Promise<void>((resolve) => {
          finish = resolve;
        }),
    );
    renderCard({ onRefresh });

    fireEvent.click(screen.getByRole("button", { name: /刷\s*新/ }));
    expect(screen.getByRole("button", { name: /刷\s*新/ }).className).toMatch(/ant-btn-loading/);

    finish();
    expect(await screen.findByText("刷新状态成功")).toBeInTheDocument();
    await waitFor(() => {
      expect(screen.getByRole("button", { name: /刷\s*新/ }).className).not.toMatch(/ant-btn-loading/);
    });
  });

  it("asks for confirmation before clearing and toasts success", async () => {
    const onRevert = vi.fn().mockResolvedValue(undefined);
    renderCard({ onRevert });

    fireEvent.click(screen.getByRole("button", { name: /清\s*除/ }));
    const pop = await screen.findByRole("tooltip");
    fireEvent.click(within(pop).getByRole("button", { name: /确\s*认/ }));

    await waitFor(() => expect(onRevert).toHaveBeenCalledTimes(1));
    expect(await screen.findByText("清除配置成功")).toBeInTheDocument();
  });

  it("opens the config path in the file manager", async () => {
    renderCard();

    fireEvent.click(screen.getByRole("button", { name: status.configPath }));
    await waitFor(() => {
      expect(revealInExplorer).toHaveBeenCalledWith(status.configPath);
    });
  });
});

describe("isConfiguredStatus", () => {
  it("treats applied, stale, and orphan as configured", () => {
    expect(isConfiguredStatus("applied")).toBe(true);
    expect(isConfiguredStatus("stale")).toBe(true);
    expect(isConfiguredStatus("orphan")).toBe(true);
    expect(isConfiguredStatus("not_applied")).toBe(false);
    expect(isConfiguredStatus("failed")).toBe(false);
    expect(isConfiguredStatus(undefined)).toBe(false);
  });
});

describe("targetsAppliedForSite", () => {
  it("returns target kinds whose live config belongs to the site", () => {
    expect(
      targetsAppliedForSite(
        [
          { ...status, kind: "claude_code", appliedSiteId: "shuai" },
          { ...status, kind: "codex", appliedSiteId: "other", status: "applied" },
        ],
        "shuai",
      ),
    ).toEqual(["claude_code"]);
    expect(targetsAppliedForSite([{ ...status, appliedSiteId: null }], "shuai")).toEqual([]);
  });
});
