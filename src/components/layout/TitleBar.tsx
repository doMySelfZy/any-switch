import { useCallback, useEffect, useRef, useState } from "react";
import { Dropdown, theme, Tooltip } from "antd";
import type { MenuProps } from "antd";
import { ArrowDownCircle, Github, Globe, Loader2, Minus, Moon, Monitor, Pin, PinOff, Settings, Square, Sun, X, XCircle } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useSettingsStore, useUIStore } from "@/stores";
import { invoke, isTauri } from "@/lib/invoke";
import { LANG_OPTIONS, APP_NAME, GITHUB_REPO_URL } from "@/lib/constants";
import { openExternalUrl } from "@/lib/openUrl";
import { useUpdateCheckBusy, useUpdateChecker } from "@/hooks/useUpdateChecker";
import { BackupQuickPopover } from "./BackupQuickPopover";

const IS_WINDOWS = navigator.userAgent.includes("Windows");

const THEME_OPTIONS = [
  { key: "system", icon: <Monitor size={14} />, labelKey: "settings.themeSystem" },
  { key: "light", icon: <Sun size={14} />, labelKey: "settings.themeLight" },
  { key: "dark", icon: <Moon size={14} />, labelKey: "settings.themeDark" },
] as const;

const THEME_ICONS: Record<string, React.ReactNode> = {
  system: <Monitor size={14} />,
  light: <Sun size={14} />,
  dark: <Moon size={14} />,
};

type AppSettingsTheme = "system" | "light" | "dark";

/** Windows "restore down" icon: two overlapping rectangles */
function RestoreIcon() {
  return (
    <svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" strokeWidth="1.2">
      <rect x="3" y="5" width="8" height="7" rx="0.5" />
      <path d="M5 5V3.5a.5.5 0 0 1 .5-.5H12a.5.5 0 0 1 .5.5V10a.5.5 0 0 1-.5.5h-1.5" />
    </svg>
  );
}

