import { Tooltip, theme } from "antd";
import { Layers, Rocket, Sparkles, Plug, ScrollText } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useUIStore, type AppPage } from "@/stores/uiStore";

const ITEMS: { key: AppPage; icon: React.ReactNode; labelKey: string }[] = [
  { key: "sites", icon: <Layers size={18} />, labelKey: "nav.sites" },
  { key: "apply", icon: <Rocket size={18} />, labelKey: "nav.apply" },
  { key: "skills", icon: <Sparkles size={18} />, labelKey: "nav.skills" },
  { key: "mcp", icon: <Plug size={18} />, labelKey: "nav.mcp" },
  { key: "rules", icon: <ScrollText size={18} />, labelKey: "nav.rules" },
];

export function SideNav() {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const activePage = useUIStore((s) => s.activePage);
  const setPage = useUIStore((s) => s.setPage);

  if (activePage === "settings") return null;

  return (
    <nav
      className="flex h-full shrink-0 flex-col items-center"
      style={{
        width: 48,
        paddingTop: 8,
        paddingBottom: 12,
        borderRight: `1px solid ${token.colorBorderSecondary}`,
        background: "transparent",
      }}
    >
      <div className="flex flex-col gap-2">
        {ITEMS.map((item) => {
          const active = activePage === item.key;
          const label = t(item.labelKey);
          return (
            <Tooltip key={item.key} title={label} placement="right">
              <button
                type="button"
                onClick={() => setPage(item.key)}
                className="flex items-center justify-center transition-colors"
                style={{
                  width: 36,
                  height: 36,
                  borderRadius: "50%",
                  border: "none",
                  cursor: "pointer",
                  backgroundColor: active ? token.colorPrimaryBg : "transparent",
                  color: active ? token.colorPrimary : token.colorTextSecondary,
                }}
                onMouseEnter={(e) => {
                  if (!active) {
                    e.currentTarget.style.backgroundColor = token.colorFillSecondary;
                    e.currentTarget.style.color = token.colorTextBase;
                  }
                }}
                onMouseLeave={(e) => {
                  if (!active) {
                    e.currentTarget.style.backgroundColor = "transparent";
                    e.currentTarget.style.color = token.colorTextSecondary;
                  }
                }}
              >
                {item.icon}
              </button>
            </Tooltip>
          );
        })}
      </div>
    </nav>
  );
}
