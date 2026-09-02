import { useEffect, useState } from "react";
import { App, Collapse, Form, Input, Modal, Select } from "antd";
import { useTranslation } from "react-i18next";
import type { Site, SiteCapabilities, SiteProtocol } from "@/types/domain";
import { useSiteStore } from "@/stores";
import { UrlWritePreviewIcon } from "./UrlWritePreview";
import { ApiKeyListInput, loadSiteKeyDrafts, normalizeApiKeyDrafts } from "./ApiKeyListInput";
import { BaseUrlListInput } from "./BaseUrlListInput";
import { isAppError } from "@/lib/invoke";
import { invalidateSiteIconCache } from "@/lib/siteIcon";
import { siteApiKeys } from "@/lib/siteApiKey";
import { normalizeBaseUrls, siteBaseUrls } from "@/lib/urlNormalize";
import {
  anyCodexCapabilityOn,
  capabilitiesFromCodexFlags,
  codexFlagsFromCapabilities,
  EMPTY_CODEX_FLAGS,
  mergeCodexCapabilities,
  type CodexCapabilityFlags,
} from "@/lib/siteCapabilities";
import { CodexCapabilitySwitchList } from "@/components/apply/CodexCapabilitySwitchList";

function toActiveKeys(keys: string | string[]): string[] {
  return Array.isArray(keys) ? keys.map(String) : [String(keys)];
}

function shouldOpenAdvanced(protocol?: SiteProtocol | null, notes?: string | null) {
  return protocol === "anthropic" || Boolean(notes?.trim());
}

export interface SiteFormInitialValues {
  name?: string;
  baseUrls?: string[];
  apiKey?: string | null;
  protocol?: SiteProtocol;
  notes?: string | null;
  capabilities?: SiteCapabilities;
}

interface Props {
  open: boolean;
  site?: Site | null;
  initialValues?: SiteFormInitialValues | null;
  onClose: () => void;
  onSaved?: (site: Site, isCreate: boolean) => void;
}

