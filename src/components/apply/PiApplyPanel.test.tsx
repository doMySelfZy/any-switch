import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { App as AntdApp, ConfigProvider } from "antd";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useApplyStore, useSiteStore, useUIStore } from "@/stores";
import type { ApplyRequest, ApplyResult, Site, SiteModel } from "@/types/domain";
import { PiApplyPanel } from "./PiApplyPanel";
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

describe("PiApplyPanel", () => {
  const apply = vi.fn<(request: ApplyRequest) => Promise<ApplyResult>>();

  beforeEach(() => {
    apply.mockReset();
    apply.mockResolvedValue({
      siteId: site.id,
      modelId: "model-a",
      results: [
        {
          target: "pi",
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
      updateSite: vi.fn().mockResolvedValue(site),
    });
    useApplyStore.setState({
      statuses: [
        {
          kind: "pi",
          installed: true,
          version: "0.84.3",
          configPath: "/tmp/models.json",
          status: "applied",
          appliedSiteId: site.id,
          appliedSiteName: site.name,
          appliedModelId: "model-a",
          providerId: "xiaobai_site1",
          orphan: false,
          liveSummary: { defaultModel: "model-a", modelCount: "1", writeAllModels: "true" },
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

  it("hydrates model count and sends the Pi-specific request", async () => {
    render(
      <ConfigProvider>
        <AntdApp>
          <PiApplyPanel />
        </AntdApp>
      </ConfigProvider>,
    );

    expect(await screen.findByText("将写入 1 个模型到目录文件")).toBeInTheDocument();
    expect(await screen.findByText("默认思考等级")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "应用配置" }));
    await waitFor(() => {
      expect(apply).toHaveBeenCalledWith({
        siteId: site.id,
        targets: ["pi"],
        modelId: "model-a",
        piWriteAllModels: true,
      });
    });
  });
});
