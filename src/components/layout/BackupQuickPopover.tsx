import { useCallback, useEffect, useState } from "react";
import { App, Button, Popover, Space, Spin, Tag, theme, Tooltip, Typography } from "antd";
import { CloudDownload, CloudUpload, DatabaseBackup, HardDriveDownload, Settings } from "lucide-react";
import { useTranslation } from "react-i18next";
import { invoke, isAppError } from "@/lib/invoke";
import { webdavErrorMessage } from "@/lib/webdavErrors";
import { useUIStore } from "@/stores";
import type { BackupOperationResult, BackupOverview, RemoteBackupInfo, SyncOutcome } from "@/types/domain";

function formatTimestamp(value: number | null): string {
  if (value == null) return "-";
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? "-" : date.toLocaleString();
}

/** Prefer first item when the list is already newest→oldest; otherwise the latest lastModified. */
function pickLatestRemoteBackup(items: RemoteBackupInfo[]): RemoteBackupInfo | null {
  if (items.length === 0) return null;
  let latest = items[0];
  let latestTime = Date.parse(latest.lastModified);
  for (const item of items.slice(1)) {
    const time = Date.parse(item.lastModified);
    if (Number.isNaN(time)) continue;
    if (Number.isNaN(latestTime) || time > latestTime) {
      latest = item;
      latestTime = time;
    }
  }
  return latest;
}

export function BackupQuickPopover() {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const { message, modal } = App.useApp();
  const [open, setOpen] = useState(false);
  const [loading, setLoading] = useState(false);
  const [destination, setDestination] = useState<"local" | null>(null);
  const [syncing, setSyncing] = useState(false);
  const [overview, setOverview] = useState<BackupOverview | null>(null);
  const [remoteBackups, setRemoteBackups] = useState<RemoteBackupInfo[]>([]);

  const loadOverview = useCallback(async () => {
    setLoading(true);
    try {
      const next = await invoke<BackupOverview>("get_backup_overview");
      setOverview(next);
      if (!next.webdavConfigured) {
        setRemoteBackups([]);
        return;
      }
      try {
        setRemoteBackups(await invoke<RemoteBackupInfo[]>("list_webdav_backups"));
      } catch (error) {
        setRemoteBackups([]);
        message.error(webdavErrorMessage(error, t, "settings.webdav.loadFailed"));
      }
    } catch (error) {
      message.error(webdavErrorMessage(error, t, "settings.webdav.loadFailed"));
    } finally {
      setLoading(false);
    }
  }, [message, t]);

  useEffect(() => {
    if (open) void loadOverview();
  }, [loadOverview, open]);

  const createLocalSnapshot = async () => {
    setDestination("local");
    try {
      const result = await invoke<BackupOperationResult>("create_app_backup", {
        destination: "local",
      });
      message[result.warning ? "warning" : "success"](
        result.warning
          ? t("settings.webdav.cleanupWarning")
          : t("settings.webdav.localBackupSuccess"),
      );
      await loadOverview();
    } catch (error) {
      message.error(isAppError(error) ? error.message : t("settings.webdav.backupFailed"));
    } finally {
      setDestination(null);
    }
  };

  const runSyncNow = async () => {
    setSyncing(true);
    try {
      const outcome = await invoke<SyncOutcome>("sync_now");
      if (outcome.pendingRestart) {
        // 应用云端数据需要重启，命令端会在返回前重启应用。
        return;
      }
      if (outcome.conflict) {
        message.warning(t("settings.webdav.syncConflictApplied"));
      }
      if (outcome.action === "upload") {
        message.success(t("settings.webdav.syncUploadSuccess", { revision: outcome.revision }));
      } else if (outcome.action === "in_sync") {
        message.info(t("settings.webdav.syncInSync"));
      }
      await loadOverview();
    } catch (error) {
      message.error(webdavErrorMessage(error, t, "settings.webdav.syncFailed"));
    } finally {
      setSyncing(false);
    }
  };

  const openBackupSettings = () => {
    useUIStore.getState().setSettingsTab("backup");
    useUIStore.getState().setPage("settings");
    setOpen(false);
  };

  const latestRemote = pickLatestRemoteBackup(remoteBackups);

  const restoreLatestRemote = () => {
    if (!latestRemote) return;
    modal.confirm({
      centered: true,
      title: t("settings.webdav.restoreConfirmTitle"),
      content: t("settings.webdav.restoreConfirmBody", { name: latestRemote.fileName }),
      okText: t("settings.webdav.restoreAction"),
      cancelText: t("common.cancel"),
      okButtonProps: { danger: true },
      onOk: async () => {
        try {
          await invoke("restore_webdav_backup", { fileName: latestRemote.fileName });
        } catch (error) {
          message.error(isAppError(error) ? error.message : t("settings.webdav.restoreFailed"));
          throw error;
        }
      },
    });
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
            <span style={{ color: token.colorTextSecondary }}>{t("settings.webdav.cloudRevision")}</span>
            <span>{overview?.syncRevision ?? "-"}</span>
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
          onClick={() => void createLocalSnapshot()}
        >
          {t("settings.webdav.localBackupNow")}
        </Button>
        <Button
          block
          type="primary"
          icon={<CloudUpload size={14} />}
          loading={syncing}
          disabled={destination !== null || !overview?.webdavConfigured}
          onClick={() => void runSyncNow()}
        >
          {t("settings.webdav.syncNow")}
        </Button>
        <Button
          block
          icon={<CloudDownload size={14} />}
          disabled={destination !== null || !overview?.webdavConfigured || latestRemote == null}
          onClick={restoreLatestRemote}
        >
          {t("settings.webdav.restoreNow")}
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
