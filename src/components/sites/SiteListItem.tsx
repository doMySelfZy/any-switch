import { useSortable } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { Button, Dropdown, Tooltip, theme } from "antd";
import type { MenuProps } from "antd";
import { Ellipsis, GripVertical, Pencil, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { ReactNode } from "react";
import type { Site } from "@/types/domain";
import { StatusDot } from "@/components/StatusDot";
import { SiteAvatar } from "@/components/sites/SiteAvatar";
import { useSiteStore } from "@/stores";
import {
  clampQuotaPercent,
  formatQuotaAmountLocalized,
  formatQuotaUpdatedText,
  isBalanceQuotaSummary,
  primaryQuotaWindow,
  quotaUsageTone,
  quotaWindowLabelKey,
} from "@/lib/quotaProbe";

interface Props {
  site: Site;
  active: boolean;
  onSelect: () => void;
  onEdit: () => void;
  onDelete: () => void;
}

export function SiteListItem({ site, active, onSelect, onEdit, onDelete }: Props) {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  // 摘要只基于最近一次成功探测（quotaBySite）；失败/加载态交给右侧详情展示。
  const quota = useSiteStore((s) => s.quotaBySite[site.id]);
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({
    id: site.id,
  });

  const menu: MenuProps = {
    items: [
      {
        key: "edit",
        icon: <Pencil size={14} />,
        label: t("sites.edit"),
      },
      {
        key: "delete",
        icon: <Trash2 size={14} />,
        label: t("sites.delete"),
        danger: true,
      },
    ],
    onClick: ({ key, domEvent }) => {
      domEvent.stopPropagation();
      onSelect();
      if (key === "edit") onEdit();
      else if (key === "delete") onDelete();
    },
  };

  // 列表额度摘要：余额型显示剩余金额（中性灰），窗口型显示 rolling 用量
  // 百分比（≥80% 橙 / ≥90% 红）。其余状态安静不显示，避免一列表灰字。
  let quotaSummary: ReactNode = null;
  if (quota?.status === "available") {
    const windows = quota.windows ?? [];
    if (windows.length > 0) {
      const primary = primaryQuotaWindow(quota);
      if (
        primary &&
        primary.usagePercent != null &&
        Number.isFinite(primary.usagePercent)
      ) {
        const percent = clampQuotaPercent(primary.usagePercent);
        const tone = quotaUsageTone(percent);
        const color =
          tone === "danger"
            ? token.colorError
            : tone === "warn"
              ? token.colorWarning
              : token.colorTextTertiary;
        const updatedText = formatQuotaUpdatedText(quota.fetchedAt, t);
        quotaSummary = (
          <Tooltip
            title={
              <div className="text-xs">
                {windows.map((window) => {
                  const labelKey = quotaWindowLabelKey(window.kind);
                  const label = labelKey ? t(labelKey) : window.kind;
                  const pct =
                    window.usagePercent != null && Number.isFinite(window.usagePercent)
                      ? `${Math.round(clampQuotaPercent(window.usagePercent))}%`
                      : "—";
                  return <div key={window.kind}>{`${label} ${pct}`}</div>;
                })}
                <div>{updatedText}</div>
              </div>
            }
          >
            <span
              className="max-w-[6rem] shrink-0 truncate text-xs tabular-nums"
              style={{ color }}
              data-testid="site-quota-summary"
            >
              {Math.round(percent)}%
            </span>
          </Tooltip>
        );
      }
    } else if (isBalanceQuotaSummary(quota)) {
      const amount = formatQuotaAmountLocalized(quota.remainingUsd, quota.unit, t);
      const updatedText = formatQuotaUpdatedText(quota.fetchedAt, t);
      quotaSummary = (
        <Tooltip
          title={
            <div className="text-xs">
              <div>{t("sites.quotaRemaining", { amount })}</div>
              <div>{updatedText}</div>
            </div>
          }
        >
          <span
            className="max-w-[6rem] shrink-0 truncate text-xs tabular-nums"
            style={{ color: token.colorTextTertiary }}
            data-testid="site-quota-summary"
          >
            {t("sites.quotaRemaining", { amount })}
          </span>
        </Tooltip>
      );
    }
  }

  return (
    <div
      ref={setNodeRef}
      data-testid="site-list-item"
      style={{
        transform: CSS.Translate.toString(transform),
        transition,
        opacity: isDragging ? 0.4 : 1,
        zIndex: isDragging ? 1 : undefined,
      }}
    >
      <Dropdown
        trigger={["contextMenu"]}
        destroyOnHidden
        menu={menu}
        onOpenChange={(open) => {
          if (open) onSelect();
        }}
      >
        <div
          className="site-list-item group flex w-full cursor-pointer items-center rounded-lg pr-0.5 transition-colors"
          data-active={active ? "true" : "false"}
          style={{
            background: active ? token.colorPrimaryBg : undefined,
            color: token.colorText,
            ["--site-item-bg" as string]: token.colorFillQuaternary,
            ["--site-item-hover" as string]: token.colorFillTertiary,
          }}
        >
          <button
            type="button"
            className="site-drag-handle inline-flex h-8 w-6 shrink-0 cursor-grab touch-none items-center justify-center active:cursor-grabbing"
            style={{ color: token.colorTextQuaternary }}
            aria-label={t("sites.dragHandle")}
            title={t("sites.dragHandle")}
            data-testid="site-drag-handle"
            {...attributes}
            {...listeners}
            onClick={(e) => {
              e.stopPropagation();
              onSelect();
            }}
          >
            <GripVertical size={14} className="block" />
          </button>
          <button
            type="button"
            onClick={onSelect}
            className="flex min-w-0 flex-1 cursor-pointer items-center gap-2.5 px-1.5 py-2 text-left"
          >
            <SiteAvatar siteId={site.id} name={site.name} baseUrl={site.baseUrl} size={28} />
            <div className="min-w-0 flex-1">
              <div className="flex min-w-0 items-center gap-1.5">
                <StatusDot
                  className="shrink-0"
                  active={site.enabled}
                  title={site.enabled ? t("sites.enabled") : t("sites.disabled")}
                />
                <div className="truncate text-sm font-medium">{site.name}</div>
              </div>
              <div className="flex min-w-0 items-center gap-2">
                <div className="min-w-0 flex-1 truncate text-xs opacity-50">{site.baseUrl}</div>
                {quotaSummary}
              </div>
            </div>
          </button>
          <Dropdown trigger={["click"]} destroyOnHidden menu={menu} placement="bottomRight">
            <Button
              type="text"
              size="small"
              className="site-more-btn mr-0.5 shrink-0"
              icon={<Ellipsis size={16} />}
              aria-label={t("sites.moreActions")}
              onClick={(e) => {
                e.stopPropagation();
                onSelect();
              }}
            />
          </Dropdown>
        </div>
      </Dropdown>
    </div>
  );
}
