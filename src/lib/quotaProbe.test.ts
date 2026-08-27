import { describe, expect, it } from "vitest";
import type { SiteQuota } from "@/types/domain";
import {
  formatQuotaAmountParts,
  formatQuotaAmount,
  formatUsd,
  isQuotaCacheFresh,
  QUOTA_TTL_MS,
  quotaCacheKey,
  quotaRemainingPercent,
  quotaTone,
  shouldShowExpiry,
} from "./quotaProbe";

function quota(partial: Partial<SiteQuota>): SiteQuota {
  return {
    status: "available",
    remainingUsd: 87.5,
    usedUsd: 12.5,
    totalUsd: 100,
    unlimited: false,
    expiresAt: null,
    source: "credit_grants",
    endpoint: "https://api.example.com/v1/dashboard/billing/credit_grants",
    fetchedAt: 1,
    latencyMs: 10,
    error: null,
    ...partial,
  };
}

describe("quotaProbe helpers", () => {
  it("formats USD with a dollar sign and two decimals", () => {
    expect(formatUsd(12.5)).toBe("$12.50");
    expect(formatUsd(0)).toBe("$0.00");
  });

  it("formats CNY display amounts from token usage", () => {
    expect(formatQuotaAmount(999.693074, "CNY")).toBe("¥999.69");
    expect(formatQuotaAmount(1000, "cny")).toBe("¥1,000.00");
  });

  it("returns a localizable unit key for raw quota amounts", () => {
    expect(formatQuotaAmountParts(108_886_337, "quota")).toEqual({
      value: "108,886,337.00",
      unit: null,
      unitI18nKey: "sites.quotaUnitRaw",
    });
    expect(formatQuotaAmountParts(24_035, "RAW_QUOTA")).toEqual({
      value: "24,035.00",
      unit: null,
      unitI18nKey: "sites.quotaUnitRaw",
    });
  });

  it("builds a cache key only from quota-relevant site configuration", () => {
    expect(
      quotaCacheKey({
        id: "site-1",
        baseUrl: "https://api.example.com",
        quotaRevision: "credential-2",
      }),
    ).toBe("site-1:https://api.example.com:credential-2");
  });

  it("treats quota snapshots as fresh inside the TTL window", () => {
    const now = 1_000_000;
    expect(isQuotaCacheFresh(quota({ fetchedAt: now - QUOTA_TTL_MS + 1 }), now)).toBe(true);
    expect(isQuotaCacheFresh(quota({ fetchedAt: now - QUOTA_TTL_MS }), now)).toBe(false);
  });

  it("hides expiry that is missing, past, or far-future", () => {
    const now = 1_700_000_000;
    expect(shouldShowExpiry(null, now)).toBe(false);
    expect(shouldShowExpiry(now - 10, now)).toBe(false);
    expect(shouldShowExpiry(now + 86_400, now)).toBe(true);
    expect(shouldShowExpiry(Date.UTC(2099, 0, 1) / 1000, now)).toBe(false);
  });

  it("computes remaining percent and tone", () => {
    expect(quotaRemainingPercent(quota({ remainingUsd: 87.5, totalUsd: 100 }))).toBe(87.5);
    expect(quotaTone(quota({ remainingUsd: 87.5, totalUsd: 100 }))).toBe("ok");
    expect(quotaTone(quota({ remainingUsd: 5, totalUsd: 100 }))).toBe("warn");
    expect(quotaTone(quota({ remainingUsd: 0.4, totalUsd: 100 }))).toBe("warn");
    expect(quotaTone(quota({ remainingUsd: 0, totalUsd: 100 }))).toBe("danger");
    expect(quotaRemainingPercent(quota({ unlimited: true, totalUsd: null }))).toBeNull();
  });
});
