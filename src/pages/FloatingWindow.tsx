import React, { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { LogicalPosition } from "@tauri-apps/api/window";
import type { SiteQuotaSummary } from "@/types/domain";
import {
  ReloadOutlined,
  CloseOutlined,
  DollarOutlined,
} from "@ant-design/icons";
import { Button, Spin, Typography, Empty, message } from "antd";

const { Text } = Typography;

export const FloatingWindow: React.FC = () => {
  const [sites, setSites] = useState<SiteQuotaSummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [lastUpdate, setLastUpdate] = useState<Date | null>(null);
  const [isDragging, setIsDragging] = useState(false);
  const [dragStart, setDragStart] = useState({ x: 0, y: 0 });

  const appWindow = getCurrentWindow();

  const fetchQuotas = async () => {
    try {
      setLoading(true);
      const data = await invoke<SiteQuotaSummary[]>("get_all_sites_quota");
      setSites(data);
      setLastUpdate(new Date());
    } catch (error) {
      console.error("Failed to fetch quotas:", error);
      message.error("获取余额失败");
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchQuotas();
  }, []);

  const handleClose = () => {
    appWindow.hide();
  };

  const handleMouseDown = (e: React.MouseEvent) => {
    if (e.target instanceof HTMLElement && e.target.closest("button")) {
      return;
    }
    setIsDragging(true);
    setDragStart({ x: e.clientX, y: e.clientY });
  };

  useEffect(() => {
    if (!isDragging) return;

    const handleMouseMove = async (e: MouseEvent) => {
      const deltaX = e.clientX - dragStart.x;
      const deltaY = e.clientY - dragStart.y;

      const position = await appWindow.outerPosition();
      const newX = position.x + deltaX;
      const newY = position.y + deltaY;

      await appWindow.setPosition(new LogicalPosition(newX, newY));

      try {
        await invoke("save_floating_window_position", { x: newX, y: newY });
      } catch (error) {
        console.error("Failed to save position:", error);
      }
    };

    const handleMouseUp = () => {
      setIsDragging(false);
    };

    document.addEventListener("mousemove", handleMouseMove);
    document.addEventListener("mouseup", handleMouseUp);

    return () => {
      document.removeEventListener("mousemove", handleMouseMove);
      document.removeEventListener("mouseup", handleMouseUp);
    };
  }, [isDragging, dragStart, appWindow]);

  const formatQuota = (quota: SiteQuotaSummary["quota"]) => {
    if (!quota) return "未知";
    if (quota.unlimited) return "无限制";
    if (quota.remainingUsd !== null) {
      return `$${quota.remainingUsd.toFixed(2)}`;
    }
    if (quota.totalUsd !== null && quota.usedUsd !== null) {
      const remaining = quota.totalUsd - quota.usedUsd;
      return `$${remaining.toFixed(2)}`;
    }
    return "未知";
  };

  return (
    <div
      className="h-screen w-full flex flex-col"
      style={{
        background: "rgba(15, 19, 28, 0.95)",
        backdropFilter: "blur(12px)",
      }}
    >
      {/* Header */}
      <div
        className="flex items-center justify-between px-4 py-3 border-b cursor-move"
        style={{ borderColor: "rgba(56, 189, 248, 0.2)" }}
        onMouseDown={handleMouseDown}
      >
        <div className="flex items-center gap-2">
          <DollarOutlined style={{ color: "#38bdf8", fontSize: 18 }} />
          <Text strong style={{ color: "#38bdf8", fontSize: 14 }}>
            站点余额
          </Text>
        </div>
        <div className="flex items-center gap-2">
          <Button
            type="text"
            size="small"
            icon={<ReloadOutlined />}
            onClick={fetchQuotas}
            loading={loading}
            style={{ color: "#94a3b8" }}
          />
          <Button
            type="text"
            size="small"
            icon={<CloseOutlined />}
            onClick={handleClose}
            style={{ color: "#94a3b8" }}
          />
        </div>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto px-4 py-3">
        {loading && sites.length === 0 ? (
          <div className="flex items-center justify-center h-full">
            <Spin />
          </div>
        ) : sites.length === 0 ? (
          <Empty
            description="暂无站点"
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
                <Text
                  strong
                  style={{
                    color: site.quota?.unlimited
                      ? "#6ee7b7"
                      : site.quota?.remainingUsd && site.quota.remainingUsd > 0
                        ? "#38bdf8"
                        : "#94a3b8",
                    fontSize: 13,
                  }}
                >
                  {formatQuota(site.quota)}
                </Text>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Footer */}
      {lastUpdate && (
        <div
          className="px-4 py-2 border-t text-center"
          style={{ borderColor: "rgba(56, 189, 248, 0.2)" }}
        >
          <Text style={{ color: "#64748b", fontSize: 11 }}>
            更新于 {lastUpdate.toLocaleTimeString()}
          </Text>
        </div>
      )}
    </div>
  );
};

