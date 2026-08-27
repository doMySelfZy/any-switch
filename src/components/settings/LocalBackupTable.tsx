import { Button, Space, Table, Tag, Tooltip, Typography } from "antd";
import { FolderOpen, Trash2, Undo2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { LocalBackupInfo } from "@/types/domain";

const { Text } = Typography;

interface LocalBackupTableProps {
  backups: LocalBackupInfo[];
  loading: boolean;
  selectedFileNames: string[];
  canReveal: boolean;
  onSelectionChange: (fileNames: string[]) => void;
  onRestore: (backup: LocalBackupInfo) => void;
  onReveal: (backup: LocalBackupInfo) => void;
  onDelete: (backup: LocalBackupInfo) => void;
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  const units = ["KB", "MB", "GB"];
  let value = bytes / 1024;
  let unit = units[0];
  for (let index = 1; index < units.length && value >= 1024; index += 1) {
    value /= 1024;
    unit = units[index];
  }
  return `${value.toFixed(1)} ${unit}`;
}

function formatTimestamp(value: number | null): string {
  if (value == null) return "-";
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? "-" : date.toLocaleString();
}

export function LocalBackupTable({
  backups,
  loading,
  selectedFileNames,
  canReveal,
  onSelectionChange,
  onRestore,
  onReveal,
  onDelete,
}: LocalBackupTableProps) {
  const { t } = useTranslation();

  const columns = [
    {
      title: t("settings.webdav.modifiedAt"),
      dataIndex: "createdAt",
      key: "createdAt",
      width: 180,
      render: formatTimestamp,
    },
    {
      title: t("settings.backupCenter.format"),
      key: "format",
      width: 100,
      render: (_: unknown, backup: LocalBackupInfo) =>
        backup.error ? (
          <Tooltip title={backup.error}>
            <Tag color="error">{t("settings.backupCenter.invalid")}</Tag>
          </Tooltip>
        ) : (
          <Tag>ZIP</Tag>
        ),
    },
    {
      title: t("settings.webdav.size"),
      dataIndex: "size",
      key: "size",
      width: 100,
      render: (value: number) => <Text type="secondary">{formatSize(value)}</Text>,
    },
    {
      title: t("settings.webdav.remoteFile"),
      dataIndex: "fileName",
      key: "fileName",
      ellipsis: { showTitle: false },
      render: (value: string) => (
        <Tooltip title={value}>
          <Text type="secondary" style={{ fontSize: 12 }}>
            {value}
          </Text>
        </Tooltip>
      ),
    },
    {
      title: t("settings.version"),
      dataIndex: "appVersion",
      key: "appVersion",
      width: 80,
      render: (value: string | null) => (
        <Text type="secondary">{value ? `v${value}` : "-"}</Text>
      ),
    },
    {
      title: "",
      key: "actions",
      width: 120,
      render: (_: unknown, backup: LocalBackupInfo) => (
        <Space size="small">
          <Tooltip title={backup.error ?? t("settings.webdav.restoreAction")}>
            <Button
              size="small"
              disabled={Boolean(backup.error)}
              aria-label={t("settings.webdav.restoreAction")}
              icon={<Undo2 size={14} />}
              onClick={() => onRestore(backup)}
            />
          </Tooltip>
          <Tooltip title={t("settings.backupCenter.openFolder")}>
            <Button
              size="small"
              disabled={!canReveal}
              aria-label={t("settings.backupCenter.openFolder")}
              icon={<FolderOpen size={14} />}
              onClick={() => onReveal(backup)}
            />
          </Tooltip>
          <Tooltip title={t("common.delete")}>
            <Button
              size="small"
              danger
              aria-label={t("common.delete")}
              icon={<Trash2 size={14} />}
              onClick={() => onDelete(backup)}
            />
          </Tooltip>
        </Space>
      ),
    },
  ];

  return (
    <Table<LocalBackupInfo>
      rowKey="fileName"
      size="small"
      pagination={false}
      loading={loading}
      dataSource={backups}
      columns={columns}
      rowSelection={{
        selectedRowKeys: selectedFileNames,
        onChange: (keys) => onSelectionChange(keys as string[]),
      }}
      locale={{ emptyText: t("settings.backupCenter.localNoBackups") }}
    />
  );
}
