import { fireEvent, render, screen } from "@testing-library/react";
import { App as AntdApp, ConfigProvider } from "antd";
import type { ReactNode } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { SiteQuota } from "@/types/domain";
import { SiteQuotaRow } from "./SiteQuotaRow";
import i18n from "@/i18n";

function Wrapper({ children }: { children: ReactNode }) {
  return (
    <ConfigProvider>
      <AntdApp>{children}</AntdApp>
    </ConfigProvider>
  );
}

function quota(partial: Partial<SiteQuota> = {}): SiteQuota {
  return {
    status: "available",
    remainingUsd: 87.5,
    usedUsd: 12.5,
    totalUsd: 100,
    unlimited: false,
    unit: "USD",
    expiresAt: Math.floor(Date.UTC(2026, 11, 31) / 1000),
    source: "credit_grants",
    endpoint: "https://api.example.com/v1/dashboard/billing/credit_grants",
    fetchedAt: Date.now(),
    latencyMs: 12,
    error: null,
    ...partial,
  };
}

describe("SiteQuotaRow", () => {
  afterEach(async () => {
    await i18n.changeLanguage("zh-CN");
  });

  it("renders remaining, used/total, and a refresh control when available", () => {
    const onRefresh = vi.fn();
    render(
      <Wrapper>
        <SiteQuotaRow quota={quota()} loading={false} onRefresh={onRefresh} />
      </Wrapper>,
    );

    expect(screen.getByTestId("site-quota-row")).toBeInTheDocument();
    expect(screen.getByText("额度")).toBeInTheDocument();
    expect(screen.getByText("剩余 $87.50")).toBeInTheDocument();
    expect(screen.getByText("$12.50 / $100.00")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "刷新额度" }));
    expect(onRefresh).toHaveBeenCalledTimes(1);
  });

  it("shows an actionable status when automatic quota lookup is unsupported", () => {
    const onRefresh = vi.fn();
    render(
      <Wrapper>
        <SiteQuotaRow
          quota={null}
          attempt={quota({
            status: "unsupported",
            remainingUsd: null,
            usedUsd: null,
            totalUsd: null,
          })}
          loading={false}
          onRefresh={onRefresh}
        />
      </Wrapper>,
    );

    expect(screen.getByTestId("site-quota-status")).toBeInTheDocument();
    expect(screen.getByText("此站点不支持自动获取额度")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "刷新额度" }));
    expect(onRefresh).toHaveBeenCalledTimes(1);
  });

  it.each([
    ["unauthorized", "额度鉴权失败，请检查 API Key"],
    ["error", "额度刷新失败，请稍后重试"],
    ["invalid_data", "额度数据异常，无法可靠显示"],
  ])("shows an actionable status for a %s quota attempt", (status, expected) => {
    render(
      <Wrapper>
        <SiteQuotaRow
          quota={null}
          attempt={quota({
            status: status as SiteQuota["status"],
            remainingUsd: null,
            usedUsd: null,
            totalUsd: null,
            error: "technical detail",
          })}
          loading={false}
          onRefresh={() => undefined}
        />
      </Wrapper>,
    );

    expect(screen.getByTestId("site-quota-status")).toBeInTheDocument();
    expect(screen.getByText(expected)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "刷新额度" })).toBeEnabled();
  });

  it.each([
    ["request timed out", "额度请求超时，请稍后重试"],
    ["HTTP 502", "额度上游服务异常，请稍后重试"],
  ])("distinguishes the %s quota failure", (error, expected) => {
    render(
      <Wrapper>
        <SiteQuotaRow
          quota={null}
          attempt={quota({
            status: "error",
            remainingUsd: null,
            usedUsd: null,
            totalUsd: null,
            error,
          })}
          loading={false}
          onRefresh={() => undefined}
        />
      </Wrapper>,
    );

    expect(screen.getByText(expected)).toBeInTheDocument();
  });

  it.each([
    ["unsupported", null, "This site does not support automatic quota lookup"],
    ["unauthorized", null, "Quota authentication failed; check the API key"],
    ["error", "request timed out", "Quota request timed out; try again"],
    ["invalid_data", null, "Quota data is invalid and cannot be displayed reliably"],
  ])("renders an actionable %s error in English", async (status, error, expected) => {
    await i18n.changeLanguage("en-US");
    render(
      <Wrapper>
        <SiteQuotaRow
          quota={null}
          attempt={quota({
            status: status as SiteQuota["status"],
            remainingUsd: null,
            usedUsd: null,
            totalUsd: null,
            error,
          })}
          loading={false}
          onRefresh={() => undefined}
        />
      </Wrapper>,
    );

    expect(screen.getByText(expected)).toBeInTheDocument();
    expect(screen.getByRole("status")).toBeInTheDocument();
  });

  it("shows a one-line skeleton while the first probe is in flight", () => {
    render(
      <Wrapper>
        <SiteQuotaRow quota={null} loading onRefresh={() => undefined} />
      </Wrapper>,
    );
    expect(screen.getByTestId("site-quota-loading")).toBeInTheDocument();
    expect(screen.getByText("额度")).toBeInTheDocument();
  });

  it("renders CNY remaining from token usage display", () => {
    render(
      <Wrapper>
        <SiteQuotaRow
          quota={quota({
            remainingUsd: 999.693074,
            usedUsd: 0.306926,
            totalUsd: 1000,
            unit: "CNY",
            source: "token_usage",
          })}
          loading={false}
          onRefresh={() => undefined}
        />
      </Wrapper>,
    );
    expect(screen.getByText("剩余 ¥999.69")).toBeInTheDocument();
    expect(screen.getByText("¥0.31 / ¥1,000.00")).toBeInTheDocument();
  });

  it("localizes raw quota units as quota credits", () => {
    render(
      <Wrapper>
        <SiteQuotaRow
          quota={quota({ unit: "RAW_QUOTA" })}
          loading={false}
          onRefresh={() => undefined}
        />
      </Wrapper>,
    );

    expect(screen.getByText("剩余 87.50 额度点数")).toBeInTheDocument();
    expect(screen.getByText("12.50 额度点数 / 100.00 额度点数")).toBeInTheDocument();
  });

  it("keeps the last successful quota visible when the latest refresh fails", () => {
    render(
      <Wrapper>
        <SiteQuotaRow
          quota={quota()}
          attempt={quota({
            status: "error",
            remainingUsd: null,
            usedUsd: null,
            totalUsd: null,
            error: "upstream unavailable",
          })}
          loading={false}
          onRefresh={() => undefined}
        />
      </Wrapper>,
    );

    expect(screen.getByText("剩余 $87.50")).toBeInTheDocument();
    expect(screen.getByText("上次成功数据，刷新失败")).toBeInTheDocument();
  });

  it("shows unlimited copy without a progress bar", () => {
    render(
      <Wrapper>
        <SiteQuotaRow
          quota={quota({
            unlimited: true,
            remainingUsd: null,
            totalUsd: null,
            usedUsd: 3,
          })}
          loading={false}
          onRefresh={() => undefined}
        />
      </Wrapper>,
    );
    expect(screen.getByText("此 Key 不限额")).toBeInTheDocument();
    expect(screen.getByText("累计已用 $3.00")).toBeInTheDocument();
    expect(screen.getByText("账户余额未知")).toBeInTheDocument();
    expect(document.querySelector(".ant-progress")).toBeNull();
  });

  it("does not label a finite usage-only result as unlimited", () => {
    render(
      <Wrapper>
        <SiteQuotaRow
          quota={quota({
            remainingUsd: null,
            usedUsd: 25,
            totalUsd: null,
            unlimited: false,
            source: "usage_only",
          })}
          loading={false}
          onRefresh={() => undefined}
        />
      </Wrapper>,
    );

    expect(screen.getByText("已用 $25.00")).toBeInTheDocument();
    expect(screen.queryByText("此 Key 不限额")).toBeNull();
  });

  it("labels an amount-less finite result as unknown instead of unlimited", () => {
    render(
      <Wrapper>
        <SiteQuotaRow
          quota={quota({
            remainingUsd: null,
            usedUsd: null,
            totalUsd: null,
            unlimited: false,
          })}
          loading={false}
          onRefresh={() => undefined}
        />
      </Wrapper>,
    );

    expect(screen.getByText("额度未知")).toBeInTheDocument();
    expect(screen.queryByText("此 Key 不限额")).toBeNull();
  });

  it("renders OpenCode Go usage windows with progress, reset countdown, and limits", () => {
    const now = Date.now();
    render(
      <Wrapper>
        <SiteQuotaRow
          quota={quota({
            source: "opencode_go",
            remainingUsd: null,
            usedUsd: null,
            totalUsd: null,
            unit: "USD",
            windows: [
              { kind: "rolling", usagePercent: 12.5, resetAt: now + 3 * 3600_000, limitUsd: 12 },
              { kind: "weekly", usagePercent: 46.2, resetAt: now + 3 * 24 * 3600_000, limitUsd: 30 },
              { kind: "monthly", usagePercent: 8.4, resetAt: now + 20 * 24 * 3600_000, limitUsd: 60 },
            ],
          })}
          loading={false}
          onRefresh={() => undefined}
        />
      </Wrapper>,
    );

    expect(screen.getByTestId("site-quota-windows")).toBeInTheDocument();
    expect(screen.getByTestId("site-quota-window-rolling")).toBeInTheDocument();
    expect(screen.getByText("5 小时")).toBeInTheDocument();
    expect(screen.getByText("本周")).toBeInTheDocument();
    expect(screen.getByText("本月")).toBeInTheDocument();
    expect(screen.getByText("13%")).toBeInTheDocument();
    expect(screen.getByText("46%")).toBeInTheDocument();
    expect(screen.getByText("8%")).toBeInTheDocument();
    expect(screen.getByText(/上限 \$12\.00/)).toBeInTheDocument();
    expect(screen.getAllByText(/后重置/).length).toBe(3);
  });

  it("marks an elapsed OpenCode Go window as resetting soon", () => {
    render(
      <Wrapper>
        <SiteQuotaRow
          quota={quota({
            source: "opencode_go",
            remainingUsd: null,
            usedUsd: null,
            totalUsd: null,
            unit: "USD",
            windows: [
              { kind: "rolling", usagePercent: 99, resetAt: Date.now() - 5_000, limitUsd: null },
            ],
          })}
          loading={false}
          onRefresh={() => undefined}
        />
      </Wrapper>,
    );

    expect(screen.getByText("即将重置")).toBeInTheDocument();
  });
});
