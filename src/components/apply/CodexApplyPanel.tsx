import { memo, useEffect, useMemo, useRef, useState } from "react";
import { Alert, App, Select, Space, Switch, Divider, Skeleton } from "antd";
import { useTranslation } from "react-i18next";
import { SettingsGroup } from "@/components/settings/SettingsGroup";
import { useApplyStore, useSiteStore } from "@/stores";
import type { CodexCapabilitySource, CodexReasoningEffort } from "@/types/domain";
import {
  codexFlagsFromCapabilities,
  EMPTY_CODEX_FLAGS,
  summarizeCodexCapabilities,
  type CodexCapabilityFlags,
} from "@/lib/siteCapabilities";
import { ApplyFooter } from "./ApplyFooter";
import { SiteSelect } from "./SiteSelect";
import { TargetStatusCard, statusFor, toolFor } from "./TargetStatusCard";
import { useApplySiteSelection } from "./useApplySiteSelection";
import { buildModelOptions, hydrateCodexForm } from "./hydrateApplyForm";
import { showApplyException, showApplyOutcome } from "./showApplyOutcome";
import { CodexCapabilitySwitchList } from "./CodexCapabilitySwitchList";

const rowStyle: React.CSSProperties = { padding: "4px 0" };

export const CodexApplyPanel = memo(function CodexApplyPanel() {
  const { t } = useTranslation();
  const { message, modal } = App.useApp();
  const modelsBySite = useSiteStore((s) => s.modelsBySite);
  const modelsLoadingBySite = useSiteStore((s) => s.modelsLoadingBySite);
  const listModels = useSiteStore((s) => s.listModels);
  const updateSite = useSiteStore((s) => s.updateSite);

  const statuses = useApplyStore((s) => s.statuses);
  const tools = useApplyStore((s) => s.tools);
  const applying = useApplyStore((s) => s.applying);
  const loadStatus = useApplyStore((s) => s.loadStatus);
  const apply = useApplyStore((s) => s.apply);
  const revert = useApplyStore((s) => s.revert);
  const restoreOfficial = useApplyStore((s) => s.restoreOfficial);
  const cleanupOrphan = useApplyStore((s) => s.cleanupOrphan);
  const statusLoading = useApplyStore((s) => s.loading);

  const status = statusFor(statuses, "codex");
  const { siteId, site, sites, selectSite, hasAnySite, hasEnabledSite } = useApplySiteSelection(
    status?.appliedSiteId,
  );

  const [modelId, setModelId] = useState<string | undefined>();
  const [writeAllModels, setWriteAllModels] = useState(false);
  const [reasoning, setReasoning] = useState<CodexReasoningEffort | undefined>();
  const [capabilitySource, setCapabilitySource] = useState<CodexCapabilitySource>("site");
  const [codexFlags, setCodexFlags] = useState<CodexCapabilityFlags>(EMPTY_CODEX_FLAGS);

  const models = siteId ? (modelsBySite[siteId] ?? []) : [];
  const modelsLoading = siteId ? Boolean(modelsLoadingBySite[siteId]) : false;

  useEffect(() => {
    if (!siteId) return;
    void listModels(siteId, { force: true }).catch(() => null);
  }, [siteId, site?.activeApiKeyId, listModels]);

  const lastHydrate = useRef<{ siteId: string; apiKeyId: string | null; stamp: number | null } | null>(null);
  useEffect(() => {
    if (!site) {
      lastHydrate.current = null;
      setModelId(undefined);
      setCapabilitySource("site");
      setCodexFlags(EMPTY_CODEX_FLAGS);
      return;
    }
    const stamp = status?.lastAppliedAt ?? null;
    const apiKeyId = site.activeApiKeyId ?? null;
    const prev = lastHydrate.current;
    if (prev && prev.siteId === site.id && prev.apiKeyId === apiKeyId && prev.stamp === stamp) return;
    const defaults = hydrateCodexForm(site, status);
    setModelId(defaults.modelId);
    setWriteAllModels(defaults.writeAllModels);
    setReasoning(defaults.reasoning);
    setCapabilitySource(defaults.capabilitySource);
    setCodexFlags({
      compact: defaults.remoteCompaction,
      vision: defaults.imageUnderstanding,
      imagegen: defaults.imageGeneration,
      search: defaults.webSearch,
    });
    lastHydrate.current = { siteId: site.id, apiKeyId, stamp };
  }, [site, status]);

  const modelOptions = useMemo(
    () => buildModelOptions(models, [modelId]),
    [models, modelId],
  );

  const handleApply = async () => {
    if (!site) {
      message.warning(t("apply.noSite"));
      return;
    }
    if (!modelId) {
      message.warning(t("sites.selectModel"));
      return;
    }
    try {
      if (site.selectedModelId !== modelId) {
        await updateSite(site.id, { selectedModelId: modelId });
      }
      const result = await apply({
        siteId: site.id,
        apiKeyId: site.activeApiKeyId ?? undefined,
        targets: ["codex"],
        modelId,
        codexWriteAllModels: writeAllModels,
        codexReasoningEffort: reasoning ?? null,
        codexCapabilitySource: capabilitySource,
        codexRemoteCompaction: capabilitySource === "custom" ? codexFlags.compact : undefined,
        codexImageUnderstanding: capabilitySource === "custom" ? codexFlags.vision : undefined,
        codexImageGeneration: capabilitySource === "custom" ? codexFlags.imagegen : undefined,
        codexWebSearch: capabilitySource === "custom" ? codexFlags.search : undefined,
      });
      showApplyOutcome(
        modal,
        t,
        result.results.find((r) => r.target === "codex"),
      );
    } catch (e) {
      showApplyException(modal, t, e);
    }
  };

  const tool = toolFor(tools, "codex");

  return (
    <div className="flex h-full min-h-0 flex-col">
      <div className="min-h-0 flex-1 overflow-y-auto p-6 pb-4">
        <SettingsGroup title={t("apply.status")}>
          {statusLoading && !status ? (
            <Skeleton active paragraph={{ rows: 3 }} title={{ width: "40%" }} />
          ) : (
            <TargetStatusCard
              status={status}
              tool={tool}
              onRefresh={() => loadStatus({ force: true })}
              onRevert={() => revert("codex")}
              onCleanupOrphan={() => cleanupOrphan("codex")}
            />
          )}
        </SettingsGroup>

        <SettingsGroup title={t("apply.groupSite")}>
          {!hasAnySite ? (
            <Alert type="info" title={t("apply.noSite")} showIcon />
          ) : (
            <Space orientation="vertical" className="w-full" size="middle">
              {!hasEnabledSite && (
                <Alert type="info" title={t("apply.noEnabledSite")} showIcon />
              )}
              <div style={rowStyle}>
                <div className="mb-1 text-sm opacity-70">{t("apply.selectSite")}</div>
                <SiteSelect
                  sites={sites}
                  value={siteId ?? undefined}
                  placeholder={t("apply.selectSite")}
                  onChange={selectSite}
                />
              </div>
              <div style={rowStyle}>
                <div className="mb-1 text-sm opacity-70">{t("apply.defaultModel")}</div>
                {modelsLoading && models.length === 0 ? (
                  <Skeleton.Input active block style={{ height: 32 }} />
                ) : (
                  <Select
                    className="w-full"
                    value={modelId}
                    placeholder={t("sites.selectModel")}
                    options={modelOptions}
                    onChange={setModelId}
                    showSearch
                    optionFilterProp="label"
                    disabled={!site}
                    loading={modelsLoading}
                    notFoundContent={t("sites.noModels")}
                  />
                )}
                <div className="mt-1 text-xs opacity-50">{t("apply.codexModelHint")}</div>
              </div>
            </Space>
          )}
        </SettingsGroup>

        {site && (
          <>
            <SettingsGroup title={t("apply.groupCodexModels")}>
              <div style={rowStyle} className="flex items-center justify-between gap-4">
                <div>
                  <div>{t("apply.writeAllModels")}</div>
                  <div className="text-xs opacity-50">{t("apply.writeAllModelsHint")}</div>
                </div>
                <Switch checked={writeAllModels} onChange={setWriteAllModels} />
              </div>
              {writeAllModels && (
                <>
                  <Divider style={{ margin: "8px 0" }} />
                  <div className="text-xs opacity-60">
                    {modelsLoading && models.length === 0 ? (
                      <Skeleton.Input active size="small" style={{ width: 160 }} />
                    ) : (
                      t("apply.catalogModelCount", { count: models.length || (modelId ? 1 : 0) })
                    )}
                  </div>
                </>
              )}
            </SettingsGroup>

            <SettingsGroup title={t("apply.groupReasoning")}>
              <div style={rowStyle} className="flex items-center justify-between gap-4">
                <div>
                  <div>{t("apply.reasoningEffort")}</div>
                  <div className="text-xs opacity-50">{t("apply.reasoningHint")}</div>
                </div>
                <Select
                  style={{ minWidth: 160 }}
                  allowClear
                  value={reasoning}
                  onChange={(v) => setReasoning(v as CodexReasoningEffort | undefined)}
                  options={[
                    { value: "minimal", label: t("apply.effortMinimal") },
                    { value: "low", label: t("apply.effortLow") },
                    { value: "medium", label: t("apply.effortMedium") },
                    { value: "high", label: t("apply.effortHigh") },
                    { value: "xhigh", label: t("apply.effortXhigh") },
                  ]}
                />
              </div>
            </SettingsGroup>

            <SettingsGroup title={t("apply.groupCodexCapabilities")}>
              <div style={rowStyle}>
                <div className="mb-1 text-sm opacity-70">{t("apply.capabilitySource")}</div>
                <Select
                  className="w-full"
                  value={capabilitySource}
                  onChange={(value) => {
                    const next = value as CodexCapabilitySource;
                    setCapabilitySource(next);
                    if (next === "custom") {
                      setCodexFlags(codexFlagsFromCapabilities(site.capabilities));
                    }
                  }}
                  options={[
                    { value: "site", label: t("apply.capabilityFollowSite") },
                    { value: "custom", label: t("apply.capabilityCustom") },
                  ]}
                />
                <div className="mt-1 text-xs opacity-50">
                  {capabilitySource === "site"
                    ? t("apply.capabilityFollowHint")
                    : t("apply.capabilityCustomHint")}
                </div>
              </div>
              {capabilitySource === "site" ? (
                <div className="mt-3 text-xs opacity-50">
                  {summarizeCodexCapabilities(codexFlagsFromCapabilities(site.capabilities))
                    .map(({ key, on }) => {
                      const title =
                        key === "codex-compact"
                          ? t("apply.remoteCompaction")
                          : key === "codex-vision"
                            ? t("apply.imageUnderstanding")
                            : key === "codex-imagegen"
                              ? t("apply.imageGeneration")
                              : t("apply.webSearch");
                      return `${title}${on ? t("apply.capabilityOn") : t("apply.capabilityOff")}`;
                    })
                    .join(" · ")}
                </div>
              ) : (
                <>
                  <Divider style={{ margin: "12px 0 8px" }} />
                  <CodexCapabilitySwitchList value={codexFlags} onChange={setCodexFlags} />
                </>
              )}
            </SettingsGroup>
          </>
        )}
      </div>

      <ApplyFooter
        target="codex"
        loading={applying}
        disabled={!modelId}
        onApply={() => void handleApply()}
        onRestoreOfficial={() => restoreOfficial("codex")}
      />
    </div>
  );
});
