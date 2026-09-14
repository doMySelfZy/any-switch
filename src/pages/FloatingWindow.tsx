import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow, LogicalPosition, LogicalSize } from "@tauri-apps/api/window";
import { useTranslation } from "react-i18next";
import { invoke } from "@/lib/invoke";
import type { AppSettings, SiteQuotaSummary } from "@/types/domain";
import {
  ReloadOutlined,
  CloseOutlined,
  DollarOutlined,
  DownOutlined,
  UpOutlined,
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

/** 展开时的窗口高度；折叠时只留标题栏。 */
const EXPANDED_HEIGHT = 400;
const COLLAPSED_HEIGHT = 48;

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

/** 余额文字颜色：主题色系里挑，避免自己造一套配色与主窗口打架。 */
function quotaColor(
  quota: SiteQuotaSummary["quota"],
  token: { colorSuccess: string; colorWarning: string; colorPrimary: string; colorTextTertiary: string },
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
  const { message } = App.useApp();
  const [sites, setSites] = useState<SiteQuotaSummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [lastUpdate, setLastUpdate] = useState<Date | null>(null);
  const [collapsed, setCollapsed] = useState(false);
  const [prefs, setPrefs] = useState<FloatingPrefs | null>(null);

  const appWindow = getCurrentWindow();
  // 拖动过程中的起点；只在 mouseup 时把最终位置写库，避免每一帧都写数据库。
  const dragOrigin = useRef<{ x: number; y: number; posX: number; posY: number } | null>(null);
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

  /** 折叠时把窗口高度收掉，否则会留一大片空白。 */
  const applyCollapsed = useCallback(
    async (next: boolean) => {
      try {
        const height = next ? COLLAPSED_HEIGHT : EXPANDED_HEIGHT;
        await appWindow.setSize(new LogicalSize(280, height));
      } catch (error) {
        console.error("Failed to resize floating window:", error);
      }
    },
    [appWindow],
  );

  /** 重读设置里的悬浮窗偏好。设置页改动后由事件触发，不必重开窗口。 */
  const reloadPrefs = useCallback(async () => {
    try {
      const settings = await invoke<AppSettings>("get_settings");
      const minutes = settings.floatingWindow?.autoRefreshMinutes ?? 5;
      setPrefs({ autoRefreshMinutes: minutes, collapsed: settings.floatingWindow?.collapsed ?? false });
    } catch (error) {
      console.error("Failed to reload floating window settings:", error);
    }
  }, []);

  // 首屏：先读缓存立刻出内容，再后台刷新一次，避免开窗白屏。
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
    // 只跑一次：后续变更由 floating-settings-changed 事件驱动。
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // 设置页改了刷新间隔：立刻跟随，不用等下一个周期或重开窗口。
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

  // 自动刷新：按设置里的分钟数定时拉取。组件卸载时清掉定时器。
  useEffect(() => {
    if (!prefs) return;
    const intervalMs = Math.max(1, prefs.autoRefreshMinutes) * 60_000;
    const timer = window.setInterval(() => {
      void refreshQuotas(false);
    }, intervalMs);
    return () => window.clearInterval(timer);
  }, [prefs, refreshQuotas]);

  const toggleCollapsed = async () => {
    const next = !collapsed;
    setCollapsed(next);
    await applyCollapsed(next);
    try {
      await invoke("set_floating_window_collapsed", { collapsed: next });
    } catch (error) {
      console.error("Failed to save collapsed state:", error);
    }
  };

  const handleClose = () => {
    // 「关闭」= 关掉这个功能。只 hide 的话下次启动它又冒出来，用户会以为关不掉；
    // 后端在 enabled=false 时会真正 close 掉窗口。
    void invoke("set_floating_window_enabled", { enabled: false }).catch((error) => {
      console.error("Failed to close floating window:", error);
    });
  };

  const handleMouseDown = (event: React.MouseEvent) => {
    if (event.target instanceof HTMLElement && event.target.closest("button")) {
      return;
    }
    // 用窗口当前绝对位置 + 鼠标位移算新位置，避免累积误差。
    void (async () => {
      const pos = await appWindow.outerPosition();
      dragOrigin.current = {
        x: event.clientX,
        y: event.clientY,
        posX: pos.x,
        posY: pos.y,
      };
    })();
  };

  useEffect(() => {
    const onMove = (event: MouseEvent) => {
      const origin = dragOrigin.current;
      if (!origin) return;
      const x = origin.posX + (event.clientX - origin.x);
      const y = origin.posY + (event.clientY - origin.y);
      pendingPosition.current = { x, y };
      void appWindow.setPosition(new LogicalPosition(x, y));
    };

    const onUp = () => {
      if (!dragOrigin.current) return;
      dragOrigin.current = null;
      const pending = pendingPosition.current;
      pendingPosition.current = null;
      // 拖动结束才写库：拖动过程中每一帧都写会打满数据库。
      if (pending) {
        void invoke("save_floating_window_position", pending).catch((error) => {
          console.error("Failed to save position:", error);
        });
      }
    };

    document.addEventListener("mousemove", onMove);
    document.addEventListener("mouseup", onUp);
    return () => {
      document.removeEventListener("mousemove", onMove);
      document.removeEventListener("mouseup", onUp);
    };
  }, [appWindow]);

  // 这三个纯函数定义在组件外（见文件顶部）：`useMemo` 在渲染期求值，
  // 若定义写在它后面会踩 TDZ。
  const lowCount = useMemo(
    () => sites.filter((site) => site.enabled && isLowBalance(site.quota)).length,
    [sites],
  );

  return (
    <div
      className="h-screen w-full flex flex-col"
      style={{
        // 半透明 + 毛玻璃：窗口本身开了 transparent，这里才能透出桌面。
        // 颜色取自主题 token（不再硬编码一套深色），深浅主题都与主窗口协调。
        background: withAlpha(token.colorBgElevated, 0.72),
        backdropFilter: "blur(24px) saturate(1.6)",
        WebkitBackdropFilter: "blur(24px) saturate(1.6)",
        borderRadius: 14,
        overflow: "hidden",
        border: `1px solid ${withAlpha(token.colorBorderSecondary, 0.7)}`,
        color: token.colorText,
      }}
    >
      {/* 标题栏：拖动区 + 刷新/折叠/关闭 */}
      <div
        className="flex items-center justify-between px-3 py-2.5 border-b"
        style={{
          borderColor: withAlpha(token.colorBorderSecondary, 0.6),
          cursor: "move",
        }}
        onMouseDown={handleMouseDown}
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
              onClick={() => void refreshQuotas()}
              loading={loading}
              style={{ color: token.colorTextTertiary }}
            />
          </Tooltip>
          <Tooltip
            title={
              collapsed
                ? t("settings.floatingWindowExpand")
                : t("settings.floatingWindowCollapse")
            }
          >
            <Button
              type="text"
              size="small"
              aria-label={
                collapsed
                  ? t("settings.floatingWindowExpand")
                  : t("settings.floatingWindowCollapse")
              }
              icon={collapsed ? <DownOutlined /> : <UpOutlined />}
              onClick={() => void toggleCollapsed()}
              style={{ color: token.colorTextTertiary }}
            />
          </Tooltip>
          <Tooltip title={t("common.close")}>
            <Button
              type="text"
              size="small"
              aria-label={t("common.close")}
              icon={<CloseOutlined />}
              onClick={handleClose}
              style={{ color: token.colorTextTertiary }}
            />
          </Tooltip>
        </div>
      </div>

      {/* 内容区：折叠时淡出并收掉，配合窗口高度变化做过渡。
          折叠后仍留在 DOM 里（过渡需要），用 aria-hidden 让辅助技术忽略它。 */}
      <div
        data-testid="floating-content"
        aria-hidden={collapsed}
        className="flex min-h-0 flex-col"
        style={{
          flex: collapsed ? "0 1 auto" : "1 1 auto",
          maxHeight: collapsed ? 0 : "100%",
          opacity: collapsed ? 0 : 1,
          overflow: "hidden",
          transition: "max-height 200ms ease, opacity 150ms ease",
          pointerEvents: collapsed ? "none" : "auto",
        }}
      >
        <>
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
                    {/* 刷新失败时说明原因：否则用户只看到「不可用」，
                        无从判断是网络、密钥还是站点不提供余额接口。 */}
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
              className="px-3 py-2 border-t text-center"
              style={{ borderColor: withAlpha(token.colorBorderSecondary, 0.6) }}
            >
              <Text style={{ color: token.colorTextTertiary, fontSize: 11 }}>
                {t("settings.floatingWindowLastUpdate")} {lastUpdate.toLocaleTimeString()}
              </Text>
            </div>
          )}
        </>
      </div>
    </div>
  );
};
