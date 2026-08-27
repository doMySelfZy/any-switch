import { useCallback, useEffect, useState } from "react";
import { App, Button, Space, Spin, Tag, Typography } from "antd";
import { Cloud, CloudUpload, RefreshCw, Settings2, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { invoke, isAppError } from "@/lib/invoke";
import type {
  BackupOperationResult,
  BackupOverview,
  RemoteBackupInfo,
  WebDavConfigView,
} from "@/types/domain";
import { WebDavBackupTable } from "./WebDavBackupTable";
import { WebDavConfigModal } from "./WebDavConfigModal";

const { Text } = Typography;

const DEFAULT_CONFIG: WebDavConfigView = {
  baseUrl: "",
  username: "",
  remotePath: "xiaobai-switch",
  acceptInvalidCerts: false,
  hasPassword: false,
  autoSyncEnabled: false,
  syncIntervalMinutes: 60,
  maxRemoteBackups: 10,
};

function formatTimestamp(value: number | string | null): string | null {
  if (value == null || value === "") return null;
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? null : date.toLocaleString();
}

export function WebDavBackupSettings() {
  const { t } = useTranslation();
  const { message, modal } = App.useApp();
  const [config, setConfig] = useState<WebDavConfigView>(DEFAULT_CONFIG);
  const [overview, setOverview] = useState<BackupOverview | null>(null);
  const [remoteBackups, setRemoteBackups] = useState<RemoteBackupInfo[]>([]);
  const [selectedFileNames, setSelectedFileNames] = useState<string[]>([]);
  const [configOpen, setConfigOpen] = useState(false);
  const [loading, setLoading] = useState(true);
  const [syncing, setSyncing] = useState(false);
  const [remoteLoading, setRemoteLoading] = useState(false);

  const loadOverview = useCallback(async () => {
    setOverview(await invoke<BackupOverview>("get_backup_overview"));
  }, []);

  const loadRemoteBackups = useCallback(async () => {
    setRemoteLoading(true);
    try {
      setRemoteBackups(await invoke<RemoteBackupInfo[]>("list_webdav_backups"));
    } catch (error) {
      message.error(isAppError(error) ? error.message : t("settings.webdav.loadFailed"));
    } finally {
      setRemoteLoading(false);
    }
  }, [message, t]);

  useEffect(() => {
    let cancelled = false;
    void Promise.all([
      invoke<WebDavConfigView>("get_webdav_config"),
      invoke<BackupOverview>("get_backup_overview"),
    ])
      .then(([nextConfig, nextOverview]) => {
        if (cancelled) return;
        setConfig(nextConfig);
        setOverview(nextOverview);
        if (nextConfig.baseUrl) void loadRemoteBackups();
      })
      .catch((error) => {
        if (!cancelled) {
          message.error(isAppError(error) ? error.message : t("settings.webdav.loadFailed"));
        }
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [loadRemoteBackups, message, t]);

  const handleConfigSaved = async (saved: WebDavConfigView) => {
    setConfig(saved);
    setConfigOpen(false);
    await loadOverview();
    void loadRemoteBackups();
  };

  const handleBackupNow = async () => {
    setSyncing(true);
    try {
      const result = await invoke<BackupOperationResult>("create_app_backup", {
        destination: "webdav",
      });
      if (result.warning) message.warning(t("settings.webdav.cleanupWarning"));
      else message.success(t("settings.webdav.backupSuccess"));
      await Promise.all([loadOverview(), loadRemoteBackups()]);
    } catch (error) {
      message.error(isAppError(error) ? error.message : t("settings.webdav.backupFailed"));
    } finally {
      setSyncing(false);
    }
  };

  const handleDelete = (backup: RemoteBackupInfo) => {
    modal.confirm({
      centered: true,
      title: t("settings.webdav.deleteConfirmTitle"),
      content: t("settings.webdav.deleteConfirmBody", { name: backup.fileName }),
      okText: t("common.delete"),
      cancelText: t("common.cancel"),
      okButtonProps: { danger: true },
      onOk: async () => {
        await invoke("delete_webdav_backup", { fileName: backup.fileName });
        setSelectedFileNames((names) => names.filter((name) => name !== backup.fileName));
        message.success(t("settings.webdav.deleteSuccess"));
        await loadRemoteBackups();
      },
    });
  };

  const handleBatchDelete = () => {
    if (selectedFileNames.length === 0) return;
    modal.confirm({
      centered: true,
      title: t("settings.backupCenter.batchDeleteConfirm", {
        count: selectedFileNames.length,
      }),
      okText: t("common.delete"),
      cancelText: t("common.cancel"),
      okButtonProps: { danger: true },
      onOk: async () => {
        for (const fileName of selectedFileNames) {
          await invoke("delete_webdav_backup", { fileName });
        }
        setSelectedFileNames([]);
        message.success(t("settings.webdav.deleteSuccess"));
        await loadRemoteBackups();
      },
    });
  };

  const handleRestore = (backup: RemoteBackupInfo) => {
    modal.confirm({
      centered: true,
      title: t("settings.webdav.restoreConfirmTitle"),
      content: t("settings.webdav.restoreConfirmBody", { name: backup.fileName }),
      okText: t("settings.webdav.restoreAction"),
      cancelText: t("common.cancel"),
      okButtonProps: { danger: true },
      onOk: async () => {
        await invoke("restore_webdav_backup", { fileName: backup.fileName });
      },
    });
  };

  const status = overview?.webdavSync.status ?? "never";
  const statusColor =
    status === "success"
      ? "success"
      : status === "warning"
        ? "warning"
        : status === "failed"
          ? "error"
          : "default";
  const isConfigured = Boolean(config.baseUrl);
  const lastSuccess = formatTimestamp(overview?.webdavSync.lastSuccessAt ?? null);

  return (
    <>
      <div className="mb-4 flex items-center justify-between">
        <Space>
          <Button
            danger
            icon={<Trash2 size={16} />}
            disabled={selectedFileNames.length === 0}
            onClick={handleBatchDelete}
          >
            {t("settings.backupCenter.batchDelete", { count: selectedFileNames.length })}
          </Button>
          {isConfigured && lastSuccess && (
            <Text type="secondary" style={{ fontSize: 12 }}>
              {t("settings.webdav.lastSuccess")}: {lastSuccess}{" "}
              <Tag color={statusColor} style={{ marginLeft: 4 }}>
                {t(`settings.webdav.status.${status}`)}
              </Tag>
            </Text>
          )}
        </Space>
        <Space>
          <Button icon={<Settings2 size={16} />} onClick={() => setConfigOpen(true)}>
            {t("settings.webdav.configure")}
          </Button>
          {isConfigured && (
            <>
              <Button
                icon={<RefreshCw size={16} />}
                loading={remoteLoading}
                onClick={() => void loadRemoteBackups()}
              >
                {t("common.refresh")}
              </Button>
              <Button
                type="primary"
                icon={<CloudUpload size={16} />}
                loading={syncing}
                onClick={() => void handleBackupNow()}
              >
                {t("settings.webdav.syncNow")}
              </Button>
            </>
          )}
        </Space>
      </div>

      {loading && !isConfigured ? (
        <div className="flex justify-center py-16">
          <Spin size="small" />
        </div>
      ) : !isConfigured ? (
        <div className="flex flex-col items-center justify-center py-16 opacity-50">
          <Cloud size={48} />
          <Text type="secondary" style={{ marginTop: 12 }}>
            {t("settings.webdav.notConfigured")}
          </Text>
        </div>
      ) : (
        <WebDavBackupTable
          backups={remoteBackups}
          loading={remoteLoading}
          selectedFileNames={selectedFileNames}
          onSelectionChange={setSelectedFileNames}
          onRestore={handleRestore}
          onDelete={handleDelete}
        />
      )}

      <WebDavConfigModal
        open={configOpen}
        config={config}
        onCancel={() => setConfigOpen(false)}
        onSaved={handleConfigSaved}
      />
    </>
  );
}
