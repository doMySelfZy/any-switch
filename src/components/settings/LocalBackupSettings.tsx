import { useCallback, useEffect, useState } from "react";
import { App, Button, Space } from "antd";
import { CloudUpload, Settings2, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { invoke, isAppError } from "@/lib/invoke";
import { revealInExplorer } from "@/lib/revealInExplorer";
import type { AppPaths, BackupOperationResult, LocalBackupInfo } from "@/types/domain";
import { LocalBackupConfigModal } from "./LocalBackupConfigModal";
import { LocalBackupTable } from "./LocalBackupTable";

function joinBackupPath(directory: string, fileName: string): string {
  const trimmed = directory.replace(/[/\\]+$/, "");
  const separator = directory.includes("\\") && !directory.includes("/") ? "\\" : "/";
  return `${trimmed}${separator}${fileName}`;
}

export function LocalBackupSettings() {
  const { t } = useTranslation();
  const { message, modal } = App.useApp();
  const [paths, setPaths] = useState<AppPaths | null>(null);
  const [backups, setBackups] = useState<LocalBackupInfo[]>([]);
  const [selectedFileNames, setSelectedFileNames] = useState<string[]>([]);
  const [loading, setLoading] = useState(true);
  const [creating, setCreating] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);

  const loadBackups = useCallback(async () => {
    setLoading(true);
    try {
      setBackups(await invoke<LocalBackupInfo[]>("list_local_backups"));
    } catch (error) {
      message.error(isAppError(error) ? error.message : t("settings.backupCenter.loadFailed"));
    } finally {
      setLoading(false);
    }
  }, [message, t]);

  useEffect(() => {
    let cancelled = false;
    void Promise.all([
      invoke<AppPaths>("get_app_paths"),
      invoke<LocalBackupInfo[]>("list_local_backups"),
    ])
      .then(([nextPaths, nextBackups]) => {
        if (cancelled) return;
        setPaths(nextPaths);
        setBackups(nextBackups);
      })
      .catch((error) => {
        if (!cancelled) {
          message.error(isAppError(error) ? error.message : t("settings.backupCenter.loadFailed"));
        }
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [message, t]);

  const createBackup = async () => {
    setCreating(true);
    try {
      await invoke<BackupOperationResult>("create_app_backup", { destination: "local" });
      await loadBackups();
      message.success(t("settings.webdav.localBackupSuccess"));
    } catch (error) {
      message.error(isAppError(error) ? error.message : t("settings.webdav.backupFailed"));
    } finally {
      setCreating(false);
    }
  };

  const deleteBackups = async (fileNames: string[]) => {
    try {
      for (const fileName of fileNames) {
        await invoke("delete_local_backup", { fileName });
      }
      setSelectedFileNames([]);
      await loadBackups();
      message.success(t("settings.backupCenter.localDeleteSuccess"));
    } catch (error) {
      message.error(
        isAppError(error) ? error.message : t("settings.backupCenter.localDeleteFailed"),
      );
      throw error;
    }
  };

  const handleBatchDelete = () => {
    if (selectedFileNames.length === 0) return;
    modal.confirm({
      centered: true,
      title: t("settings.backupCenter.localBatchDeleteConfirm", {
        count: selectedFileNames.length,
      }),
      okText: t("common.delete"),
      cancelText: t("common.cancel"),
      okButtonProps: { danger: true },
      onOk: () => deleteBackups(selectedFileNames),
    });
  };

  const handleDelete = (backup: LocalBackupInfo) => {
    modal.confirm({
      centered: true,
      title: t("settings.backupCenter.localDeleteConfirmTitle"),
      content: t("settings.backupCenter.localDeleteConfirmBody", { name: backup.fileName }),
      okText: t("common.delete"),
      cancelText: t("common.cancel"),
      okButtonProps: { danger: true },
      onOk: () => deleteBackups([backup.fileName]),
    });
  };

  const handleRestore = (backup: LocalBackupInfo) => {
    modal.confirm({
      centered: true,
      title: t("settings.webdav.restoreConfirmTitle"),
      content: t("settings.webdav.restoreConfirmBody", { name: backup.fileName }),
      okText: t("settings.webdav.restoreAction"),
      cancelText: t("common.cancel"),
      okButtonProps: { danger: true },
      onOk: async () => {
        try {
          await invoke("restore_local_backup", { fileName: backup.fileName });
        } catch (error) {
          message.error(isAppError(error) ? error.message : t("settings.webdav.restoreFailed"));
          throw error;
        }
      },
    });
  };

  const handleReveal = (backup: LocalBackupInfo) => {
    if (!paths) return;
    void revealInExplorer(joinBackupPath(paths.appBackupsDir, backup.fileName));
  };

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
        </Space>
        <Space>
          <Button icon={<Settings2 size={16} />} onClick={() => setSettingsOpen(true)}>
            {t("settings.backupCenter.localSettings")}
          </Button>
          <Button
            type="primary"
            icon={<CloudUpload size={16} />}
            loading={creating}
            onClick={() => void createBackup()}
          >
            {t("settings.backupCenter.create")}
          </Button>
        </Space>
      </div>

      <LocalBackupTable
        backups={backups}
        loading={loading}
        selectedFileNames={selectedFileNames}
        canReveal={Boolean(paths)}
        onSelectionChange={setSelectedFileNames}
        onRestore={handleRestore}
        onReveal={handleReveal}
        onDelete={handleDelete}
      />

      <LocalBackupConfigModal
        open={settingsOpen}
        directory={paths?.appBackupsDir ?? ""}
        onCancel={() => setSettingsOpen(false)}
        onOpenDirectory={() => {
          if (paths) void invoke("open_path", { path: paths.appBackupsDir });
        }}
      />
    </>
  );
}
