import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import {
  getCurrentWindow,
  LogicalSize,
  PhysicalPosition,
} from "@tauri-apps/api/window";
import { useTranslation } from "react-i18next";
import { invoke } from "@/lib/invoke";
import type { AppSettings, SiteQuotaSummary } from "@/types/domain";
import {
  ReloadOutlined,
  CloseOutlined,
  DollarOutlined,
  DownOutlined,
  WarningOutlined,
} from "@ant-design/icons";
import { App, Button, Empty, Spin, Tooltip, Typography, theme } from "antd";

const { Text } = Typography;

/**
 * 给颜色套一个透明度上限。
 *
 * 注意是「上限」而不是「覆盖」：antd 在深色主题下很多 token 本身就是 rgba
 * （如 colorFillTertiary 约等于 rgba(255,255,255,0.08)），那一层低透明度正是
 * 「很淡的填充」这个设计意图。直接替换成 0.6 会让本该几乎看不见的填充变成
 * 一块明显的白 —— 卡片就会亮得刺眼。
 */
export function withAlpha(color: string, alpha: number): string {
  const c = color.trim();
  if (c.startsWith("#")) {
    const body = c.slice(1);
    const full =
      body.length === 3
        ? body
            .split("")
            .map((ch) => ch + ch)
            .join("")
        : body;
    const num = Number.parseInt(full.slice(0, 6), 16);
    if (Number.isNaN(num)) return c;
    return `rgba(${(num >> 16) & 255}, ${(num >> 8) & 255}, ${num & 255}, ${alpha})`;
  }
  const match = c.match(/^rgba?\(([^)]+)\)$/);
  if (match) {
    const parts = match[1].split(",").map((s) => s.trim());
    const original = parts.length > 3 ? Number(parts[3]) : 1;
    const final = Number.isNaN(original) ? alpha : Math.min(original, alpha);
    return `rgba(${parts[0]}, ${parts[1]}, ${parts[2]}, ${final})`;
  }
  return c;
}

/** 余额低于这个值（美元）就提示，避免用户用到一半才发现没钱了。 */
const LOW_BALANCE_USD = 5;

/** 收起态是一个小球，展开态是完整列表——对齐主流流量悬浮窗的交互。 */
const EXPANDED_WIDTH = 280;
const EXPANDED_HEIGHT = 400;
const ORB_SIZE = 56;

/** 位移超过这个像素数就算拖动、不算点击。 */
const DRAG_THRESHOLD = 4;

/** 余额低（但不为 0）时用警示色；无限额或未知不提示。 */
function isLowBalance(quota: SiteQuotaSummary["quota"]): boolean {
  return (
    !!quota &&
    !quota.unlimited &&
    quota.remainingUsd !== null &&
    quota.remainingUsd !== undefined &&
    quota.remainingUsd > 0 &&
    quota.remainingUsd < LOW_BALANCE_USD
  );
}

function quotaColor(
  quota: SiteQuotaSummary["quota"],
  token: {
    colorSuccess: string;
    colorWarning: string;
    colorPrimary: string;
    colorTextTertiary: string;
  },
): string {
  if (!quota) return token.colorTextTertiary;
  if (quota.unlimited) return token.colorSuccess;
  if (isLowBalance(quota)) return token.colorWarning;
  if ((quota.remainingUsd ?? 0) > 0) return token.colorPrimary;
  return token.colorTextTertiary;
}

function formatQuota(
  quota: SiteQuotaSummary["quota"],
  t: (key: string) => string,
): string {
  if (!quota) return t("settings.floatingWindowUnavailable");
  if (quota.unlimited) return t("settings.floatingWindowUnlimited");
  if (quota.remainingUsd !== null && quota.remainingUsd !== undefined) {
    return `$${quota.remainingUsd.toFixed(2)}`;
  }
  if (
    quota.totalUsd !== null &&
    quota.totalUsd !== undefined &&
    quota.usedUsd !== null &&
    quota.usedUsd !== undefined
  ) {
    return `$${(quota.totalUsd - quota.usedUsd).toFixed(2)}`;
  }
  return t("settings.floatingWindowUnavailable");
}

/** 从设置里读需要的两个值。 */
interface FloatingPrefs {
  autoRefreshMinutes: number;
  collapsed: boolean;
}

