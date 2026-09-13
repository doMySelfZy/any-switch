import { useCallback, useEffect, useMemo, useState } from "react";
import {
  App,
  Button,
  Card,
  Checkbox,
  Empty,
  Form,
  Input,
  Modal,
  Segmented,
  Space,
  Table,
  Tag,
  Tooltip,
  Typography,
  theme,
} from "antd";
import {
  CloudUploadOutlined,
  DeleteOutlined,
  EditOutlined,
  InfoCircleOutlined,
  PlusOutlined,
} from "@ant-design/icons";
import { useTranslation } from "react-i18next";
import { invoke } from "@/lib/invoke";
import { useMcpStore } from "@/stores";
import type { McpKind, McpServerInput, McpServerSummary } from "@/types/mcp";
import type { TargetKind } from "@/types/domain";

const TARGETS: TargetKind[] = ["claude_code", "codex", "pi", "prime"];

const TARGET_LABEL_KEYS: Record<TargetKind, string> = {
  claude_code: "mcp.targetClaudeCode",
  codex: "mcp.targetCodex",
  pi: "mcp.targetPi",
  prime: "mcp.targetPrime",
};

const KIND_OPTIONS: { label: string; value: McpKind }[] = [
  { value: "stdio", label: "stdio" },
  { value: "sse", label: "SSE" },
  { value: "http", label: "HTTP" },
];

interface FormValues {
  name: string;
  kind: McpKind;
  enabled: boolean;
  targets: TargetKind[];
  config: string;
  env: string;
  headers: string;
}

function parseJsonField(value: string, field: string, t: (key: string, options?: Record<string, unknown>) => string) {
  const trimmed = (value ?? "").trim();
  if (!trimmed) return {};
  try {
    const parsed: unknown = JSON.parse(trimmed);
    if (typeof parsed !== "object" || parsed === null || Array.isArray(parsed)) {
      throw new Error("not an object");
    }
    return parsed as Record<string, unknown>;
  } catch {
    throw new Error(t("mcp.invalidJson", { field }));
  }
}

/** 统一的错误文案提取：AppError 走 message，其它错误退回字符串。 */
function errorText(error: unknown): string {
  if (error instanceof Error) return error.message;
  const message = (error as { message?: string } | null)?.message;
  return message ?? String(error);
}

