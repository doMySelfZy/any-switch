import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { DndContext } from "@dnd-kit/core";
import { SortableContext, verticalListSortingStrategy } from "@dnd-kit/sortable";
import { App as AntdApp, ConfigProvider, theme } from "antd";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { resetBrowserMock } from "@/lib/browserMock";
import { useSiteStore } from "@/stores";
import { resetQuotaInflight } from "@/stores/siteStore";
import type { QuotaWindow, Site, SiteQuota } from "@/types/domain";
import { SiteListItem } from "./SiteListItem";
import "@/i18n";

let token: Record<string, string> = {};

function CaptureToken() {
  const { token: antdToken } = theme.useToken();
  token = antdToken as unknown as Record<string, string>;
  return null;
}

function Harness({ site, onSelect = () => {} }: { site: Site; onSelect?: () => void }) {
  return (
    <ConfigProvider>
      <AntdApp>
        <CaptureToken />
        <DndContext>
          <SortableContext items={[site.id]} strategy={verticalListSortingStrategy}>
            <SiteListItem
              site={site}
              active={false}
              onSelect={onSelect}
              onEdit={() => {}}
              onDelete={() => {}}
            />
          </SortableContext>
        </DndContext>
      </AntdApp>
    </ConfigProvider>
  );
}

async function seedSite(name = "Relay One", baseUrl = "https://api.example.com"): Promise<Site> {
  let site: Site | null = null;
  await act(async () => {
    site = await useSiteStore.getState().createSite({ name, baseUrl, apiKey: "sk-test" });
  });
  if (!site) throw new Error("site was not created");
  return site;
}

function quota(partial: Partial<SiteQuota>): SiteQuota {
  return {
    status: "available",
    remainingUsd: null,
    usedUsd: null,
    totalUsd: null,
    unlimited: false,
    unit: "USD",
    expiresAt: null,
    source: "credit_grants",
    endpoint: null,
    fetchedAt: Date.now(),
    latencyMs: 10,
    error: null,
    ...partial,
  };
}

function window(kind: string, usagePercent: number | null): QuotaWindow {
  return { kind, usagePercent, resetAt: Date.now() + 3600_000, limitUsd: null };
}

function seedQuota(site: Site, value: SiteQuota) {
  act(() => {
    useSiteStore.setState({ quotaBySite: { [site.id]: value } });
  });
}

function renderListItem(site: Site, onSelect?: () => void) {
  return render(<Harness site={site} onSelect={onSelect} />);
}

/** jsdom serializes colors as rgb()/rgba(); normalize token hex/rgba for comparison. */
function toRgb(color: string): string {
  const value = color.replace(/\s+/g, "");
  if (value.startsWith("#")) {
    const hex = value.slice(1);
    const full = hex.length === 3 ? hex.split("").map((c) => c + c).join("") : hex;
    const r = Number.parseInt(full.slice(0, 2), 16);
    const g = Number.parseInt(full.slice(2, 4), 16);
    const b = Number.parseInt(full.slice(4, 6), 16);
    return `rgb(${r},${g},${b})`;
  }
  return value;
}

async function waitForWindowSummaryColor(kind: string, tokenColor: string) {
  await waitFor(() => {
    const span = screen.getByTestId(`site-quota-window-summary-${kind}`);
    expect(toRgb(span.style.color)).toBe(toRgb(tokenColor));
  });
}