export const FloatingWindow: React.FC = () => {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const appWindow = getCurrentWindow();
  const { message } = App.useApp();
  const [sites, setSites] = useState<SiteQuotaSummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [lastUpdate, setLastUpdate] = useState<Date | null>(null);
  const [collapsed, setCollapsed] = useState(false);
  const [prefs, setPrefs] = useState<FloatingPrefs | null>(null);

  /**
   * 拖动状态。
   *
   * 坐标必须统一：`outerPosition()` 给的是**物理**像素，而鼠标事件是 **CSS** 像素，
   * 两者相差一个 devicePixelRatio。之前把「物理位置 + CSS 位移」当成逻辑坐标交给
   * setPosition，在 175% 缩放下窗口会以 1.75 倍速乱飞 —— 表现就是"拖不动"。
   * 现在统一换算到物理像素再用 PhysicalPosition。
   */
  const drag = useRef<{
    mouseX: number;
    mouseY: number;
    winX: number;
    winY: number;
    moved: boolean;
  } | null>(null);
  const pendingPosition = useRef<{ x: number; y: number } | null>(null);

  /** 拉最新余额：后端并发探测所有站点并更新缓存，返回汇总。 */
  const refreshQuotas = useCallback(
    async (showSpinner = true) => {
      try {
        if (showSpinner) setLoading(true);
        const data = await invoke<SiteQuotaSummary[]>("refresh_sites_quota");
        setSites(data);
        setLastUpdate(new Date());
      } catch (error) {
        console.error("Failed to refresh quotas:", error);
        message.error(t("settings.floatingWindowFetchFailed"));
      } finally {
        setLoading(false);
      }
    },
    [message, t],
  );

  /** 展开/收起时同步窗口尺寸：收起是圆球，展开是面板。 */
  const applyCollapsed = useCallback(
    async (next: boolean) => {
      try {
        await appWindow.setSize(
          next
            ? new LogicalSize(ORB_SIZE, ORB_SIZE)
            : new LogicalSize(EXPANDED_WIDTH, EXPANDED_HEIGHT),
        );
      } catch (error) {
        console.error("Failed to resize floating window:", error);
      }
    },
    [appWindow],
  );

  /** 重读设置里的悬浮窗偏好。设置页改动后由事件触发。 */
  const reloadPrefs = useCallback(async () => {
    try {
      const settings = await invoke<AppSettings>("get_settings");
      const minutes = settings.floatingWindow?.autoRefreshMinutes ?? 5;
      setPrefs({
        autoRefreshMinutes: minutes,
        collapsed: settings.floatingWindow?.collapsed ?? false,
      });
    } catch (error) {
      console.error("Failed to reload floating window settings:", error);
    }
  }, []);

  // 首屏：先读缓存立刻出内容，再后台刷新一次。
  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        const [cached, settings] = await Promise.all([
          invoke<SiteQuotaSummary[]>("get_all_sites_quota"),
          invoke<AppSettings>("get_settings"),
        ]);
        if (cancelled) return;
        setSites(cached);
        if (cached.length > 0) setLastUpdate(new Date());
        const minutes = settings.floatingWindow?.autoRefreshMinutes ?? 5;
        const isCollapsed = settings.floatingWindow?.collapsed ?? false;
        setPrefs({ autoRefreshMinutes: minutes, collapsed: isCollapsed });
        setCollapsed(isCollapsed);
        await applyCollapsed(isCollapsed);
        void refreshQuotas(false);
      } catch (error) {
        console.error("Failed to load floating window state:", error);
        if (!cancelled) void refreshQuotas();
      }
    })();
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // 自动刷新：按设置里的分钟数定时拉取。组件卸载时清掉定时器。
  useEffect(() => {
    if (!prefs) return;
    const intervalMs = Math.max(1, prefs.autoRefreshMinutes) * 60_000;
    const timer = window.setInterval(() => {
      void refreshQuotas(false);
    }, intervalMs);
    return () => window.clearInterval(timer);
  }, [prefs, refreshQuotas]);

  // 设置页改了刷新间隔：立刻跟随，不用重开窗口。
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let disposed = false;
    void listen("floating-settings-changed", () => {
      void reloadPrefs();
    })
      .then((fn) => {
        if (disposed) fn();
        else unlisten = fn;
      })
      .catch((error) => console.error("Failed to listen for settings changes:", error));
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [reloadPrefs]);

  const persistCollapsed = useCallback(
    async (next: boolean) => {
      setCollapsed(next);
      await applyCollapsed(next);
      try {
        await invoke("set_floating_window_collapsed", { collapsed: next });
      } catch (error) {
        console.error("Failed to save collapsed state:", error);
      }
    },
    [applyCollapsed],
  );

  /** 小球与标题栏共用的拖动：按下记录起点，移动超过阈值才算拖动。 */
  const beginDrag = useCallback(
    async (event: React.MouseEvent) => {
      if (event.button !== 0) return;
      if (event.target instanceof HTMLElement && event.target.closest("button")) return;
      try {
        const pos = await appWindow.outerPosition();
        drag.current = {
          mouseX: event.clientX,
          mouseY: event.clientY,
          winX: pos.x,
          winY: pos.y,
          moved: false,
        };
      } catch (error) {
        console.error("Failed to read window position:", error);
      }
    },
    [appWindow],
  );

  useEffect(() => {
    const onMove = (event: MouseEvent) => {
      const origin = drag.current;
      if (!origin) return;
      const scale = window.devicePixelRatio || 1;
      const dx = (event.clientX - origin.mouseX) * scale;
      const dy = (event.clientY - origin.mouseY) * scale;
      if (!origin.moved && Math.hypot(dx, dy) < DRAG_THRESHOLD * scale) return;
      origin.moved = true;
      const x = Math.round(origin.winX + dx);
      const y = Math.round(origin.winY + dy);
      pendingPosition.current = { x, y };
      void appWindow.setPosition(new PhysicalPosition(x, y));
    };

    const onUp = () => {
      const origin = drag.current;
      if (!origin) return;
      drag.current = null;
      const pending = pendingPosition.current;
      pendingPosition.current = null;
      if (!origin.moved || !pending) return;
      // 拖动结束才写库：拖动过程中每一帧都写会打满数据库。
      void invoke("save_floating_window_position", pending).catch((error) => {
        console.error("Failed to save position:", error);
      });
    };

    document.addEventListener("mousemove", onMove);
    document.addEventListener("mouseup", onUp);
    return () => {
      document.removeEventListener("mousemove", onMove);
      document.removeEventListener("mouseup", onUp);
    };
  }, [appWindow]);

  /** 点击（而非拖动）时切换展开/收起。 */
  const handleOrbClick = () => {
    if (drag.current?.moved) return;
    void persistCollapsed(!collapsed);
  };

  const handleClose = () => {
    // 「关闭」= 关掉这个功能；只 hide 的话下次启动它又冒出来，用户会以为关不掉。
    void invoke("set_floating_window_enabled", { enabled: false }).catch((error) => {
      console.error("Failed to close floating window:", error);
    });
  };

  const lowCount = useMemo(
    () => sites.filter((site) => site.enabled && isLowBalance(site.quota)).length,
    [sites],
  );

  /** 收起态的小球：一眼看出是否有站点余额偏低。 */
  const orb = (
    <div
      className="flex h-full w-full items-center justify-center"
      style={{
        background: withAlpha(
          lowCount > 0 ? token.colorWarning : token.colorPrimary,
          0.9,
        ),
        backdropFilter: "blur(20px) saturate(1.6)",
        WebkitBackdropFilter: "blur(20px) saturate(1.6)",
        borderRadius: "50%",
        border: `1px solid ${withAlpha(token.colorBorderSecondary, 0.6)}`,
        cursor: "pointer",
        boxShadow: "0 4px 16px rgba(0, 0, 0, 0.28)",
      }}
      onMouseDown={(event) => void beginDrag(event)}
      onClick={handleOrbClick}
      role="button"
      tabIndex={0}
      aria-label={
        collapsed
          ? t("settings.floatingWindowExpand")
          : t("settings.floatingWindowCollapse")
      }
      onKeyDown={(event) => {
        if (event.key === "Enter" || event.key === " ") void persistCollapsed(!collapsed);
      }}
    >
      <DollarOutlined style={{ color: "#fff", fontSize: 20 }} />
    </div>
  );

  return (
    <div className="h-screen w-full" style={{ padding: collapsed ? 0 : 0 }}>
      {collapsed ? (
        orb
      ) : (
        <div
          data-testid="floating-panel"
          className="flex h-full w-full flex-col"
          style={{
            background: withAlpha(token.colorBgElevated, 0.72),
            backdropFilter: "blur(24px) saturate(1.6)",
            WebkitBackdropFilter: "blur(24px) saturate(1.6)",
            borderRadius: 14,
            overflow: "hidden",
            border: `1px solid ${withAlpha(token.colorBorderSecondary, 0.7)}`,
            color: token.colorText,
          }}
        >
          {/* 标题栏：整条都是拖动区（按下拖动、松手不位移则视为点击收起） */}
          <div
            data-testid="floating-header"
            className="flex items-center justify-between px-3 py-2.5 border-b"
            style={{
              borderColor: withAlpha(token.colorBorderSecondary, 0.6),
              cursor: "move",
            }}
            onMouseDown={(event) => void beginDrag(event)}
            onClick={handleOrbClick}
            title={t("settings.floatingWindowCollapse")}
          >
            <div className="flex items-center gap-2">
              <DollarOutlined style={{ color: token.colorPrimary, fontSize: 16 }} />
              <Text strong style={{ color: token.colorText, fontSize: 13 }}>
                {t("settings.floatingWindowTitle")}
              </Text>
              {lowCount > 0 && (
                <Tooltip title={t("settings.floatingWindowLowBalance")}>
                  <WarningOutlined style={{ color: token.colorWarning, fontSize: 13 }} />
                </Tooltip>
              )}
            </div>
            <div className="flex items-center gap-1">
              <Tooltip title={t("common.refresh")}>
                <Button
                  type="text"
                  size="small"
                  aria-label={t("common.refresh")}
                  icon={<ReloadOutlined />}
                  onClick={(event) => {
                    event.stopPropagation();
                    void refreshQuotas();
                  }}
                  loading={loading}
                  style={{ color: token.colorTextTertiary }}
                />
              </Tooltip>
              <Tooltip title={t("settings.floatingWindowCollapse")}>
                <Button
                  type="text"
                  size="small"
                  aria-label={t("settings.floatingWindowCollapse")}
                  icon={<DownOutlined />}
                  onClick={(event) => {
                    event.stopPropagation();
                    void persistCollapsed(true);
                  }}
                  style={{ color: token.colorTextTertiary }}
                />
              </Tooltip>
              <Tooltip title={t("common.close")}>
                <Button
                  type="text"
                  size="small"
                  aria-label={t("common.close")}
                  icon={<CloseOutlined />}
                  onClick={(event) => {
                    event.stopPropagation();
                    handleClose();
                  }}
                  style={{ color: token.colorTextTertiary }}
                />
              </Tooltip>
            </div>
          </div>

          <div className="flex-1 overflow-y-auto px-4 py-3">
            {loading && sites.length === 0 ? (
              <div className="flex items-center justify-center h-full">
                <Spin />
              </div>
            ) : sites.length === 0 ? (
              <Empty
                description={t("settings.floatingWindowNoSites")}
                image={Empty.PRESENTED_IMAGE_SIMPLE}
                style={{ marginTop: 60 }}
              />
            ) : (
              <div className="space-y-1.5">
                {sites.map((site) => (
                  <div
                    key={site.siteId}
                    className="flex items-center justify-between rounded-lg px-3 py-2"
                    style={{
                      background: withAlpha(token.colorFillTertiary, 0.6),
                      border: `1px solid ${withAlpha(token.colorBorderSecondary, 0.5)}`,
                      opacity: site.enabled ? 1 : 0.45,
                    }}
                  >
                    <Text
                      style={{
                        color: token.colorText,
                        fontSize: 13,
                        maxWidth: 150,
                        overflow: "hidden",
                        textOverflow: "ellipsis",
                        whiteSpace: "nowrap",
                      }}
                      title={site.siteName}
                    >
                      {site.siteName}
                    </Text>
                    <Text strong style={{ color: quotaColor(site.quota, token), fontSize: 13 }}>
                      {formatQuota(site.quota, t)}
                    </Text>
                    {/* 刷新失败时说明原因：否则用户只看到「不可用」，无从判断原因。 */}
                    {site.quota?.error && (
                      <Tooltip title={site.quota.error}>
                        <WarningOutlined
                          style={{ color: token.colorWarning, fontSize: 12, marginLeft: 4 }}
                        />
                      </Tooltip>
                    )}
                  </div>
                ))}
              </div>
            )}
          </div>

          {lastUpdate && (
            <div
              className="px-4 py-2 border-t text-center"
              style={{ borderColor: withAlpha(token.colorBorderSecondary, 0.6) }}
            >
              <Text style={{ color: token.colorTextTertiary, fontSize: 11 }}>
                {t("settings.floatingWindowLastUpdate")} {lastUpdate.toLocaleTimeString()}
              </Text>
            </div>
          )}
        </div>
      )}
    </div>
  );
};
