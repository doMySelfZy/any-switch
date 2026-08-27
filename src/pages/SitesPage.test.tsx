import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { App as AntdApp, ConfigProvider } from "antd";
import type { ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { resetBrowserMock, seedTargetStatuses } from "@/lib/browserMock";
import { useApplyStore, useSiteStore, useUIStore } from "@/stores";
import { resetQuotaInflight } from "@/stores/siteStore";
import type { TargetLiveStatus } from "@/types/domain";
import { SitesPage } from "./SitesPage";
import "@/i18n";

function Wrapper({ children }: { children: ReactNode }) {
  return (
    <ConfigProvider>
      <AntdApp>{children}</AntdApp>
    </ConfigProvider>
  );
}

function modelTag(modelId: string) {
  return document.querySelector(`[data-model-tag][title="${modelId}"]`);
}

async function seedSite() {
  const site = await useSiteStore.getState().createSite({
    name: "Relay One",
    baseUrl: "https://api.example.com",
    apiKey: "sk-test",
  });
  await useSiteStore.getState().fetchModels(site.id);
  useUIStore.getState().setSelectedSiteId(site.id);
  return site;
}

describe("SitesPage", () => {
  beforeEach(() => {
    resetBrowserMock();
    resetQuotaInflight();
    useSiteStore.setState({
      sites: [],
      modelsBySite: {},
      modelsLoadingBySite: {},
      quotaBySite: {},
      quotaCacheKeyBySite: {},
      quotaLoadingBySite: {},
      loading: false,
      hydrated: false,
      fetchingModels: false,
      error: null,
    });
    useUIStore.setState({ selectedSiteId: null, activePage: "sites", pendingSiteForm: null });
    useApplyStore.setState({
      statuses: [],
      tools: [],
      records: [],
      backups: [],
      applying: false,
      loading: false,
      statusHydrated: false,
      lastResult: null,
    });
  });

  afterEach(() => {
    resetBrowserMock();
  });

  it("renders detail without a card, current-model copy, small add, and apply beside the site name", async () => {
    await act(async () => {
      await seedSite();
    });

    render(
      <Wrapper>
        <SitesPage />
      </Wrapper>,
    );

    const tag = await waitFor(() => {
      const el = modelTag("gpt-4.1");
      expect(el).toBeTruthy();
      return el as HTMLElement;
    });

    expect(document.querySelector(".ant-card")).toBeNull();
    expect(screen.queryByText("列表没有？手动输入")).toBeNull();
    expect(screen.getByText("（当前模型：gpt-4.1）")).toBeInTheDocument();

    const search = screen.getByPlaceholderText("搜索模型…");
    expect(search.closest(".ant-input-search")).not.toHaveClass("ant-input-search-small");
    expect(search.closest(".ant-input-affix-wrapper")).not.toHaveClass(
      "ant-input-affix-wrapper-sm",
    );

    expect(tag.querySelector("[data-model-tag-close]")).toBeNull();
    expect(tag.querySelector(".ant-checkbox")).toBeNull();
    expect(tag).toHaveClass("model-tag");
    expect(tag).toHaveAttribute("data-selected", "true");
    expect(tag.style.fontSize).toBe("");
    expect(tag.style.paddingBlock).toBe("");

    const addBtn = screen.getByRole("button", { name: "手动添加" });
    const multiBtn = screen.getByRole("button", { name: "多选" });
    const testBtn = screen.getByRole("button", { name: "测试" });
    const clearBtn = screen.getByRole("button", { name: /清\s*空/ });
    expect(addBtn.className).toMatch(/ant-btn-sm/);
    expect(multiBtn.className).toMatch(/ant-btn-sm/);
    expect(testBtn.className).toMatch(/ant-btn-sm/);
    expect(clearBtn.querySelector("svg.lucide")).toBeTruthy();
    expect(addBtn.compareDocumentPosition(screen.getByText("主模型")) & Node.DOCUMENT_POSITION_PRECEDING).toBeTruthy();
    expect(addBtn.compareDocumentPosition(multiBtn) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect(multiBtn.compareDocumentPosition(testBtn) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect(testBtn.compareDocumentPosition(clearBtn) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();

    const applyBtn = screen.getByRole("button", { name: /去 Claude Code 应用|去 Codex 应用/ });
    expect(applyBtn.className).toMatch(/ant-btn-sm/);
    const detailName = document.querySelector(".text-base.font-medium");
    expect(detailName?.textContent).toBe("Relay One");
    expect(applyBtn.compareDocumentPosition(detailName as Node) & Node.DOCUMENT_POSITION_PRECEDING).toBeTruthy();
  });

  it("opens the test-models modal from the toolbar", async () => {
    await act(async () => {
      await seedSite();
    });

    render(
      <Wrapper>
        <SitesPage />
      </Wrapper>,
    );

    await waitFor(() => {
      expect(screen.getByRole("button", { name: "测试" })).toBeInTheDocument();
    });
    fireEvent.click(screen.getByRole("button", { name: "测试" }));

    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getByText("测试模型")).toBeInTheDocument();
    expect(within(dialog).getByRole("checkbox", { name: "全选" })).toBeChecked();
    expect(within(dialog).getByRole("button", { name: /立\s*即\s*测\s*试/ })).toBeInTheDocument();
    expect(within(dialog).getByRole("button", { name: /取\s*消/ })).toBeInTheDocument();
  });

  it("opens a modal to add a model manually", async () => {
    await act(async () => {
      await seedSite();
    });

    render(
      <Wrapper>
        <SitesPage />
      </Wrapper>,
    );

    await waitFor(() => {
      expect(screen.getByRole("button", { name: "手动添加" })).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole("button", { name: "手动添加" }));

    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getByText("手动添加模型")).toBeInTheDocument();

    fireEvent.change(within(dialog).getByPlaceholderText("输入 model id"), {
      target: { value: "gpt-5.6-terra" },
    });
    fireEvent.click(within(dialog).getByRole("button", { name: /保\s*存/ }));

    await waitFor(() => {
      expect(modelTag("gpt-5.6-terra")).toBeTruthy();
    });
    expect(useSiteStore.getState().sites[0]?.selectedModelId).toBe("gpt-5.6-terra");
  });

  it("offers edit and delete on site list context menu", async () => {
    await act(async () => {
      await seedSite();
    });

    render(
      <Wrapper>
        <SitesPage />
      </Wrapper>,
    );

    const item = await screen.findByRole("button", { name: /Relay One/ });
    fireEvent.contextMenu(item);

    expect(await screen.findByRole("menuitem", { name: "编辑站点" })).toBeInTheDocument();
    expect(screen.getByRole("menuitem", { name: "删除站点" })).toBeInTheDocument();
  });

  it("centers an empty-model hint with a fetch button", async () => {
    await act(async () => {
      const site = await useSiteStore.getState().createSite({
        name: "Empty One",
        baseUrl: "https://api.example.com",
        apiKey: "sk-test",
      });
      useUIStore.getState().setSelectedSiteId(site.id);
    });

    render(
      <Wrapper>
        <SitesPage />
      </Wrapper>,
    );

    const hint = await screen.findByText("暂无模型，请先拉取或手动添加");
    expect(hint.parentElement).toHaveClass("justify-center");
    expect(screen.getByRole("button", { name: "测试" })).toBeDisabled();

    const fetchInEmpty = hint.parentElement?.querySelector("button");
    expect(fetchInEmpty).toBeTruthy();
    expect(fetchInEmpty).toHaveTextContent("拉取模型");
    expect(fetchInEmpty?.className).toMatch(/ant-btn-sm/);
    expect(document.querySelector("[data-model-list]")).toBeTruthy();

    fireEvent.click(fetchInEmpty as HTMLButtonElement);
    await waitFor(() => {
      expect(document.querySelector("[data-model-tag]")).toBeTruthy();
    });
    const toasts = await screen.findAllByText("同步模型成功，本次同步 2 个模型");
    expect(toasts).toHaveLength(1);
    expect(screen.queryByText("成功")).toBeNull();
  });

  it("shows edit and delete from the site row more menu", async () => {
    await act(async () => {
      await seedSite();
    });

    render(
      <Wrapper>
        <SitesPage />
      </Wrapper>,
    );

    const more = await screen.findByRole("button", { name: "更多操作" });
    fireEvent.click(more);

    expect(await screen.findByRole("menuitem", { name: "编辑站点" })).toBeInTheDocument();
    expect(screen.getByRole("menuitem", { name: "删除站点" })).toBeInTheDocument();
  });

  it("clears all models from the header button", async () => {
    await act(async () => {
      await seedSite();
    });

    render(
      <Wrapper>
        <SitesPage />
      </Wrapper>,
    );

    await waitFor(() => {
      expect(modelTag("gpt-4.1")).toBeTruthy();
    });

    const addBtn = screen.getByRole("button", { name: "手动添加" });
    const clearBtn = screen.getByRole("button", { name: /清\s*空/ });
    expect(clearBtn.className).toMatch(/ant-btn-sm/);
    expect(addBtn.compareDocumentPosition(clearBtn) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();

    fireEvent.click(clearBtn);
    const dialog = await screen.findByRole("dialog");
    fireEvent.click(within(dialog).getByRole("button", { name: /确\s*认/ }));

    await waitFor(() => {
      expect(document.querySelector("[data-model-tag]")).toBeNull();
    });
    expect(await screen.findByText("清空模型成功")).toBeInTheDocument();
    expect(screen.queryByText("成功")).toBeNull();
    expect(useSiteStore.getState().modelsBySite[useUIStore.getState().selectedSiteId ?? ""]).toEqual(
      [],
    );
  });

  it("opens the route dropdown and can switch the active base url", async () => {
    await act(async () => {
      const created = await useSiteStore.getState().createSite({
        name: "Relay One",
        baseUrl: "https://api.example.com",
        apiKey: "sk-test",
      });
      await useSiteStore.getState().updateSite(created.id, {
        baseUrls: ["https://api.example.com", "https://api2.example.com"],
      });
      useUIStore.getState().setSelectedSiteId(created.id);
    });

    render(
      <Wrapper>
        <SitesPage />
      </Wrapper>,
    );

    const trigger = await screen.findByRole("button", { name: "切换线路" });
    fireEvent.click(trigger);

    expect(await screen.findByText("https://api2.example.com")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /测\s*速/ })).toBeInTheDocument();

    fireEvent.click(screen.getByText("https://api2.example.com"));
    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getAllByText("切换线路？").length).toBeGreaterThan(0);
    expect(within(dialog).getByRole("button", { name: /取\s*消/ })).toBeInTheDocument();
    expect(within(dialog).getByRole("button", { name: "跳过应用" })).toBeInTheDocument();
    const option = screen.getByText("https://api2.example.com").closest("button");
    expect(option).toHaveClass("route-option");
    expect(option).toHaveClass("cursor-pointer");
    expect(useSiteStore.getState().sites[0]?.baseUrl).toBe("https://api.example.com");

    fireEvent.click(within(dialog).getByRole("button", { name: /确\s*认/ }));
    await waitFor(() => {
      expect(useSiteStore.getState().sites[0]?.baseUrl).toBe("https://api2.example.com");
    });
  });

  it("can switch a route without applying to target CLIs", async () => {
    await act(async () => {
      const created = await useSiteStore.getState().createSite({
        name: "Relay One",
        baseUrl: "https://api.example.com",
        apiKey: "sk-test",
      });
      await useSiteStore.getState().updateSite(created.id, {
        baseUrls: ["https://api.example.com", "https://api2.example.com"],
      });
      useUIStore.getState().setSelectedSiteId(created.id);
    });

    render(
      <Wrapper>
        <SitesPage />
      </Wrapper>,
    );

    fireEvent.click(await screen.findByRole("button", { name: "切换线路" }));
    fireEvent.click(await screen.findByText("https://api2.example.com"));
    const dialog = await screen.findByRole("dialog");
    fireEvent.click(within(dialog).getByRole("button", { name: "跳过应用" }));
    await waitFor(() => {
      expect(useSiteStore.getState().sites[0]?.baseUrl).toBe("https://api2.example.com");
    });
  });

  it("does not switch the route when the confirm dialog is cancelled", async () => {
    await act(async () => {
      const created = await useSiteStore.getState().createSite({
        name: "Relay One",
        baseUrl: "https://api.example.com",
        apiKey: "sk-test",
      });
      await useSiteStore.getState().updateSite(created.id, {
        baseUrls: ["https://api.example.com", "https://api2.example.com"],
      });
      useUIStore.getState().setSelectedSiteId(created.id);
    });

    render(
      <Wrapper>
        <SitesPage />
      </Wrapper>,
    );

    fireEvent.click(await screen.findByRole("button", { name: "切换线路" }));
    fireEvent.click(await screen.findByText("https://api2.example.com"));
    const dialog = await screen.findByRole("dialog");
    fireEvent.click(within(dialog).getByRole("button", { name: /取\s*消/ }));
    expect(useSiteStore.getState().sites[0]?.baseUrl).toBe("https://api.example.com");
  });

  it("removes a model when the tag close icon is clicked", async () => {
    await act(async () => {
      await seedSite();
    });

    render(
      <Wrapper>
        <SitesPage />
      </Wrapper>,
    );

    await waitFor(() => {
      expect(modelTag("gpt-4.1")).toBeTruthy();
    });

    fireEvent.click(screen.getByRole("button", { name: "多选" }));

    const tag = modelTag("gpt-4.1") as HTMLElement;
    const close = tag.querySelector("[data-model-tag-close]");
    expect(close).toBeTruthy();
    fireEvent.click(close as Element);

    await waitFor(() => {
      expect(modelTag("gpt-4.1")).toBeNull();
    });
    expect(
      useSiteStore.getState().modelsBySite[useUIStore.getState().selectedSiteId ?? ""]?.some(
        (m) => m.modelId === "gpt-4.1",
      ),
    ).toBe(false);
  });

  it("groups model tags by family prefix", async () => {
    await act(async () => {
      await seedSite();
    });

    render(
      <Wrapper>
        <SitesPage />
      </Wrapper>,
    );

    const gptGroup = await waitFor(() => {
      const el = document.querySelector('[data-model-group="gpt"]');
      expect(el).toBeTruthy();
      return el as HTMLElement;
    });
    const claudeGroup = document.querySelector('[data-model-group="claude"]') as HTMLElement;
    expect(claudeGroup).toBeTruthy();

    const gptModelTag = gptGroup.querySelector('[data-model-tag][title="gpt-4.1"]') as HTMLElement;
    const gptCountTag = gptGroup.querySelector("[data-model-count]") as HTMLElement;
    expect(gptModelTag).toBeTruthy();
    expect(claudeGroup.querySelector('[data-model-tag][title="claude-sonnet-4"]')).toBeTruthy();
    expect(gptCountTag).toBeTruthy();
    expect(gptGroup).toHaveTextContent("1 个模型");
    expect(claudeGroup).toHaveTextContent("1 个模型");
    expect(gptModelTag.style.fontSize).toBe("");
    expect(Number.parseFloat(gptCountTag.style.fontSize)).toBeLessThan(12);
    expect(gptGroup.compareDocumentPosition(claudeGroup) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  });

  it("enables checkboxes and a floating delete bar in multi-select", async () => {
    await act(async () => {
      await seedSite();
    });

    render(
      <Wrapper>
        <SitesPage />
      </Wrapper>,
    );

    const gptTag = await waitFor(() => {
      const el = modelTag("gpt-4.1");
      expect(el).toBeTruthy();
      return el as HTMLElement;
    });
    const primaryBefore = useSiteStore.getState().sites[0]?.selectedModelId;

    fireEvent.click(screen.getByRole("button", { name: "多选" }));
    expect(screen.getByRole("button", { name: "完成" })).toBeInTheDocument();
    expect(gptTag.querySelector(".ant-checkbox")).toBeTruthy();
    const close = gptTag.querySelector("[data-model-tag-close]");
    expect(close).toBeTruthy();
    expect(close).toHaveClass("model-tag-close");
    expect(document.querySelector("[data-model-multi-actions]")).toBeNull();

    fireEvent.click(gptTag);
    const bar = await waitFor(() => {
      const el = document.querySelector("[data-model-multi-actions]");
      expect(el).toBeTruthy();
      return el as HTMLElement;
    });
    expect(bar).toHaveTextContent("已选 1 项");
    expect(bar).toHaveTextContent("删除 1 项");
    expect(useSiteStore.getState().sites[0]?.selectedModelId).toBe(primaryBefore);

    const claudeTag = modelTag("claude-sonnet-4") as HTMLElement;
    const claudeBox = claudeTag.querySelector("input[type='checkbox']") as HTMLInputElement;
    expect(claudeBox).toBeTruthy();
    fireEvent.click(claudeBox);
    expect(bar).toHaveTextContent("已选 2 项");
    expect(bar).toHaveTextContent("删除 2 项");

    fireEvent.click(within(bar).getByRole("button", { name: /删除/ }));
    const dialog = await screen.findByRole("dialog");
    fireEvent.click(within(dialog).getByRole("button", { name: /确\s*认/ }));

    await waitFor(() => {
      expect(modelTag("gpt-4.1")).toBeNull();
      expect(modelTag("claude-sonnet-4")).toBeNull();
    });
    expect(await screen.findByText("已删除 2 个模型")).toBeInTheDocument();
    expect(useSiteStore.getState().modelsBySite[useUIStore.getState().selectedSiteId ?? ""]).toEqual(
      [],
    );
  });

  it("exits multi-select and hides tag controls", async () => {
    await act(async () => {
      await seedSite();
    });

    render(
      <Wrapper>
        <SitesPage />
      </Wrapper>,
    );

    const tag = await waitFor(() => {
      const el = modelTag("gpt-4.1");
      expect(el).toBeTruthy();
      return el as HTMLElement;
    });

    fireEvent.click(screen.getByRole("button", { name: "多选" }));
    fireEvent.click(tag);
    expect(document.querySelector("[data-model-multi-actions]")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "完成" }));
    expect(screen.getByRole("button", { name: "多选" })).toBeInTheDocument();
    expect(tag.querySelector(".ant-checkbox")).toBeNull();
    expect(tag.querySelector("[data-model-tag-close]")).toBeNull();
    expect(document.querySelector("[data-model-multi-actions]")).toBeNull();
  });

  it("shows 停用 on the enable switch when the site is off", async () => {
    await act(async () => {
      const site = await seedSite();
      await useSiteStore.getState().updateSite(site.id, { enabled: false });
    });

    render(
      <Wrapper>
        <SitesPage />
      </Wrapper>,
    );

    const sw = await screen.findByRole("switch");
    expect(sw).not.toBeChecked();
    expect(sw).toHaveTextContent("停用");
  });

  it("shows a pulsing status dot for enabled sites and a gray one when disabled", async () => {
    await act(async () => {
      await seedSite();
    });

    render(
      <Wrapper>
        <SitesPage />
      </Wrapper>,
    );

    const item = await screen.findByRole("button", { name: /Relay One/ });
    expect(item.querySelector("[data-status='active']")).toBeTruthy();
    expect(item.querySelector(".ant-badge-status-processing")).toBeTruthy();

    await act(async () => {
      const site = useSiteStore.getState().sites[0];
      if (site) await useSiteStore.getState().updateSite(site.id, { enabled: false });
    });

    expect(item.querySelector("[data-status='inactive']")).toBeTruthy();
    expect(item.querySelector(".ant-badge-status-default")).toBeTruthy();
  });

  it("disables a site immediately when no target is using it", async () => {
    await act(async () => {
      await seedSite();
    });

    render(
      <Wrapper>
        <SitesPage />
      </Wrapper>,
    );

    const sw = await screen.findByRole("switch");
    expect(sw).toBeChecked();
    fireEvent.click(sw);

    await waitFor(() => {
      expect(useSiteStore.getState().sites[0]?.enabled).toBe(false);
    });
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("asks before disabling a site that is applied to a tool", async () => {
    await act(async () => {
      const site = await seedSite();
      seedTargetStatuses([
        appliedStatus("claude_code", site.id, site.name),
        appliedStatus("codex", null, null),
        appliedStatus("pi", site.id, site.name),
      ]);
    });

    render(
      <Wrapper>
        <SitesPage />
      </Wrapper>,
    );

    fireEvent.click(await screen.findByRole("switch"));
    const dialog = await screen.findByRole("dialog");
    expect(dialog).toHaveTextContent("停用站点");
    expect(dialog).toHaveTextContent("Claude Code");
    expect(dialog).toHaveTextContent("Pi");
    expect(within(dialog).getByRole("button", { name: /取\s*消/ })).toBeInTheDocument();
    expect(within(dialog).getByRole("button", { name: /跳\s*过/ })).toBeInTheDocument();
    expect(within(dialog).getByRole("button", { name: /清\s*除/ })).toBeInTheDocument();
    expect(useSiteStore.getState().sites[0]?.enabled).toBe(true);

    fireEvent.click(within(dialog).getByRole("button", { name: /取\s*消/ }));
    expect(useSiteStore.getState().sites[0]?.enabled).toBe(true);
  });

  it("can skip clearing and only disable the site", async () => {
    await act(async () => {
      const site = await seedSite();
      seedTargetStatuses([
        appliedStatus("claude_code", site.id, site.name),
        appliedStatus("codex", null, null),
      ]);
    });

    render(
      <Wrapper>
        <SitesPage />
      </Wrapper>,
    );

    fireEvent.click(await screen.findByRole("switch"));
    const dialog = await screen.findByRole("dialog");
    fireEvent.click(within(dialog).getByRole("button", { name: /跳\s*过/ }));

    await waitFor(() => {
      expect(useSiteStore.getState().sites[0]?.enabled).toBe(false);
    });
    expect(useApplyStore.getState().statuses.find((s) => s.kind === "claude_code")?.status).toBe(
      "applied",
    );
  });

  it("clears applied tool config when confirming disable", async () => {
    await act(async () => {
      const site = await seedSite();
      seedTargetStatuses([
        appliedStatus("claude_code", site.id, site.name),
        appliedStatus("codex", site.id, site.name),
        appliedStatus("pi", site.id, site.name),
      ]);
    });

    render(
      <Wrapper>
        <SitesPage />
      </Wrapper>,
    );

    fireEvent.click(await screen.findByRole("switch"));
    const dialog = await screen.findByRole("dialog");
    fireEvent.click(within(dialog).getByRole("button", { name: /清\s*除/ }));

    await waitFor(() => {
      expect(useSiteStore.getState().sites[0]?.enabled).toBe(false);
    });
    expect(useApplyStore.getState().statuses.every((s) => s.status === "not_applied")).toBe(true);
  });

  it("shows quota in site details when the probe returns a balance", async () => {
    await act(async () => {
      await seedSite();
    });

    render(
      <Wrapper>
        <SitesPage />
      </Wrapper>,
    );

    await waitFor(() => {
      expect(screen.getByTestId("site-quota-row")).toBeInTheDocument();
    });
    expect(screen.getByText("剩余 $87.50")).toBeInTheDocument();
    expect(document.querySelector(".ant-card")).toBeNull();
  });

  it("hides quota when the upstream does not implement billing", async () => {
    await act(async () => {
      const site = await useSiteStore.getState().createSite({
        name: "No Quota",
        baseUrl: "https://no-quota.example.com",
        apiKey: "sk-test",
      });
      await useSiteStore.getState().fetchModels(site.id);
      useUIStore.getState().setSelectedSiteId(site.id);
    });

    render(
      <Wrapper>
        <SitesPage />
      </Wrapper>,
    );

    await waitFor(() => {
      expect(modelTag("gpt-4.1")).toBeTruthy();
    });
    await waitFor(() => {
      expect(useSiteStore.getState().quotaBySite[useUIStore.getState().selectedSiteId ?? ""]?.status).toBe(
        "unsupported",
      );
    });
    expect(screen.queryByTestId("site-quota-row")).toBeNull();
  });
});

function appliedStatus(
  kind: TargetLiveStatus["kind"],
  siteId: string | null,
  siteName: string | null,
): TargetLiveStatus {
  return {
    kind,
    installed: true,
    version: "1.0.0",
    configPath: kind === "claude_code" ? "~/.claude/settings.json" : "~/.codex/config.toml",
    status: siteId ? "applied" : "not_applied",
    appliedSiteId: siteId,
    appliedSiteName: siteName,
    appliedModelId: siteId ? "gpt-4.1" : null,
    providerId: null,
    orphan: false,
    liveSummary: {},
    lastAppliedAt: siteId ? 1 : null,
    staleReason: null,
  };
}
