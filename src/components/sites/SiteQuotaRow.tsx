import { Button, Progress, Skeleton, Tooltip, theme } from "antd";
import { RefreshCw } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { SiteQuota } from "@/types/domain";
import {
  formatExpiryDate,
  formatQuotaAmountParts,
  quotaRemainingPercent,
  quotaTone,
  shouldShowExpiry,
} from "@/lib/quotaProbe";

interface Props {
  quota: SiteQuota | null;
  attempt?: SiteQuota | null;
  loading: boolean;
  refreshing?: boolean;
  onRefresh: () => void;
  /** 标准额度探测失败且未配置访问令牌时，显示"配置访问令牌"入口。 */
  showConfigureHint?: boolean;
  onConfigureQuota?: () => void;
}

const quotaStatusMessageKeys: Record<string, string> = {
  unsupported: "sites.quotaUnsupported",
  unauthorized: "sites.quotaUnauthorized",
  error: "sites.quotaError",
  invalid_data: "sites.quotaInvalidData",
};

const quotaWindowLabelKeys: Record<string, string> = {
  rolling: "sites.quotaWindowRolling",
  weekly: "sites.quotaWindowWeekly",
  monthly: "sites.quotaWindowMonthly",
};

function quotaStatusMessageKey(quota: SiteQuota | null): string | null {
  if (!quota) return null;
  if (quota.status === "error" && quota.error === "request timed out") {
    return "sites.quotaTimeout";
  }
  if (quota.status === "error" && /^HTTP 5\d\d$/.test(quota.error ?? "")) {
    return "sites.quotaUpstreamError";
  }
  return quotaStatusMessageKeys[quota.status] ?? null;
}

