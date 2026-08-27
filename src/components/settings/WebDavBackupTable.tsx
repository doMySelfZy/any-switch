import { Button, Space, Table, Tag, Tooltip, Typography } from "antd";
import { Trash2, Undo2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { RemoteBackupInfo } from "@/types/domain";

const { Text } = Typography;

interface WebDavBackupTableProps {
  backups: RemoteBackupInfo[];
  loading: boolean;
  selectedFileNames: string[];
  onSelectionChange: (fileNames: string[]) => void;
  onRestore: (backup: RemoteBackupInfo) => void;
  onDelete: (backup: RemoteBackupInfo) => void;
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

export function WebDavBackupTable({
  backups,
  loading,
  selectedFileNames,
  onSelectionChange,
  onRestore,
  onDelete,
}: WebDavBackupTableProps) {
  const { t } = useTranslation();

  const columns = [
    {
      title: t("settings.webdav.remoteFile"),
      dataIndex: "fileName",
      key: "fileName",
      ellipsis: { showTitle: false },
      render: (value: string) => (
        <Tooltip title={value}>
          <Text style={{ fontSize: 12 }}>{value}</Text>
        </Tooltip>
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
      title: t("settings.webdav.device"),
      dataIndex: "deviceName",
      key: "deviceName",
      width: 140,
      ellipsis: true,
      render: (value: string) => <Tag>{value}</Tag>,
    },
    {
      title: "",
      key: "actions",
      width: 80,
      render: (_: unknown, backup: RemoteBackupInfo) => (
        <Space size="small">
          <Tooltip title={t("settings.webdav.restoreAction")}>
            <Button
              size="small"
              aria-label={t("settings.webdav.restoreAction")}
              icon={<Undo2 size={14} />}
              onClick={() => onRestore(backup)}
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
    <Table<RemoteBackupInfo>
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
      locale={{ emptyText: t("settings.webdav.noRemoteBackups") }}
    />
  );
}