describe("SiteListItem quota summary", () => {
  beforeEach(() => {
    resetBrowserMock();
    resetQuotaInflight();
    useSiteStore.setState({
      sites: [],
      modelsBySite: {},
      modelsLoadingBySite: {},
      quotaBySite: {},
      quotaAttemptBySite: {},
      quotaCacheKeyBySite: {},
      quotaAttemptCacheKeyBySite: {},
      quotaLoadingBySite: {},
      loading: false,
      hydrated: false,
      fetchingModels: false,
      error: null,
    });
  });

  afterEach(() => {
    resetBrowserMock();
  });

  it("renders no summary before quota data or for unsupported / error / unlimited sites", async () => {
    const site = await seedSite();
    renderListItem(site);
    // Flush the async SiteAvatar icon resolution so its setState stays in act.
    await act(async () => {});

    // No quota data yet.
    expect(screen.queryByTestId("site-quota-summary")).toBeNull();

    // Unsupported / error / unlimited all stay quiet.
    seedQuota(site, quota({ status: "unsupported" }));
    expect(screen.queryByTestId("site-quota-summary")).toBeNull();

    seedQuota(site, quota({ status: "error", error: "boom" }));
    expect(screen.queryByTestId("site-quota-summary")).toBeNull();

    seedQuota(site, quota({ unlimited: true, remainingUsd: null }));
    expect(screen.queryByTestId("site-quota-summary")).toBeNull();
  });

  it("shows a neutral balance summary with amount and update time in the tooltip", async () => {
    const site = await seedSite();
    seedQuota(site, quota({ remainingUsd: 12.34, usedUsd: 88.5, totalUsd: 100, unit: "USD" }));
    const { unmount } = renderListItem(site);

    const summary = await screen.findByTestId("site-quota-summary");
    expect(summary).toHaveTextContent("剩余 $12.34");
    expect(toRgb(summary.style.color)).toBe(toRgb(token.colorTextTertiary));

    // 有进度条就摆出已用，分母（剩余 + 已用）才可还原。
    fireEvent.mouseEnter(summary);
    expect(await screen.findByText("已用 $88.50")).toBeInTheDocument();
    expect(screen.getByText("刚刚更新")).toBeInTheDocument();
    unmount();
  });

  it("shows every window as a remaining percent and colorizes by remaining thresholds", async () => {
    const site = await seedSite();
    seedQuota(
      site,
      quota({
        remainingUsd: null,
        source: "opencode_go",
        windows: [window("rolling", 83), window("weekly", 46), window("monthly", 8)],
      }),
    );
    const { unmount } = renderListItem(site);

    // 三个窗口全部展示，且都是「剩余 = 100 - 已用」口径。
    const summary = await screen.findByTestId("site-quota-summary");
    expect(summary).toHaveTextContent("5h 17%");
    expect(summary).toHaveTextContent("周 54%");
    expect(summary).toHaveTextContent("月 92%");

    // 已用 83% → 剩余 17%，落在 ≤20% 的告警档。
    const rolling = screen.getByTestId("site-quota-window-summary-rolling");
    expect(toRgb(rolling.style.color)).toBe(toRgb(token.colorWarning));
    // 其余两档剩余充足，保持中性灰。
    expect(toRgb(screen.getByTestId("site-quota-window-summary-weekly").style.color)).toBe(
      toRgb(token.colorTextTertiary),
    );
    expect(toRgb(screen.getByTestId("site-quota-window-summary-monthly").style.color)).toBe(
      toRgb(token.colorTextTertiary),
    );

    // Tooltip 同样是剩余口径，并列出所有窗口与更新时间。
    fireEvent.mouseEnter(summary);
    expect(await screen.findByText("5 小时 剩余 17%")).toBeInTheDocument();
    expect(screen.getByText("本周 剩余 54%")).toBeInTheDocument();
    expect(screen.getByText("本月 剩余 92%")).toBeInTheDocument();
    expect(screen.getByText("刚刚更新")).toBeInTheDocument();
    fireEvent.mouseLeave(summary);

    // 已用 95% → 剩余 5%，进入告警红。
    seedQuota(
      site,
      quota({
        remainingUsd: null,
        source: "opencode_go",
        windows: [window("rolling", 95), window("weekly", 46), window("monthly", 8)],
      }),
    );
    await waitForWindowSummaryColor("rolling", token.colorError);

    // 已用 50% → 剩余 50%，保持中性灰。
    seedQuota(
      site,
      quota({
        remainingUsd: null,
        source: "opencode_go",
        windows: [window("rolling", 50), window("weekly", 46), window("monthly", 8)],
      }),
    );
    await waitForWindowSummaryColor("rolling", token.colorTextTertiary);
    unmount();
  });

  it("keeps selecting the site when the summary itself is clicked", async () => {
    const site = await seedSite();
    seedQuota(site, quota({ remainingUsd: 87.5, unit: "USD" }));
    const onSelect = vi.fn();
    const { unmount } = renderListItem(site, onSelect);

    fireEvent.click(await screen.findByTestId("site-quota-summary"));
    expect(onSelect).toHaveBeenCalledTimes(1);
    unmount();
  });
});
