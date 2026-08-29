import { useSortable } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { Button, Dropdown, theme } from "antd";
import type { MenuProps } from "antd";
import { Ellipsis, GripVertical, Pencil, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { Site } from "@/types/domain";
import { StatusDot } from "@/components/StatusDot";
import { SiteAvatar } from "@/components/sites/SiteAvatar";

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
              <div className="truncate text-xs opacity-50">{site.baseUrl}</div>
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