export function TitleBar() {
  const { t, i18n } = useTranslation();
  const { token } = theme.useToken();
  const activePage = useUIStore((s) => s.activePage);
  const setPage = useUIStore((s) => s.setPage);
  const themeMode = useSettingsStore((s) => s.settings.themeMode);
  const alwaysOnTop = useSettingsStore((s) => s.settings.alwaysOnTop);
  const saveSettings = useSettingsStore((s) => s.saveSettings);
  const [pinned, setPinned] = useState(alwaysOnTop);
  const [isMaximized, setIsMaximized] = useState(false);
  const dragTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const isInSettings = activePage === "settings";
  const { checkForUpdate } = useUpdateChecker();
  const checkingUpdate = useUpdateCheckBusy();

  useEffect(() => {
    setPinned(alwaysOnTop);
  }, [alwaysOnTop]);

  useEffect(() => {
    if (!IS_WINDOWS || !isTauri()) return;
    let unlisten: (() => void) | undefined;
    void (async () => {
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      const win = getCurrentWindow();
      setIsMaximized(await win.isMaximized());
      unlisten = await win.onResized(async () => {
        setIsMaximized(await win.isMaximized());
      });
    })();
    return () => {
      unlisten?.();
    };
  }, []);

  const handleCheckUpdate = useCallback(() => {
    if (checkingUpdate) return;
    void checkForUpdate();
  }, [checkForUpdate, checkingUpdate]);

  const handlePin = useCallback(async () => {
    const next = !pinned;
    setPinned(next);
    try {
      await invoke("set_always_on_top", { enabled: next });
      await saveSettings({ alwaysOnTop: next });
    } catch {
      setPinned(!next);
    }
  }, [pinned, saveSettings]);

  const handleDragMouseDown = useCallback((e: React.MouseEvent<HTMLDivElement>) => {
    const target = e.target as HTMLElement;
    if (target.closest("button") || target.closest(".title-bar-nodrag") || target.closest("a")) {
      return;
    }
    if (!isTauri()) return;
    e.preventDefault();

    void (async () => {
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      if (IS_WINDOWS) {
        // Delay so double-click maximize can cancel the pending drag.
        if (dragTimerRef.current) clearTimeout(dragTimerRef.current);
        dragTimerRef.current = setTimeout(() => {
          void getCurrentWindow().startDragging();
        }, 200);
      } else {
        await getCurrentWindow().startDragging();
      }
    })();
  }, []);

  const handleTitleBarDoubleClick = useCallback(() => {
    if (!IS_WINDOWS) return;
    if (dragTimerRef.current) {
      clearTimeout(dragTimerRef.current);
      dragTimerRef.current = null;
    }
    void invoke("toggle_maximize_window");
  }, []);

  const themeMenu: MenuProps["items"] = THEME_OPTIONS.map((o) => ({
    key: o.key,
    icon: o.icon,
    label: t(o.labelKey),
  }));

  const langMenu: MenuProps["items"] = LANG_OPTIONS.map((o) => ({
    key: o.key,
    label: `${o.icon} ${o.label}`,
  }));

  const buttonBase: React.CSSProperties = {
    width: 28,
    height: 28,
    display: "flex",
    alignItems: "center",
    justifyContent: "center",
    borderRadius: token.borderRadius,
    fontSize: 14,
    cursor: "pointer",
    border: "none",
    backgroundColor: "transparent",
    color: token.colorTextSecondary,
  };

  const hoverHandlers = (baseColor: string) => ({
    onMouseEnter: (e: React.MouseEvent<HTMLButtonElement>) => {
      e.currentTarget.style.backgroundColor = token.colorFillSecondary;
      e.currentTarget.style.color = token.colorTextBase;
    },
    onMouseLeave: (e: React.MouseEvent<HTMLButtonElement>) => {
      e.currentTarget.style.backgroundColor = "transparent";
      e.currentTarget.style.color = baseColor;
    },
  });

  return (
    <div
      className="title-bar-drag"
      {...(!IS_WINDOWS ? { "data-tauri-drag-region": true } : {})}
      onMouseDown={handleDragMouseDown}
      onDoubleClick={IS_WINDOWS ? handleTitleBarDoubleClick : undefined}
      style={{
        height: 36,
        display: "flex",
        alignItems: "center",
        justifyContent: "space-between",
        paddingLeft: IS_WINDOWS ? 12 : 72,
        paddingRight: IS_WINDOWS ? 0 : 12,
        backgroundColor: "transparent",
        flexShrink: 0,
        borderBottom: `1px solid ${token.colorBorderSecondary}`,
      }}
    >
      {IS_WINDOWS ? (
        <div className="title-bar-nodrag" style={{ display: "flex", alignItems: "center", gap: 6, marginRight: 8 }}>
          <span style={{ fontSize: 13, fontWeight: 600, color: token.colorTextBase, userSelect: "none" }}>
            {APP_NAME}
          </span>
        </div>
      ) : (
        <div />
      )}

      <div style={{ display: "flex", alignItems: "center", gap: 0 }}>
        <div className="title-bar-nodrag" style={{ display: "flex", alignItems: "center", gap: 4 }}>
          <Tooltip title={pinned ? t("desktop.unpin") : t("desktop.pin")}>
            <button
              type="button"
              onClick={() => void handlePin()}
              style={{
                ...buttonBase,
                color: pinned ? token.colorPrimary : token.colorTextSecondary,
              }}
              onMouseEnter={(e) => {
                e.currentTarget.style.backgroundColor = pinned ? token.colorPrimaryBg : token.colorFillSecondary;
                e.currentTarget.style.color = pinned ? token.colorPrimary : token.colorTextBase;
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.backgroundColor = "transparent";
                e.currentTarget.style.color = pinned ? token.colorPrimary : token.colorTextSecondary;
              }}
            >
              {pinned ? <Pin size={14} /> : <PinOff size={14} />}
            </button>
          </Tooltip>

          <BackupQuickPopover />

          <Dropdown
            menu={{
              items: themeMenu,
              onClick: ({ key }) => {
                void saveSettings({ themeMode: key as AppSettingsTheme });
              },
              selectedKeys: [themeMode],
            }}
            trigger={["click"]}
            placement="bottomRight"
            destroyOnHidden
          >
            <button type="button" style={buttonBase} {...hoverHandlers(token.colorTextSecondary)}>
              {THEME_ICONS[themeMode] ?? <Monitor size={14} />}
            </button>
          </Dropdown>

          <Dropdown
            menu={{
              items: langMenu,
              onClick: ({ key }) => {
                void i18n.changeLanguage(key);
                void saveSettings({ language: key as "zh-CN" | "en-US" });
              },
              selectedKeys: [i18n.language],
            }}
            trigger={["click"]}
            placement="bottomRight"
            destroyOnHidden
          >
            <button type="button" style={buttonBase} {...hoverHandlers(token.colorTextSecondary)}>
              <Globe size={14} />
            </button>
          </Dropdown>

          {isTauri() && (
            <Tooltip title={checkingUpdate ? t("settings.checkingUpdate") : t("settings.checkUpdate")}>
              <button
                type="button"
                aria-label={checkingUpdate ? t("settings.checkingUpdate") : t("settings.checkUpdate")}
                disabled={checkingUpdate}
                onClick={handleCheckUpdate}
                style={{
                  ...buttonBase,
                  opacity: checkingUpdate ? 0.5 : 1,
                }}
                {...hoverHandlers(token.colorTextSecondary)}
              >
                {checkingUpdate ? (
                  <Loader2 size={14} className="animate-spin" />
                ) : (
                  <ArrowDownCircle size={14} />
                )}
              </button>
            </Tooltip>
          )}

          <Tooltip title={t("desktop.github")}>
            <button
              type="button"
              aria-label={t("desktop.github")}
              onClick={() => {
                void openExternalUrl(GITHUB_REPO_URL);
              }}
              style={buttonBase}
              {...hoverHandlers(token.colorTextSecondary)}
            >
              <Github size={14} />
            </button>
          </Tooltip>

          <Tooltip title={isInSettings ? t("settings.closeSettings") : t("settings.openSettings")}>
            <button
              type="button"
              onClick={(e) => {
                setPage(isInSettings ? "sites" : "settings");
                e.currentTarget.style.backgroundColor = "transparent";
                e.currentTarget.blur();
              }}
              style={{
                ...buttonBase,
                color: isInSettings ? token.colorError : token.colorTextSecondary,
              }}
              onMouseEnter={(e) => {
                e.currentTarget.style.backgroundColor = isInSettings
                  ? token.colorErrorBg
                  : token.colorFillSecondary;
                e.currentTarget.style.color = isInSettings ? token.colorError : token.colorTextBase;
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.backgroundColor = "transparent";
                e.currentTarget.style.color = isInSettings ? token.colorError : token.colorTextSecondary;
              }}
            >
              {isInSettings ? <XCircle size={14} /> : <Settings size={14} />}
            </button>
          </Tooltip>
        </div>

        {IS_WINDOWS && isTauri() && (
          <div className="title-bar-nodrag" style={{ display: "flex", alignItems: "center", marginLeft: 4 }}>
            <button
              type="button"
              onClick={() => void invoke("minimize_window")}
              style={{
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                width: 46,
                height: 36,
                border: "none",
                background: "transparent",
                color: token.colorTextSecondary,
                cursor: "pointer",
              }}
              onMouseEnter={(e) => {
                e.currentTarget.style.backgroundColor = token.colorFillSecondary;
                e.currentTarget.style.color = token.colorTextBase;
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.backgroundColor = "transparent";
                e.currentTarget.style.color = token.colorTextSecondary;
              }}
            >
              <Minus size={16} />
            </button>
            <button
              type="button"
              onClick={() => void invoke("toggle_maximize_window")}
              style={{
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                width: 46,
                height: 36,
                border: "none",
                background: "transparent",
                color: token.colorTextSecondary,
                cursor: "pointer",
              }}
              onMouseEnter={(e) => {
                e.currentTarget.style.backgroundColor = token.colorFillSecondary;
                e.currentTarget.style.color = token.colorTextBase;
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.backgroundColor = "transparent";
                e.currentTarget.style.color = token.colorTextSecondary;
              }}
            >
              {isMaximized ? <RestoreIcon /> : <Square size={14} />}
            </button>
            <button
              type="button"
              onClick={() => {
                void import("@tauri-apps/api/window").then(({ getCurrentWindow }) => getCurrentWindow().close());
              }}
              style={{
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                width: 46,
                height: 36,
                border: "none",
                background: "transparent",
                color: token.colorTextSecondary,
                cursor: "pointer",
              }}
              onMouseEnter={(e) => {
                e.currentTarget.style.backgroundColor = "#e81123";
                e.currentTarget.style.color = "#ffffff";
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.backgroundColor = "transparent";
                e.currentTarget.style.color = token.colorTextSecondary;
              }}
            >
              <X size={16} />
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