export function McpPage() {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const { message, modal } = App.useApp();
  const { servers, loading, loadServers, getServer, saveServer, deleteServer, applyServers } =
    useMcpStore();
  const [open, setOpen] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [applying, setApplying] = useState(false);
  const [targetPaths, setTargetPaths] = useState<[TargetKind, string][]>([]);
  const [form] = Form.useForm<FormValues>();

  useEffect(() => {
    void loadServers();
    void invoke<[TargetKind, string][]>("mcp_target_paths")
      .then(setTargetPaths)
      .catch(() => setTargetPaths([]));
  }, [loadServers]);

  const targetLabel = useCallback(
    (target: TargetKind) => t(TARGET_LABEL_KEYS[target] ?? target),
    [t],
  );

  const openCreate = () => {
    setEditingId(null);
    form.setFieldsValue({
      name: "",
      kind: "stdio",
      enabled: true,
      targets: [],
      config: "{\n  \"command\": \"npx\",\n  \"args\": [\"-y\", \"@modelcontextprotocol/server-filesystem\"]\n}",
      env: "{}",
      headers: "{}",
    });
    setOpen(true);
  };

  const openEdit = async (id: string) => {
    try {
      const server = await getServer(id);
      setEditingId(server.id);
      form.setFieldsValue({
        name: server.name,
        kind: server.kind,
        enabled: server.enabled,
        targets: server.targets,
        config: JSON.stringify(server.config ?? {}, null, 2),
        env: JSON.stringify(server.env ?? {}, null, 2),
        headers: JSON.stringify(server.headers ?? {}, null, 2),
      });
      setOpen(true);
    } catch (error) {
      // 读详情失败时不要把 rejection 抛给 onClick 的 void 调用。
      void message.error(errorText(error));
    }
  };

  const handleSave = async () => {
    let values: FormValues;
    try {
      values = await form.validateFields();
    } catch {
      // 校验失败时 antd 已在表单上展示错误；这里吞掉 rejection，
      // 否则 onOk 的 void 调用会产生未处理的 Promise。
      return;
    }
    let input: McpServerInput;
    try {
      input = {
        id: editingId ?? undefined,
        name: values.name.trim(),
        kind: values.kind,
        enabled: values.enabled,
        targets: values.targets ?? [],
        config: parseJsonField(values.config, t("mcp.config"), t),
        env: parseJsonField(values.env, t("mcp.env"), t),
        headers: parseJsonField(values.headers, t("mcp.headers"), t),
      };
    } catch (error) {
      void message.error(errorText(error));
      return;
    }
    try {
      const { sweep } = await saveServer(input);
      setOpen(false);
      void message.success(t("common.success"));
      // 改名/禁用/删除可能清掉了旧客户端条目；只在确实动过目标时展示结果。
      if (sweep.results.some((item) => !item.ok)) {
        void message.warning(t("mcp.applyPartial"));
      }
      showApplyOutcome(sweep);
    } catch (error) {
      void message.error(errorText(error));
    }
  };

  const handleDelete = (record: McpServerSummary) => {
    modal.confirm({
      centered: true,
      title: t("mcp.delete"),
      content: (
        <div>
          <div>{t("mcp.deleteConfirm", { name: record.name })}</div>
          <div style={{ marginTop: 8, color: token.colorTextTertiary }}>
            {t("mcp.deleteCleansTargets")}
          </div>
        </div>
      ),
      okButtonProps: { danger: true },
      // 失败时必须自己消化 rejection：antd 会把它重新抛出，页面既不提示也关不掉弹窗。
      onOk: async () => {
        try {
          const result = await deleteServer(record.id);
          void message.success(t("common.success"));
          showApplyOutcome(result);
        } catch (error) {
          void message.error(errorText(error));
        }
      },
    });
  };

  const showApplyOutcome = (result: { results: { target: TargetKind; ok: boolean; message: string }[] }) => {
    if (result.results.length === 0) return;
    modal.info({
      centered: true,
      title: t("mcp.applyResultTitle"),
      width: 520,
      content: (
        <div className="flex flex-col gap-2">
          {result.results.map((item) => (
            <div key={item.target}>
              {item.ok ? (
                <span>
                  {t("mcp.targetSuccess", { target: targetLabel(item.target) })}
                  {item.message ? ` — ${item.message}` : ""}
                </span>
              ) : (
                <span style={{ color: token.colorError }}>
                  {t("mcp.targetFailed", {
                    target: targetLabel(item.target),
                    message: item.message,
                  })}
                </span>
              )}
            </div>
          ))}
        </div>
      ),
      okText: t("common.confirm"),
    });
  };

  const applyTargets = async (targets: TargetKind[]) => {
    if (targets.length === 0) {
      void message.warning(t("mcp.noTargets"));
      return;
    }
    setApplying(true);
    try {
      const result = await applyServers(targets);
      const failed = result.results.filter((item) => !item.ok);
      if (failed.length === 0) {
        void message.success(t("mcp.applySuccess"));
      } else if (failed.length < result.results.length) {
        void message.warning(t("mcp.applyPartial"));
      } else {
        void message.error(t("mcp.applyFailed"));
      }
      showApplyOutcome(result);
    } catch (error) {
      void message.error(
        error instanceof Error ? error.message : String((error as { message?: string })?.message ?? error),
      );
    } finally {
      setApplying(false);
    }
  };

  // 只对「有启用的服务指向它」的目标做应用，避免清掉用户手动维护的客户端 MCP。
  const activeTargets = useMemo(() => {
    const set = new Set<TargetKind>();
    servers
      .filter((server) => server.enabled)
      .forEach((server) => server.targets.forEach((target) => set.add(target)));
    return TARGETS.filter((target) => set.has(target));
  }, [servers]);

  const columns = [
    {
      title: t("mcp.name"),
      dataIndex: "name",
      key: "name",
      render: (name: string) => <Typography.Text strong>{name}</Typography.Text>,
    },
    {
      title: t("mcp.kind"),
      dataIndex: "kind",
      key: "kind",
      width: 90,
      render: (kind: McpKind) => <Tag>{kind}</Tag>,
    },
    {
      title: t("mcp.targets"),
      dataIndex: "targets",
      key: "targets",
      render: (targets: TargetKind[]) =>
        targets.length === 0 ? (
          <Typography.Text type="secondary">—</Typography.Text>
        ) : (
          <Space size={4} wrap>
            {targets.map((target) => (
              <Tag key={target}>{targetLabel(target)}</Tag>
            ))}
          </Space>
        ),
    },
    {
      title: t("mcp.enabled"),
      dataIndex: "enabled",
      key: "enabled",
      width: 90,
      render: (enabled: boolean) =>
        enabled ? <Tag color="green">{t("mcp.enabled")}</Tag> : <Tag>{t("mcp.disabled")}</Tag>,
    },
    {
      title: t("common.actions"),
      key: "actions",
      render: (_: unknown, record: McpServerSummary) => (
        <Space size={0}>
          <Tooltip title={t("common.edit")}>
            <Button
              type="text"
              size="small"
              aria-label={t("common.edit")}
              icon={<EditOutlined />}
              onClick={() => void openEdit(record.id)}
            />
          </Tooltip>
          <Tooltip title={t("common.delete")}>
            <Button
              type="text"
              size="small"
              danger
              aria-label={t("common.delete")}
              icon={<DeleteOutlined />}
              onClick={() => handleDelete(record)}
            />
          </Tooltip>
        </Space>
      ),
    },
  ];

  return (
    <div className="flex h-full min-h-0 flex-col gap-4 overflow-auto p-6">
      <div className="flex items-center justify-between gap-4">
        <div>
          <Typography.Title level={4} style={{ margin: 0 }}>
            {t("mcp.title")}
          </Typography.Title>
          <Typography.Text type="secondary">{t("mcp.emptyDesc")}</Typography.Text>
        </div>
        <Space>
          <Button
            type="primary"
            icon={<CloudUploadOutlined />}
            loading={applying}
            disabled={activeTargets.length === 0}
            onClick={() => void applyTargets(activeTargets)}
          >
            {t("mcp.applyToTargets")}
          </Button>
          <Button icon={<PlusOutlined />} onClick={openCreate}>
            {t("mcp.add")}
          </Button>
        </Space>
      </div>

      <Card
        size="small"
        styles={{ body: { display: "flex", gap: 8, alignItems: "flex-start" } }}
      >
        <InfoCircleOutlined style={{ color: token.colorTextTertiary, marginTop: 2 }} />
        <div>
          <Typography.Text type="secondary" style={{ fontSize: 12 }}>
            {t("mcp.secretsNotice")}
          </Typography.Text>
          <details style={{ marginTop: 6 }}>
            <summary style={{ cursor: "pointer", fontSize: 12, color: token.colorTextTertiary }}>
              {t("mcp.pathsTitle")}
            </summary>
            <div style={{ marginTop: 4 }}>
              {targetPaths.map(([target, path]) => (
                <div key={target} style={{ fontSize: 12 }}>
                  <Typography.Text type="secondary">
                    {targetLabel(target)}: <Typography.Text code>{path}</Typography.Text>
                  </Typography.Text>
                </div>
              ))}
            </div>
          </details>
        </div>
      </Card>

      {servers.length === 0 && !loading ? (
        <Card>
          <Empty description={t("mcp.emptyTitle")}>
            <Button type="primary" icon={<PlusOutlined />} onClick={openCreate}>
              {t("mcp.add")}
            </Button>
          </Empty>
        </Card>
      ) : (
        <Table
          columns={columns}
          dataSource={servers}
          rowKey="id"
          loading={loading}
          pagination={false}
          size="small"
        />
      )}

      <Modal
        centered
        destroyOnHidden
        mask={{ enabled: true, blur: true }}
        width={560}
        open={open}
        title={editingId ? t("mcp.edit") : t("mcp.add")}
        onCancel={() => setOpen(false)}
        onOk={() => void handleSave()}
        okText={t("common.save")}
        cancelText={t("common.cancel")}
      >
        <Form form={form} layout="vertical">
          <Form.Item
            name="name"
            label={t("mcp.name")}
            rules={[
              { required: true, message: t("mcp.nameRequired") },
              { pattern: /^[A-Za-z0-9_-]+$/, message: t("mcp.nameRule") },
            ]}
            extra={t("mcp.nameRule")}
          >
            <Input allowClear placeholder="filesystem" />
          </Form.Item>
          <Form.Item name="kind" label={t("mcp.kind")}>
            <Segmented options={KIND_OPTIONS} />
          </Form.Item>
          <Form.Item name="targets" label={t("mcp.targets")}>
            <Checkbox.Group
              options={TARGETS.map((target) => ({
                label: targetLabel(target),
                value: target,
              }))}
            />
          </Form.Item>
          <Form.Item
            name="config"
            label={t("mcp.config")}
            extra={t("mcp.configHint")}
          >
            <Input.TextArea rows={5} autoSize={{ minRows: 4, maxRows: 12 }} />
          </Form.Item>
          <Form.Item name="env" label={t("mcp.env")}>
            <Input.TextArea rows={3} autoSize={{ minRows: 2, maxRows: 8 }} />
          </Form.Item>
          <Form.Item name="headers" label={t("mcp.headers")}>
            <Input.TextArea rows={3} autoSize={{ minRows: 2, maxRows: 8 }} />
          </Form.Item>
          <Form.Item name="enabled" valuePropName="checked">
            <Checkbox>{t("mcp.enabled")}</Checkbox>
          </Form.Item>
        </Form>
      </Modal>
    </div>
  );
}
