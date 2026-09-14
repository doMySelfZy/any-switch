import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { App as AntdApp, ConfigProvider } from "antd";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useApplyStore, useSiteStore, useUIStore } from "@/stores";
import type { ApplyRequest, ApplyResult, Site, SiteModel } from "@/types/domain";
import { ZCodeApplyPanel } from "./ZCodeApplyPanel";
import "@/i18n";

const site: Site = {
  id: "site-1",
  name: "Relay",
  baseUrl: "https://api.example.com",
  baseUrls: ["https://api.example.com"],
  keyPrefix: "sk-xx",
  quotaRevision: "rev-1",
  hasKey: true,
  protocol: "openai_compatible",
  claudeAuthKeyStyle: "anthropic_auth_token",
  notes: null,
  enabled: true,
  sortOrder: 0,
  selectedModelId: "model-a",
  lastModelFetchAt: null,
  lastModelFetchLatencyMs: null,
  lastModelFetchError: null,
  createdAt: 1,
  updatedAt: 1,
  zcodeApiType: "openai-chat-completions",
};

const models: SiteModel[] = [
  {
    id: "model-row",
    siteId: site.id,
    modelId: "model-a",
    displayName: "Model A",
    ownedBy: null,
    raw: null,
  },
];

function renderPanel() {
  render(
    <ConfigProvider>
      <AntdApp>
        <ZCodeApplyPanel />
      </AntdApp>
    </ConfigProvider>,
  );
}

/** 协议下拉框：三个 Select 在 DOM 里依次是站点、模型、协议。 */
function protocolCombobox(): HTMLElement {
  const row = screen.getByText("API 协议").parentElement as HTMLElement;
  const combobox = row.querySelector('[role="combobox"]');
  if (!combobox) throw new Error("protocol combobox not found");
  return combobox as HTMLElement;
}

describe("ZCodeApplyPanel", () => {
  const apply = vi.fn<(request: ApplyRequest) => Promise<ApplyResult>>();
  const updateSite = vi.fn().mockResolvedValue(site);

  beforeEach(() => {
    apply.mockReset();
    updateSite.mockClear();
    apply.mockResolvedValue({
      siteId: site.id,
      modelId: "model-a",
      results: [
        {
          target: "zcode",
          ok: true,
          status: "applied",
          backupPaths: [],
          message: "ok",
        },
      ],
      appliedAt: 2,
    });
    useUIStore.setState({ selectedSiteId: site.id, applyPrefillSiteId: null });
    useSiteStore.setState({
      sites: [site],
      modelsBySite: { [site.id]: models },
      modelsLoadingBySite: {},
      listModels: vi.fn().mockResolvedValue(models),
      updateSite,
    });
    useApplyStore.setState({
      statuses: [
        {
          kind: "zcode",
          installed: true,
          version: "0.1.0",
          configPath: "/tmp/.zcode/v2/config.json",
          status: "applied",
          appliedSiteId: site.id,
          appliedSiteName: site.name,
          appliedModelId: "model-a",
          providerId: "xiaobai_site1",
          orphan: false,
          liveSummary: {
            defaultModel: "model-a",
            apiType: "openai-chat-completions",
            modelCount: "2",
            writeAllModels: "true",
          },
          lastAppliedAt: 1,
          staleReason: null,
        },
      ],
      tools: [],
      loading: false,
      applying: false,
      apply,
    });
  });

  it("hydrates the stored protocol and the write-all-models switch", async () => {
    renderPanel();

    expect(await screen.findByText("ZCode 连接协议")).toBeInTheDocument();
    // 站点已存的协议回显在 select 上，而不是按站点 protocol 推断的 openai-responses。
    expect(screen.getByText("OpenAI Chat Completions")).toBeInTheDocument();
    // 开启状态由 live summary 还原（模型数来自本地模型列表）。
    expect(await screen.findByText("将写入 1 个模型到目录文件")).toBeInTheDocument();
  });

  it("persists a protocol change before applying the ZCode request", async () => {
    renderPanel();

    fireEvent.mouseDown(protocolCombobox());
    fireEvent.click(await screen.findByTitle("Anthropic Messages"));

    await waitFor(() => {
      expect(updateSite).toHaveBeenCalledWith(site.id, {
        zcodeApiType: "anthropic-messages",
      });
    });

    fireEvent.click(screen.getByRole("button", { name: "应用配置" }));

    await waitFor(() => {
      expect(apply).toHaveBeenCalledWith({
        siteId: site.id,
        targets: ["zcode"],
        modelId: "model-a",
        zcodeWriteAllModels: true,
      });
    });
    // 协议先落库、再应用：后端读的是站点上的 zcode_api_type。
    const persistOrder =
      updateSite.mock.invocationCallOrder[updateSite.mock.invocationCallOrder.length - 1]!;
    const applyOrder = apply.mock.invocationCallOrder[apply.mock.invocationCallOrder.length - 1]!;
    expect(persistOrder).toBeLessThan(applyOrder);
  });
});