export function SiteFormModal({ open, site, initialValues, onClose, onSaved }: Props) {
  const { t } = useTranslation();
  const { message } = App.useApp();
  const getSiteApiKey = useSiteStore((s) => s.getSiteApiKey);
  const createSite = useSiteStore((s) => s.createSite);
  const updateSite = useSiteStore((s) => s.updateSite);
  const [form] = Form.useForm();
  const [saving, setSaving] = useState(false);
  const [keyLoading, setKeyLoading] = useState(false);
  const [codexFlags, setCodexFlags] = useState<CodexCapabilityFlags>(EMPTY_CODEX_FLAGS);
  const [advancedOpen, setAdvancedOpen] = useState<string[]>([]);
  const [capOpen, setCapOpen] = useState<string[]>([]);
  const watchedUrls = Form.useWatch("baseUrls", form) as string[] | undefined;
  const previewUrl = watchedUrls?.find((u) => String(u ?? "").trim()) ?? "";

  useEffect(() => {
    if (!open) return;
    let cancelled = false;
    const caps = site?.capabilities ?? initialValues?.capabilities ?? {};
    const flags = codexFlagsFromCapabilities(caps);
    const protocol = site?.protocol ?? initialValues?.protocol ?? "openai_compatible";
    const notes = site ? (site.notes ?? "") : (initialValues?.notes ?? "");
    setCodexFlags(flags);
    setAdvancedOpen(shouldOpenAdvanced(protocol, notes) ? ["advanced"] : []);
    setCapOpen(anyCodexCapabilityOn(caps) ? ["codex"] : []);
    if (site) {
      const currentKeys = siteApiKeys(site);
      const summaries = [
        ...currentKeys.filter((key) => key.isActive),
        ...currentKeys.filter((key) => !key.isActive),
      ];
      setKeyLoading(summaries.length > 0 || site.hasKey);
      form.setFieldsValue({
        name: site.name,
        baseUrls: siteBaseUrls(site),
        protocol: site.protocol,
        notes: site.notes ?? "",
        apiKeys: summaries.length
          ? summaries.map((key) => ({ id: key.id, label: key.label, apiKey: "" }))
          : [{ label: "", apiKey: "" }],
      });
      void loadSiteKeyDrafts(site, getSiteApiKey)
        .then((apiKeys) => {
          if (!cancelled) form.setFieldValue("apiKeys", apiKeys);
        })
        .catch((error: unknown) => {
          if (cancelled) return;
          message.error(isAppError(error) ? error.message : t("sites.apiKeyLoadFailed"));
        })
        .finally(() => {
          if (!cancelled) setKeyLoading(false);
        });
    } else {
      setKeyLoading(false);
      form.resetFields();
      form.setFieldsValue({
        protocol: initialValues?.protocol ?? "openai_compatible",
        baseUrls: initialValues?.baseUrls?.length ? initialValues.baseUrls : [""],
        name: initialValues?.name,
        apiKeys: [{ label: "", apiKey: initialValues?.apiKey ?? "" }],
        notes: initialValues?.notes ?? "",
      });
    }
    return () => {
      cancelled = true;
    };
  }, [open, site, form, initialValues, getSiteApiKey, message, t]);

  const handleOk = async () => {
    try {
      const values = await form.validateFields();
      const baseUrls = normalizeBaseUrls(values.baseUrls as string[]);
      const capabilities = mergeCodexCapabilities(
        site?.capabilities ?? initialValues?.capabilities ?? {},
        capabilitiesFromCodexFlags(codexFlags),
      );
      const keys = normalizeApiKeyDrafts(values.apiKeys);
      if (keys.length === 0) {
        message.error(t("sites.apiKey"));
        return;
      }
      setSaving(true);
      let saved: Site;
      const isCreate = !site;
      if (site) {
        saved = await updateSite(site.id, {
          name: values.name,
          baseUrls,
          baseUrl: baseUrls[0],
          apiKeys: keys,
          protocol: values.protocol as SiteProtocol,
          notes: values.notes || null,
          capabilities,
        });
        invalidateSiteIconCache(site.id);
      } else {
        saved = await createSite({
          name: values.name,
          baseUrls,
          baseUrl: baseUrls[0],
          apiKey: keys[0]!.apiKey,
          apiKeyLabel: keys[0]!.label,
          extraApiKeys: keys.slice(1).map((row) => ({
            label: row.label,
            apiKey: row.apiKey,
          })),
          protocol: values.protocol,
          notes: values.notes || null,
          capabilities,
        });
      }
      message.success(isCreate ? t("sites.createSuccess") : t("sites.updateSuccess"));
      onSaved?.(saved, isCreate);
      onClose();
    } catch (e) {
      if (isAppError(e)) message.error(e.message);
    } finally {
      setSaving(false);
    }
  };

  return (
    <Modal
      open={open}
      title={site ? t("sites.edit") : t("sites.add")}
      onCancel={onClose}
      onOk={() => void handleOk()}
      confirmLoading={saving}
      okButtonProps={{ disabled: keyLoading }}
      okText={t("sites.save")}
      cancelText={t("sites.cancel")}
      width={560}
      destroyOnHidden
      centered
      mask={{ enabled: true, blur: true }}
      styles={{
        container: {
          maxHeight: "calc(100vh - 32px)",
          display: "flex",
          flexDirection: "column",
          overflow: "hidden",
        },
        body: {
          overflowY: "auto",
          overflowX: "hidden",
          minHeight: 0,
        },
      }}
    >
      <Form form={form} layout="vertical" className="mt-2" requiredMark="optional">
        <Form.Item name="name" label={t("sites.name")} rules={[{ required: true, message: t("sites.name") }]}>
          <Input placeholder="My Relay" allowClear />
        </Form.Item>
        <Form.Item
          label={
            <span className="inline-flex items-center gap-1.5">
              {t("sites.baseUrl")}
              <UrlWritePreviewIcon baseUrl={previewUrl} />
            </span>
          }
          extra={t("sites.baseUrlDefaultHint")}
          required
        >
          <BaseUrlListInput />
        </Form.Item>
        <Form.Item label={t("sites.apiKey")} extra={t("sites.apiKeyCreateHint")} required>
          <ApiKeyListInput disabled={keyLoading} />
        </Form.Item>
        <div className="flex flex-col gap-2">
          <Collapse
            size="small"
            activeKey={advancedOpen}
            onChange={(keys) => setAdvancedOpen(toActiveKeys(keys))}
            items={[
              {
                key: "advanced",
                label: t("sites.advanced"),
                children: (
                  <>
                    <Form.Item name="protocol" label={t("sites.protocol")}>
                      <Select
                        options={[
                          { value: "openai_compatible", label: t("sites.protocolOpenai") },
                          { value: "anthropic", label: t("sites.protocolAnthropic") },
                        ]}
                      />
                    </Form.Item>
                    <Form.Item name="notes" label={t("sites.notes")} className="!mb-0">
                      <Input.TextArea rows={2} allowClear />
                    </Form.Item>
                  </>
                ),
              },
            ]}
          />
          <Collapse
            size="small"
            activeKey={capOpen}
            onChange={(keys) => setCapOpen(toActiveKeys(keys))}
            items={[
              {
                key: "codex",
                label: t("sites.codexPrivateCapabilities"),
                children: (
                  <CodexCapabilitySwitchList value={codexFlags} onChange={setCodexFlags} />
                ),
              },
            ]}
          />
        </div>
      </Form>
    </Modal>
  );
}
