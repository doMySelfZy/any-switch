import { useCallback, useEffect, useState } from "react";
import {
  Alert,
  App,
  Button,
  Checkbox,
  Divider,
  Form,
  Input,
  InputNumber,
  Select,
  Space,
  Switch,
  Table,
  Tag,
  theme,
  Typography,
} from "antd";
import { CloudUpload, RefreshCw, RotateCcw, Trash2, Wifi } from "lucide-react";
import { useTranslation } from "react-i18next";
import { invoke, isAppError } from "@/lib/invoke";
import type {
  BackupOperationResult,
  BackupOverview,
  RemoteBackupInfo,
  SaveWebDavConfigInput,
  TestWebDavConnectionInput,
  WebDavConfigView,
} from "@/types/domain";
import { SettingsGroup } from "./SettingsGroup";

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

interface FormValues {
  baseUrl: string;
  username: string;
  password?: string;
  remotePath: string;
  acceptInvalidCerts: boolean;
  autoSyncEnabled: boolean;
  syncIntervalMinutes: number;
  maxRemoteBackups: number;
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

function formatTimestamp(value: number | string | null): string {
  if (value == null || value === "") return "-";
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? String(value) : date.toLocaleString();
}

export function WebDavBackupSettings() {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const { message, modal } = App.useApp();
  const [form] = Form.useForm<FormValues>();
  const [config, setConfig] = useState<WebDavConfigView>(DEFAULT_CONFIG);
  const [overview, setOverview] = useState<BackupOverview | null>(null);
  const [remoteBackups, setRemoteBackups] = useState<RemoteBackupInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [testing, setTesting] = useState(false);
  const [saving, setSaving] = useState(false);
  const [syncing, setSyncing] = useState(false);
  const [remoteLoading, setRemoteLoading] = useState(false);
  const acceptInvalidCerts = Form.useWatch("acceptInvalidCerts", form);

  const loadOverview = useCallback(async () => {
    const value = await invoke<BackupOverview>("get_backup_overview");
    setOverview(value);
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
        form.setFieldsValue({
          baseUrl: nextConfig.baseUrl,
          username: nextConfig.username,
          password: "",
          remotePath: nextConfig.remotePath,
          acceptInvalidCerts: nextConfig.acceptInvalidCerts,
          autoSyncEnabled: nextConfig.autoSyncEnabled,
          syncIntervalMinutes: nextConfig.syncIntervalMinutes,
          maxRemoteBackups: nextConfig.maxRemoteBackups,
        });
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
  }, [form, loadRemoteBackups, message, t]);

  const valuesToConnection = (values: FormValues): TestWebDavConnectionInput => ({
    baseUrl: values.baseUrl.trim(),
    username: values.username.trim(),
    password: values.password || null,
    remotePath: values.remotePath.trim(),
    acceptInvalidCerts: values.acceptInvalidCerts,
  });

  const handleTest = async () => {
    setTesting(true);
    try {
      const values = await form.validateFields([
        "baseUrl",
        "username",
        "password",
        "remotePath",
      ]);
      await invoke("test_webdav_connection", { input: valuesToConnection(values as FormValues) });
      message.success(t("settings.webdav.testSuccess"));
    } catch (error) {
      if (isAppError(error)) message.error(error.message);
    } finally {
      setTesting(false);
    }
  };

  const handleSave = async () => {
    setSaving(true);
    try {
      const values = await form.validateFields();
      const input: SaveWebDavConfigInput = {
        ...valuesToConnection(values),
        autoSyncEnabled: values.autoSyncEnabled,
        syncIntervalMinutes: values.syncIntervalMinutes,
        maxRemoteBackups: values.maxRemoteBackups,
      };
      const saved = await invoke<WebDavConfigView>("save_webdav_config", { input });
      setConfig(saved);
      form.setFieldValue("password", "");
      await loadOverview();
      message.success(t("settings.webdav.saveSuccess"));
      void loadRemoteBackups();
    } catch (error) {
      if (isAppError(error)) message.error(error.message);
    } finally {
      setSaving(false);
    }
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

  const columns = [
    {
      title: t("settings.webdav.remoteFile"),
      dataIndex: "fileName",
      key: "fileName",
      ellipsis: true,
    },
    {
      title: t("settings.webdav.device"),
      dataIndex: "deviceName",
      key: "deviceName",
      width: 130,
      render: (value: string) => <Tag>{value}</Tag>,
    },
    {
      title: t("settings.webdav.modifiedAt"),
      dataIndex: "lastModified",
      key: "lastModified",
      width: 180,
      render: formatTimestamp,
    },
    {
      title: t("settings.webdav.size"),
      dataIndex: "size",
      key: "size",
      width: 90,
      render: formatSize,
    },
    {
      title: t("settings.webdav.actions"),
      key: "actions",
      width: 90,
      render: (_: unknown, backup: RemoteBackupInfo) => (
        <Space size={4}>
          <Button
            size="small"
            aria-label={t("settings.webdav.restoreAction")}
            icon={<RotateCcw size={14} />}
            onClick={() => handleRestore(backup)}
          />
          <Button
            size="small"
            danger
            aria-label={t("common.delete")}
            icon={<Trash2 size={14} />}
            onClick={() => handleDelete(backup)}
          />
        </Space>
      ),
    },
  ];

  const statusColor =
    overview?.webdavSync.status === "success"
      ? "success"
      : overview?.webdavSync.status === "warning"
        ? "warning"
        : overview?.webdavSync.status === "failed"
          ? "error"
          : "default";

  return (
    <>
      <SettingsGroup title={t("settings.webdav.title")}>
        <Alert
          type="warning"
          showIcon
          className="mb-4"
          title={t("settings.webdav.securityWarningTitle")}
          description={t("settings.webdav.securityWarningBody")}
        />
        <Form<FormValues>
          form={form}
          layout="vertical"
          disabled={loading}
          initialValues={{ ...DEFAULT_CONFIG, password: "" }}
        >
          <Form.Item
            name="baseUrl"
            label={t("settings.webdav.baseUrl")}
            rules={[{ required: true }]}
          >
            <Input allowClear placeholder="https://dav.example.com/dav/" />
          </Form.Item>
          <div className="grid grid-cols-2 gap-3">
            <Form.Item
              name="username"
              label={t("settings.webdav.username")}
              rules={[{ required: true }]}
            >
              <Input allowClear />
            </Form.Item>
            <Form.Item
              name="password"
              label={t("settings.webdav.password")}
              rules={config.hasPassword ? [] : [{ required: true }]}
              extra={config.hasPassword ? t("settings.webdav.passwordSavedHint") : undefined}
            >
              <Input.Password allowClear autoComplete="new-password" />
            </Form.Item>
          </div>
          <Form.Item
            name="remotePath"
            label={t("settings.webdav.remotePath")}
            rules={[{ required: true }]}
          >
            <Input allowClear placeholder="xiaobai-switch" />
          </Form.Item>
          <Form.Item name="acceptInvalidCerts" valuePropName="checked">
            <Checkbox>{t("settings.webdav.acceptInvalidCerts")}</Checkbox>
          </Form.Item>
          {acceptInvalidCerts && (
            <Alert
              type="error"
              showIcon
              className="mb-4"
              title={t("settings.webdav.invalidCertWarning")}
            />
          )}
          <Divider />
          <div className="grid grid-cols-3 gap-3">
            <Form.Item
              name="autoSyncEnabled"
              label={t("settings.webdav.autoSync")}
              valuePropName="checked"
            >
              <Switch />
            </Form.Item>
            <Form.Item name="syncIntervalMinutes" label={t("settings.webdav.interval") }>
              <Select
                options={[15, 30, 60, 120, 360, 720, 1440].map((value) => ({
                  value,
                  label: t("settings.webdav.intervalMinutes", { count: value }),
                }))}
              />
            </Form.Item>
            <Form.Item name="maxRemoteBackups" label={t("settings.webdav.retention") }>
              <InputNumber min={1} max={100} precision={0} className="w-full" />
            </Form.Item>
          </div>
          <Space>
            <Button icon={<Wifi size={14} />} loading={testing} onClick={() => void handleTest()}>
              {t("settings.webdav.test")}
            </Button>
            <Button type="primary" loading={saving} onClick={() => void handleSave()}>
              {t("settings.save")}
            </Button>
          </Space>
        </Form>
        {overview && (
          <div className="mt-4 text-xs" style={{ color: token.colorTextSecondary }}>
            <Space wrap>
              <Tag color={statusColor}>{t(`settings.webdav.status.${overview.webdavSync.status}`)}</Tag>
              <span>
                {t("settings.webdav.lastAttempt")}: {formatTimestamp(overview.webdavSync.lastAttemptAt)}
              </span>
              <span>
                {t("settings.webdav.lastSuccess")}: {formatTimestamp(overview.webdavSync.lastSuccessAt)}
              </span>
              <span>
                {t("settings.webdav.nextRun")}: {formatTimestamp(overview.nextScheduledAt)}
              </span>
            </Space>
            {overview.webdavSync.error && (
              <Typography.Text type="danger" className="mt-2 block text-xs">
                {overview.webdavSync.error}
              </Typography.Text>
            )}
          </div>
        )}
      </SettingsGroup>

      <SettingsGroup title={t("settings.webdav.remoteBackups")}>
        <div className="mb-3 flex justify-end gap-2">
          <Button
            icon={<RefreshCw size={14} />}
            loading={remoteLoading}
            disabled={!config.baseUrl}
            onClick={() => void loadRemoteBackups()}
          >
            {t("common.refresh")}
          </Button>
          <Button
            type="primary"
            icon={<CloudUpload size={14} />}
            loading={syncing}
            disabled={!config.baseUrl}
            onClick={() => void handleBackupNow()}
          >
            {t("settings.webdav.backupNow")}
          </Button>
        </div>
        <Table<RemoteBackupInfo>
          rowKey="fileName"
          size="small"
          pagination={false}
          loading={remoteLoading}
          dataSource={remoteBackups}
          columns={columns}
          locale={{ emptyText: t("settings.webdav.noRemoteBackups") }}
        />
      </SettingsGroup>
    </>
  );
}
