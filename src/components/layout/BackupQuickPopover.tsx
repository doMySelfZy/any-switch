import { useCallback, useEffect, useState } from "react";
import { App, Button, Popover, Space, Spin, Tag, theme, Tooltip, Typography } from "antd";
import { CloudUpload, DatabaseBackup, HardDriveDownload, Settings } from "lucide-react";
import { useTranslation } from "react-i18next";
import { invoke, isAppError } from "@/lib/invoke";
import { useUIStore } from "@/stores";
import type { BackupOperationResult, BackupOverview } from "@/types/domain";

function formatTimestamp(value: number | null): string {
  if (value == null) return "-";
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? "-" : date.toLocaleString();
}

export function BackupQuickPopover() {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const { message } = App.useApp();
  const [open, setOpen] = useState(false);
  const [loading, setLoading] = useState(false);
  const [destination, setDestination] = useState<"local" | "webdav" | null>(null);
  const [overview, setOverview] = useState<BackupOverview | null>(null);

  const loadOverview = useCallback(async () => {
    setLoading(true);
    try {
      setOverview(await invoke<BackupOverview>("get_backup_overview"));
    } catch (error) {
      message.error(isAppError(error) ? error.message : t("settings.webdav.loadFailed"));
    } finally {
      setLoading(false);
    }
  }, [message, t]);

  useEffect(() => {
    if (open) void loadOverview();
  }, [loadOverview, open]);

  const createBackup = async (nextDestination: "local" | "webdav") => {
    setDestination(nextDestination);
    try {
      const result = await invoke<BackupOperationResult>("create_app_backup", {
        destination: nextDestination,
      });
      message[result.warning ? "warning" : "success"](
        result.warning
          ? t("settings.webdav.cleanupWarning")
          : t(
              nextDestination === "local"
                ? "settings.webdav.localBackupSuccess"
                : "settings.webdav.backupSuccess",
            ),
      );
      await loadOverview();
    } catch (error) {
      message.error(isAppError(error) ? error.message : t("settings.webdav.backupFailed"));
    } finally {
      setDestination(null);
    }
  };

  const openBackupSettings = () => {
    useUIStore.getState().setSettingsTab("backup");
    useUIStore.getState().setPage("settings");
    setOpen(false);
  };

  const status = overview?.webdavSync.status ?? "never";
  const content = (
    <div style={{ width: 320 }}>
      <div
        style={{
          fontSize: 13,
          fontWeight: 600,
          color: token.colorTextSecondary,
          marginBottom: 6,
        }}
      >
        {t("settings.webdav.quickTitle")}
      </div>
      <Typography.Text type="secondary" style={{ fontSize: 12, display: "block", marginBottom: 12 }}>
        {t("settings.webdav.quickSecurityWarning")}
      </Typography.Text>
      {loading && !overview ? (
        <div className="flex justify-center py-4"><Spin size="small" /></div>
      ) : (
        <Space orientation="vertical" size={6} className="w-full">
          <div className="flex justify-between gap-4 text-xs">
            <span style={{ color: token.colorTextSecondary }}>{t("settings.webdav.lastLocal")}</span>
            <span>{formatTimestamp(overview?.latestLocalBackupAt ?? null)}</span>
          </div>
          <div className="flex justify-between gap-4 text-xs">
            <span style={{ color: token.colorTextSecondary }}>{t("settings.webdav.webdavStatus")}</span>
            <Tag className="m-0" color={status === "success" ? "success" : status === "failed" ? "error" : status === "warning" ? "warning" : "default"}>
              {t(`settings.webdav.status.${status}`)}
            </Tag>
          </div>
          <div className="flex justify-between gap-4 text-xs">
            <span style={{ color: token.colorTextSecondary }}>{t("settings.webdav.lastSuccess")}</span>
            <span>{formatTimestamp(overview?.webdavSync.lastSuccessAt ?? null)}</span>
          </div>
          <div className="flex justify-between gap-4 text-xs">
            <span style={{ color: token.colorTextSecondary }}>{t("settings.webdav.nextRun")}</span>
            <span>{formatTimestamp(overview?.nextScheduledAt ?? null)}</span>
          </div>
        </Space>
      )}
      <Space className="mt-3 w-full" orientation="vertical">
        <Button
          block
          icon={<HardDriveDownload size={14} />}
          loading={destination === "local"}
          disabled={destination !== null}
          onClick={() => void createBackup("local")}
        >
          {t("settings.webdav.localBackupNow")}
        </Button>
        <Button
          block
          type="primary"
          icon={<CloudUpload size={14} />}
          loading={destination === "webdav"}
          disabled={destination !== null || !overview?.webdavConfigured}
          onClick={() => void createBackup("webdav")}
        >
          {t("settings.webdav.backupNow")}
        </Button>
        {!overview?.webdavConfigured && (
          <Button block type="link" icon={<Settings size={14} />} onClick={openBackupSettings}>
            {t("settings.webdav.configureFirst")}
          </Button>
        )}
      </Space>
    </div>
  );

  const buttonStyle: React.CSSProperties = {
    width: 28,
    height: 28,
    display: "flex",
    alignItems: "center",
    justifyContent: "center",
    borderRadius: token.borderRadius,
    cursor: "pointer",
    border: "none",
    backgroundColor: "transparent",
    color: token.colorTextSecondary,
  };

  return (
    <Popover
      open={open}
      onOpenChange={setOpen}
      placement="bottomRight"
      trigger="click"
      content={content}
      destroyOnHidden
    >
      <Tooltip title={t("settings.webdav.quickTitle")}>
        <button
          type="button"
          aria-label={t("settings.webdav.quickTitle")}
          style={buttonStyle}
          onMouseEnter={(event) => {
            event.currentTarget.style.backgroundColor = token.colorFillSecondary;
            event.currentTarget.style.color = token.colorTextBase;
          }}
          onMouseLeave={(event) => {
            event.currentTarget.style.backgroundColor = "transparent";
            event.currentTarget.style.color = token.colorTextSecondary;
          }}
        >
          <DatabaseBackup size={14} />
        </button>
      </Tooltip>
    </Popover>
  );
}