export function SiteQuotaRow({
  quota,
  attempt,
  loading,
  refreshing,
  onRefresh,
  showConfigureHint,
  onConfigureQuota,
}: Props) {
  const { t, i18n } = useTranslation();
  const { token } = theme.useToken();
  const latestAttempt = attempt ?? quota;

  if (loading && quota?.status !== "available") {
    return (
      <div className="flex gap-2" data-testid="site-quota-loading">
        <span className="w-28 shrink-0 opacity-50">{t("sites.quota")}</span>
        <Skeleton.Input active size="small" style={{ width: 180, minWidth: 180, height: 18 }} />
      </div>
    );
  }

  const statusMessageKey = quotaStatusMessageKey(latestAttempt);
  if (quota?.status !== "available" && statusMessageKey) {
    return (
      <div
        className="flex gap-2"
        data-testid="site-quota-status"
        role="status"
        aria-live="polite"
      >
        <span className="w-28 shrink-0 opacity-50">{t("sites.quota")}</span>
        <div className="flex min-w-0 flex-1 items-center gap-1.5">
          <span className="min-w-0 text-xs" style={{ color: token.colorTextSecondary }}>
            {t(statusMessageKey)}
          </span>
          <Tooltip title={t("sites.quotaRefresh")}>
            <Button
              type="text"
              size="small"
              loading={Boolean(refreshing)}
              icon={<RefreshCw size={14} />}
              onClick={onRefresh}
              aria-label={t("sites.quotaRefresh")}
            />
          </Tooltip>
          {showConfigureHint && onConfigureQuota && quota != null && quota.status !== "invalid_data" && (
            <Button type="link" size="small" className="!px-1" onClick={onConfigureQuota}>
              {t("sites.quotaConfigureToken")}
            </Button>
          )}
        </div>
      </div>
    );
  }

  if (quota?.status !== "available") return null;

  const formatMoney = (n: number) => {
    const parts = formatQuotaAmountParts(n, quota.unit);
    const unit = parts.unitI18nKey ? t(parts.unitI18nKey) : parts.unit;
    return unit ? `${parts.value} ${unit}` : parts.value;
  };
  const formatReset = (resetAt: number | null): string | null => {
    if (resetAt == null || !Number.isFinite(resetAt)) return null;
    const remaining = resetAt - Date.now();
    if (remaining <= 60_000) return t("sites.quotaWindowResetsSoon");
    const totalMinutes = Math.ceil(remaining / 60_000);
    const hours = Math.floor(totalMinutes / 60);
    const minutes = totalMinutes % 60;
    const parts: string[] = [];
    if (hours > 0) parts.push(t("sites.quotaWindowDurationHours", { hours }));
    if (minutes > 0 || hours === 0) {
      parts.push(t("sites.quotaWindowDurationMinutes", { minutes: Math.max(minutes, 1) }));
    }
    return t("sites.quotaWindowResetsIn", { time: parts.join(" ") });
  };

  const windows = quota.windows ?? [];
  if (windows.length > 0) {
    const mins = Math.max(0, Math.round((Date.now() - quota.fetchedAt) / 60_000));
    const updated =
      mins < 1
        ? t("sites.quotaUpdatedJustNow")
        : t("sites.quotaUpdated", { time: t("sites.quotaMinutesAgo", { count: mins }) });
    const latestAttemptFailed = latestAttempt?.status !== "available";
    return (
      <div className="flex flex-col gap-1.5" data-testid="site-quota-row">
        <div className="flex gap-2" data-testid="site-quota-windows">
          <span className="w-28 shrink-0 opacity-50">{t("sites.quota")}</span>
          <div className="flex min-w-0 flex-1 items-center gap-1.5">
            <Tooltip title={t("sites.quotaRefresh")}>
              <Button
                type="text"
                size="small"
                loading={Boolean(refreshing)}
                icon={<RefreshCw size={14} />}
                onClick={onRefresh}
                aria-label={t("sites.quotaRefresh")}
              />
            </Tooltip>
            <span className="min-w-0 truncate text-xs opacity-50">{updated}</span>
            {latestAttemptFailed && (
              <span className="text-xs" style={{ color: token.colorWarning }}>
                {t("sites.quotaLastSuccessRefreshFailed")}
              </span>
            )}
          </div>
        </div>
        {windows.map((window) => {
          const labelKey = quotaWindowLabelKeys[window.kind];
          const label = labelKey ? t(labelKey) : window.kind;
          const percent =
            window.usagePercent == null
              ? null
              : Math.max(0, Math.min(100, window.usagePercent));
          const stroke =
            percent == null
              ? token.colorPrimary
              : percent >= 90
                ? token.colorError
                : percent >= 70
                  ? token.colorWarning
                  : token.colorPrimary;
          const details: string[] = [];
          const resetText = formatReset(window.resetAt);
          if (resetText) details.push(resetText);
          if (window.limitUsd != null) {
            details.push(
              t("sites.quotaWindowLimit", { amount: formatMoney(window.limitUsd) }),
            );
          }
          return (
            <div
              className="flex gap-2"
              key={window.kind}
              data-testid={`site-quota-window-${window.kind}`}
            >
              <span className="w-28 shrink-0 text-xs opacity-50">{label}</span>
              <div className="min-w-0 flex-1">
                <div className="flex items-center gap-2">
                  {percent != null ? (
                    <Progress
                      percent={percent}
                      showInfo={false}
                      size="small"
                      strokeColor={stroke}
                      style={{ flex: 1, marginBottom: 0, marginTop: 0 }}
                    />
                  ) : (
                    <span className="flex-1 text-xs opacity-50">—</span>
                  )}
                  <span
                    className="shrink-0 text-xs"
                    style={{ color: token.colorTextSecondary }}
                  >
                    {percent != null ? `${Math.round(percent)}%` : ""}
                  </span>
                </div>
                {details.length > 0 && (
                  <div className="mt-0.5 text-xs opacity-50">{details.join(" · ")}</div>
                )}
              </div>
            </div>
          );
        })}
      </div>
    );
  }

  const tone = quotaTone(quota);
  const percent = quotaRemainingPercent(quota);
  const stroke =
    tone === "danger"
      ? token.colorError
      : tone === "warn"
        ? token.colorWarning
        : token.colorPrimary;

  const money = (n: number) => {
    const parts = formatQuotaAmountParts(n, quota.unit);
    const unit = parts.unitI18nKey ? t(parts.unitI18nKey) : parts.unit;
    return unit ? `${parts.value} ${unit}` : parts.value;
  };
  const primary = quota.unlimited
    ? t("sites.quotaUnlimited")
    : quota.remainingUsd != null
      ? t("sites.quotaRemaining", { amount: money(quota.remainingUsd) })
      : quota.usedUsd != null
        ? t("sites.quotaUsed", { amount: money(quota.usedUsd) })
        : quota.totalUsd != null
          ? money(quota.totalUsd)
          : t("sites.quotaUnknown");

  const secondary: string[] = [];
  if (!quota.unlimited && quota.usedUsd != null && quota.totalUsd != null) {
    secondary.push(
      t("sites.quotaUsedOfTotal", {
        used: money(quota.usedUsd),
        total: money(quota.totalUsd),
      }),
    );
  } else if (quota.unlimited && quota.usedUsd != null) {
    secondary.push(t("sites.quotaCumulativeUsed", { amount: money(quota.usedUsd) }));
  }
  if (quota.unlimited) {
    secondary.push(t("sites.quotaBalanceUnknown"));
  }

  const showExpiry = shouldShowExpiry(quota.expiresAt);
  const latestAttemptFailed = latestAttempt?.status !== "available";
  const mins = Math.max(0, Math.round((Date.now() - quota.fetchedAt) / 60_000));
  const updated =
    mins < 1
      ? t("sites.quotaUpdatedJustNow")
      : t("sites.quotaUpdated", { time: t("sites.quotaMinutesAgo", { count: mins }) });

  return (
    <div className="flex gap-2" data-testid="site-quota-row">
      <span className="w-28 shrink-0 opacity-50">{t("sites.quota")}</span>
      <div className="min-w-0 flex-1">
        <div className="flex min-w-0 items-center gap-1.5">
          <span className="min-w-0 truncate">{primary}</span>
          {secondary.map((detail) => (
            <span className="contents" key={detail}>
              <span className="opacity-40">·</span>
              <span className="min-w-0 truncate opacity-70">{detail}</span>
            </span>
          ))}
          <Tooltip title={t("sites.quotaRefresh")}>
            <Button
              type="text"
              size="small"
              loading={Boolean(refreshing)}
              icon={<RefreshCw size={14} />}
              onClick={onRefresh}
              aria-label={t("sites.quotaRefresh")}
            />
          </Tooltip>
        </div>
        {percent != null && (
          <Progress
            percent={percent}
            showInfo={false}
            size="small"
            strokeColor={stroke}
            style={{ marginBottom: 0, marginTop: 4 }}
          />
        )}
        {latestAttemptFailed && (
          <div
            className="mt-0.5 text-xs"
            style={{ color: token.colorWarning }}
            role="status"
            aria-live="polite"
          >
            {t("sites.quotaLastSuccessRefreshFailed")}
          </div>
        )}
        <div className="mt-0.5 text-xs opacity-50">
          {showExpiry && quota.expiresAt != null && (
            <span>
              {t("sites.quotaExpires", {
                date: formatExpiryDate(quota.expiresAt, i18n.language),
              })}
            </span>
          )}
          {showExpiry && <span> · </span>}
          <span>{updated}</span>
        </div>
      </div>
    </div>
  );
}
