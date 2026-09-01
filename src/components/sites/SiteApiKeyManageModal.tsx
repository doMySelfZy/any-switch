import { useEffect, useState } from "react";
import { App, Button, Form, Input, Modal, Popconfirm, Space } from "antd";
import { useTranslation } from "react-i18next";
import type { Site } from "@/types/domain";
import { useSiteStore } from "@/stores";
import { copyText } from "@/lib/copyText";
import { isAppError } from "@/lib/invoke";
import { siteApiKeys } from "@/lib/siteApiKey";

interface Props {
  open: boolean;
  site: Site;
  onClose: () => void;
}

type Mode = "list" | "add" | "rename" | "rotate";

export function SiteApiKeyManageModal({ open, site, onClose }: Props) {
  const { t } = useTranslation();
  const { message } = App.useApp();
  const getSiteApiKey = useSiteStore((s) => s.getSiteApiKey);
  const addApiKey = useSiteStore((s) => s.addApiKey);
  const updateApiKey = useSiteStore((s) => s.updateApiKey);
  const deleteApiKey = useSiteStore((s) => s.deleteApiKey);
  const [form] = Form.useForm();
  const [mode, setMode] = useState<Mode>("list");
  const [targetId, setTargetId] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const keys = siteApiKeys(site);

  useEffect(() => {
    if (!open) {
      setMode("list");
      setTargetId(null);
      form.resetFields();
    }
  }, [open, form]);

  const closeForm = () => {
    form.resetFields();
    setMode("list");
    setTargetId(null);
  };

  const handleSave = async () => {
    const values = await form.validateFields();
    setSaving(true);
    try {
      if (mode === "add") {
        await addApiKey(site.id, { label: values.label, apiKey: values.apiKey });
        message.success(t("sites.keyAdded"));
      } else if (mode === "rename" && targetId) {
        await updateApiKey(site.id, targetId, { label: values.label });
        message.success(t("sites.keyRenamed"));
      } else if (mode === "rotate" && targetId) {
        await updateApiKey(site.id, targetId, { apiKey: values.apiKey });
        message.success(t("sites.keyRotated"));
      }
      closeForm();
    } catch (e) {
      if (e && typeof e === "object" && "errorFields" in e) return;
      message.error(isAppError(e) ? e.message : String(e));
    } finally {
      setSaving(false);
    }
  };

  const handleCopy = async (apiKeyId: string) => {
    try {
      const secret = await getSiteApiKey(site.id, apiKeyId);
      await copyText(secret);
      message.success(t("common.copied"));
    } catch (e) {
      message.error(isAppError(e) ? e.message : String(e));
    }
  };

  const handleDelete = async (apiKeyId: string) => {
    try {
      await deleteApiKey(site.id, apiKeyId);
      message.success(t("sites.keyDeleted"));
    } catch (e) {
      message.error(isAppError(e) ? e.message : String(e));
    }
  };

  return (
    <Modal
      open={open}
      centered
      destroyOnHidden
      mask={{ enabled: true, blur: true }}
      width={520}
      title={t("sites.manageKeys")}
      onCancel={() => {
        form.resetFields();
        onClose();
      }}
      footer={
        mode === "list"
          ? [
              <Button key="close" onClick={onClose}>
                {t("common.close")}
              </Button>,
              <Button key="add" type="primary" onClick={() => setMode("add")}>
                {t("sites.addKey")}
              </Button>,
            ]
          : [
              <Button key="back" onClick={closeForm}>
                {t("common.cancel")}
              </Button>,
              <Button key="save" type="primary" loading={saving} onClick={() => void handleSave()}>
                {t("common.save")}
              </Button>,
            ]
      }
    >
      {mode === "list" ? (
        <div className="space-y-2">
          {keys.map((key) => (
            <div
              key={key.id}
              className="flex items-center justify-between gap-3 rounded-lg border px-3 py-2"
              style={{ borderColor: "var(--border-color)" }}
            >
              <div className="min-w-0">
                <div className="truncate text-sm">
                  {key.label}
                  {key.isActive ? ` · ${t("sites.currentKey")}` : ""}
                </div>
                <div className="font-mono text-xs opacity-60">{key.keyPrefix}</div>
              </div>
              <Space size={4}>
                <Button size="small" onClick={() => void handleCopy(key.id)}>
                  {t("common.copy")}
                </Button>
                <Button
                  size="small"
                  onClick={() => {
                    setTargetId(key.id);
                    form.setFieldsValue({ label: key.label, apiKey: "" });
                    setMode("rename");
                  }}
                >
                  {t("sites.renameKey")}
                </Button>
                <Button
                  size="small"
                  onClick={() => {
                    setTargetId(key.id);
                    form.setFieldsValue({ apiKey: "" });
                    setMode("rotate");
                  }}
                >
                  {t("sites.rotateKey")}
                </Button>
                <Popconfirm
                  title={t("sites.deleteKeyConfirm")}
                  onConfirm={() => void handleDelete(key.id)}
                >
                  <Button size="small" danger disabled={key.isActive || keys.length <= 1}>
                    {t("common.delete")}
                  </Button>
                </Popconfirm>
              </Space>
            </div>
          ))}
        </div>
      ) : (
        <Form form={form} layout="vertical">
          {mode !== "rotate" && (
            <Form.Item name="label" label={t("sites.keyName")} rules={mode === "add" ? [] : [{ required: true }]}>
              <Input allowClear placeholder={t("sites.keyNamePlaceholder")} />
            </Form.Item>
          )}
          {mode !== "rename" && (
            <Form.Item name="apiKey" label={t("sites.apiKey")} rules={[{ required: true }]}>
              <Input.Password allowClear autoComplete="off" />
            </Form.Item>
          )}
        </Form>
      )}
    </Modal>
  );
}
