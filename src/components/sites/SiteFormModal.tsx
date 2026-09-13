import { useEffect, useState } from "react";
import { App, Button, Collapse, Form, Input, Modal, Select, Typography } from "antd";
import { useTranslation } from "react-i18next";
import type { NewApiAccessProbe, Site, SiteCapabilities, SiteProtocol } from "@/types/domain";
import { invoke, isAppError } from "@/lib/invoke";
import { useSiteStore } from "@/stores";
import { UrlWritePreviewIcon } from "./UrlWritePreview";
import { ApiKeyListInput, loadSiteKeyDrafts, normalizeApiKeyDrafts } from "./ApiKeyListInput";
import { BaseUrlListInput } from "./BaseUrlListInput";
import { invalidateSiteIconCache } from "@/lib/siteIcon";
import { siteApiKeys } from "@/lib/siteApiKey";
import { normalizeBaseUrls, siteBaseUrls } from "@/lib/urlNormalize";
import { formatQuotaAmountLocalized } from "@/lib/quotaProbe";
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

const { Text } = Typography;

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
  /** 打开时强制展开高级配置（如从额度提示跳入）。 */
  forceAdvancedOpen?: boolean;
  onClose: () => void;
  onSaved?: (site: Site, isCreate: boolean) => void | Promise<void>;
}

export function SiteFormModal({ open, site, initialValues, forceAdvancedOpen, onClose, onSaved }: Props) {
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
  const [newapiTokenLoadFailed, setNewapiTokenLoadFailed] = useState(false);
  const [newapiTesting, setNewapiTesting] = useState(false);
  const [newapiTestResult, setNewapiTestResult] = useState<{
    ok: boolean;
    amount?: string;
    detail?: string;
  } | null>(null);
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
      setNewapiTokenLoadFailed(false);
      setNewapiTestResult(null);
      setAdvancedOpen(
        shouldOpenAdvanced(protocol, notes) || site.newapiConfigured || forceAdvancedOpen
          ? ["advanced"]
          : [],
      );
      form.setFieldsValue({
        name: site.name,
        baseUrls: siteBaseUrls(site),
        protocol: site.protocol,
        notes: site.notes ?? "",
        newapiAccessToken: "",
        newapiUserId: site.newapiUserId ?? "",
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
      if (site.newapiConfigured) {
        void invoke<string>("get_site_newapi_token", { id: site.id })
          .then((token) => {
            if (!cancelled) form.setFieldValue("newapiAccessToken", token);
          })
          .catch((error: unknown) => {
            if (cancelled) {
              return;
            }
            setNewapiTokenLoadFailed(true);
            message.error(isAppError(error) ? error.message : t("sites.newapiTokenLoadFailed"));
          });
      }
    } else {
      setKeyLoading(false);
      setNewapiTokenLoadFailed(false);
      setAdvancedOpen(
        forceAdvancedOpen ||
        shouldOpenAdvanced(
          initialValues?.protocol ?? "openai_compatible",
          initialValues?.notes ?? "",
        )
          ? ["advanced"]
          : [],
      );
      form.resetFields();
      form.setFieldsValue({
        protocol: initialValues?.protocol ?? "openai_compatible",
        baseUrls: initialValues?.baseUrls?.length ? initialValues.baseUrls : [""],
        name: initialValues?.name,
        apiKeys: [{ label: "", apiKey: initialValues?.apiKey ?? "" }],
        notes: initialValues?.notes ?? "",
        newapiAccessToken: "",
        newapiUserId: "",
      });
    }
    return () => {
      cancelled = true;
    };
  }, [open, site, form, initialValues, forceAdvancedOpen, getSiteApiKey, message, t]);

  const handleTestNewapi = async () => {
    const values = form.getFieldsValue(["newapiAccessToken", "newapiUserId", "baseUrls"]);
    const baseUrls = normalizeBaseUrls((values.baseUrls as string[] | undefined) ?? []);
    const accessToken = ((values.newapiAccessToken as string | undefined) ?? "").trim();
    const userId = ((values.newapiUserId as string | undefined) ?? "").trim();
    if (!baseUrls[0]) {
      message.error(t("sites.newapiTestMissingBaseUrl"));
      return;
    }
    if (!userId) {
      message.error(t("sites.newapiTestMissingCredentials"));
      return;
    }
    if (!accessToken && !(site?.newapiConfigured && !newapiTokenLoadFailed)) {
      message.error(t("sites.newapiTestMissingCredentials"));
      return;
    }
    setNewapiTesting(true);
    setNewapiTestResult(null);
    try {
      const probe = await invoke<NewApiAccessProbe>("test_newapi_access", {
        input: {
          baseUrl: baseUrls[0],
          accessToken: accessToken || null,
          userId,
          siteId: site?.id ?? null,
        },
      });
      if (probe.ok) {
        setNewapiTestResult({
          ok: true,
          amount:
            probe.remainingUsd != null
              ? formatQuotaAmountLocalized(probe.remainingUsd, probe.unit, t)
              : "-",
        });
      } else {
        setNewapiTestResult({
          ok: false,
          detail: `HTTP ${probe.status}${probe.message ? ` · ${probe.message}` : ""}`,
        });
      }
    } catch (error) {
      setNewapiTestResult({
        ok: false,
        detail: isAppError(error) ? error.message : String(error),
      });
    } finally {
      setNewapiTesting(false);
    }
  };

  const handleOk = async () => {    try {
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
      // 已配置令牌但解密回填失败时省略字段，避免把令牌意外清空。
      const omitNewapiToken = site?.newapiConfigured === true && newapiTokenLoadFailed;
      const newapiAccessToken = omitNewapiToken
        ? undefined
        : (values.newapiAccessToken?.trim() || "");
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
          newapiAccessToken,
          newapiUserId: values.newapiUserId?.trim() || "",
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
          newapiAccessToken: values.newapiAccessToken?.trim() || null,
          newapiUserId: values.newapiUserId?.trim() || null,
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
                    <Form.Item name="notes" label={t("sites.notes")}>
                      <Input.TextArea rows={2} allowClear />
                    </Form.Item>
                    <Form.Item
                      name="newapiAccessToken"
                      label={t("sites.newapiAccessToken")}
                      extra={
                        site?.newapiConfigured && !newapiTokenLoadFailed
                          ? t("sites.newapiTokenSavedHint")
                          : t("sites.newapiTokenHint")
                      }
                    >
                      <Input.Password autoComplete="new-password" placeholder="Access Token" />
                    </Form.Item>
                    <Form.Item
                      name="newapiUserId"
                      label={t("sites.newapiUserId")}
                      className="!mb-0"
                      extra={t("sites.newapiUserIdHint")}
                    >
                      <Input allowClear placeholder="1" inputMode="numeric" />
                    </Form.Item>
                    <div className="mt-3 flex items-center gap-3">
                      <Button
                        size="small"
                        loading={newapiTesting}
                        onClick={() => void handleTestNewapi()}
                      >
                        {t("sites.newapiTest")}
                      </Button>
                      {newapiTestResult && (
                        <Text
                          type={newapiTestResult.ok ? "success" : "danger"}
                          style={{ fontSize: 12 }}
                        >
                          {newapiTestResult.ok
                            ? t("sites.newapiTestOk", { amount: newapiTestResult.amount })
                            : t("sites.newapiTestFailed", {
                                detail: newapiTestResult.detail,
                              })}
                        </Text>
                      )}
                    </div>
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
