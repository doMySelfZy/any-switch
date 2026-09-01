import { memo, useEffect, useMemo, useRef, useState } from "react";
import { Alert, App, Select, Skeleton, Space, Switch } from "antd";
import { useTranslation } from "react-i18next";
import { SettingsGroup } from "@/components/settings/SettingsGroup";
import { useApplyStore, useSiteStore } from "@/stores";
import { ApplyFooter } from "./ApplyFooter";
import { SiteSelect } from "./SiteSelect";
import { TargetStatusCard, statusFor, toolFor } from "./TargetStatusCard";
import { buildModelOptions, hydratePiForm } from "./hydrateApplyForm";
import { showApplyException, showApplyOutcome } from "./showApplyOutcome";
import { useApplySiteSelection } from "./useApplySiteSelection";

const rowStyle: React.CSSProperties = { padding: "4px 0" };

export const PiApplyPanel = memo(function PiApplyPanel() {
  const { t } = useTranslation();
  const { message, modal } = App.useApp();
  const modelsBySite = useSiteStore((state) => state.modelsBySite);
  const modelsLoadingBySite = useSiteStore((state) => state.modelsLoadingBySite);
  const listModels = useSiteStore((state) => state.listModels);
  const updateSite = useSiteStore((state) => state.updateSite);
  const statuses = useApplyStore((state) => state.statuses);
  const tools = useApplyStore((state) => state.tools);
  const applying = useApplyStore((state) => state.applying);
  const loadStatus = useApplyStore((state) => state.loadStatus);
  const apply = useApplyStore((state) => state.apply);
  const revert = useApplyStore((state) => state.revert);
  const restoreOfficial = useApplyStore((state) => state.restoreOfficial);
  const cleanupOrphan = useApplyStore((state) => state.cleanupOrphan);
  const statusLoading = useApplyStore((state) => state.loading);

  const status = statusFor(statuses, "pi");
  const { siteId, site, sites, selectSite, hasAnySite, hasEnabledSite } = useApplySiteSelection(
    status?.appliedSiteId,
  );
  const [modelId, setModelId] = useState<string>();
  const [writeAllModels, setWriteAllModels] = useState(false);
  const models = siteId ? (modelsBySite[siteId] ?? []) : [];
  const modelsLoading = siteId ? Boolean(modelsLoadingBySite[siteId]) : false;

  useEffect(() => {
    if (siteId) void listModels(siteId, { force: true }).catch(() => null);
  }, [siteId, site?.activeApiKeyId, listModels]);

  const lastHydrate = useRef<{ siteId: string; apiKeyId: string | null; stamp: number | null } | null>(null);
  useEffect(() => {
    if (!site) {
      lastHydrate.current = null;
      setModelId(undefined);
      setWriteAllModels(false);
      return;
    }
    const stamp = status?.lastAppliedAt ?? null;
    const apiKeyId = site.activeApiKeyId ?? null;
    if (
      lastHydrate.current?.siteId === site.id &&
      lastHydrate.current.apiKeyId === apiKeyId &&
      lastHydrate.current.stamp === stamp
    )
      return;
    const defaults = hydratePiForm(site, status);
    setModelId(defaults.modelId);
    setWriteAllModels(defaults.writeAllModels);
    lastHydrate.current = { siteId: site.id, apiKeyId, stamp };
  }, [site, status]);

  const modelOptions = useMemo(() => buildModelOptions(models, [modelId]), [models, modelId]);
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
        targets: ["pi"],
        modelId,
        piWriteAllModels: writeAllModels,
      });
      showApplyOutcome(
        modal,
        t,
        result.results.find((item) => item.target === "pi"),
      );
    } catch (error) {
      showApplyException(modal, t, error);
    }
  };

  return (
    <div className="flex h-full min-h-0 flex-col">
      <div className="min-h-0 flex-1 overflow-y-auto p-6 pb-4">
        <SettingsGroup title={t("apply.status")}>
          {statusLoading && !status ? (
            <Skeleton active paragraph={{ rows: 3 }} title={{ width: "40%" }} />
          ) : (
            <TargetStatusCard
              status={status}
              tool={toolFor(tools, "pi")}
              onRefresh={() => loadStatus({ force: true })}
              onRevert={() => revert("pi")}
              onCleanupOrphan={() => cleanupOrphan("pi")}
            />
          )}
        </SettingsGroup>

        <SettingsGroup title={t("apply.groupSite")}>
          {!hasAnySite ? (
            <Alert type="info" title={t("apply.noSite")} showIcon />
          ) : (
            <Space orientation="vertical" className="w-full" size="middle">
              {!hasEnabledSite && <Alert type="info" title={t("apply.noEnabledSite")} showIcon />}
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
              </div>
            </Space>
          )}
        </SettingsGroup>

        {site && (
          <SettingsGroup title={t("apply.groupPiModels")}>
            <div style={rowStyle} className="flex items-center justify-between gap-4">
              <div>
                <div>{t("apply.piWriteAllModels")}</div>
                <div className="text-xs opacity-50">{t("apply.piWriteAllModelsHint")}</div>
              </div>
              <Switch checked={writeAllModels} onChange={setWriteAllModels} />
            </div>
            {writeAllModels && (
              <div className="mt-2 text-xs opacity-60">
                {t("apply.catalogModelCount", { count: models.length || (modelId ? 1 : 0) })}
              </div>
            )}
          </SettingsGroup>
        )}
      </div>

      <ApplyFooter
        target="pi"
        loading={applying}
        disabled={!modelId}
        onApply={() => void handleApply()}
        onRestoreOfficial={() => restoreOfficial("pi")}
      />
    </div>
  );
});
