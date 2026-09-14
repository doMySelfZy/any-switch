import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
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
import { App, Button, Empty, Spin, Tooltip, Typography } from "antd";

const { Text } = Typography;

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

function quotaColor(quota: SiteQuotaSummary["quota"]): string {
  if (!quota) return "#94a3b8";
  if (quota.unlimited) return "#6ee7b7";
  if (isLowBalance(quota)) return "#fbbf24";
  if ((quota.remainingUsd ?? 0) > 0) return "#38bdf8";
  return "#94a3b8";
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
    // 只跑一次：设置变更由设置页负责，这里不需要跟随。
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
    void appWindow.hide();
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
        background: "rgba(15, 19, 28, 0.95)",
        backdropFilter: "blur(12px)",
      }}
    >
      {/* 标题栏：拖动区 + 刷新/折叠/关闭 */}
      <div
        className="flex items-center justify-between px-4 py-3 border-b"
        style={{ borderColor: "rgba(56, 189, 248, 0.2)", cursor: "move" }}
        onMouseDown={handleMouseDown}
      >
        <div className="flex items-center gap-2">
          <DollarOutlined style={{ color: "#38bdf8", fontSize: 18 }} />
          <Text strong style={{ color: "#38bdf8", fontSize: 14 }}>
            {t("settings.floatingWindowTitle")}
          </Text>
          {lowCount > 0 && (
            <Tooltip title={t("settings.floatingWindowLowBalance")}>
              <WarningOutlined style={{ color: "#fbbf24", fontSize: 13 }} />
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
              style={{ color: "#94a3b8" }}
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
              style={{ color: "#94a3b8" }}
            />
          </Tooltip>
          <Tooltip title={t("common.close")}>
            <Button
              type="text"
              size="small"
              aria-label={t("common.close")}
              icon={<CloseOutlined />}
              onClick={handleClose}
              style={{ color: "#94a3b8" }}
            />
          </Tooltip>
        </div>
      </div>

      {/* 内容区：折叠时整块隐藏 */}
      {!collapsed && (
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
              <div className="space-y-2">
                {sites.map((site) => (
                  <div
                    key={site.siteId}
                    className="flex items-center justify-between px-3 py-2 rounded-lg transition-colors"
                    style={{
                      background: "rgba(30, 38, 54, 0.6)",
                      border: "1px solid rgba(56, 189, 248, 0.1)",
                      opacity: site.enabled ? 1 : 0.5,
                    }}
                  >
                    <Text
                      style={{
                        color: "#e2e8f0",
                        fontSize: 13,
                        maxWidth: 140,
                        overflow: "hidden",
                        textOverflow: "ellipsis",
                        whiteSpace: "nowrap",
                      }}
                      title={site.siteName}
                    >
                      {site.siteName}
                    </Text>
                    <Text strong style={{ color: quotaColor(site.quota), fontSize: 13 }}>
                      {formatQuota(site.quota, t)}
                    </Text>
                  </div>
                ))}
              </div>
            )}
          </div>

          {lastUpdate && (
            <div
              className="px-4 py-2 border-t text-center"
              style={{ borderColor: "rgba(56, 189, 248, 0.2)" }}
            >
              <Text style={{ color: "#64748b", fontSize: 11 }}>
                {t("settings.floatingWindowLastUpdate")} {lastUpdate.toLocaleTimeString()}
              </Text>
            </div>
          )}
        </>
      )}
    </div>
  );
};
