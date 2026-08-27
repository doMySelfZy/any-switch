import { memo, useEffect, useMemo, useRef, useState } from "react";
import { Alert, App, Divider, Select, Space, Skeleton, Switch } from "antd";
import { useTranslation } from "react-i18next";
import { SettingsGroup } from "@/components/settings/SettingsGroup";
import { useApplyStore, useSiteStore } from "@/stores";
import type { ClaudeAuthKeyStyle, ClaudeEffortLevel } from "@/types/domain";
import { ApplyFooter } from "./ApplyFooter";
import { SiteSelect } from "./SiteSelect";
import { TargetStatusCard, statusFor, toolFor } from "./TargetStatusCard";
import { useApplySiteSelection } from "./useApplySiteSelection";
import { buildModelOptions, hydrateClaudeForm } from "./hydrateApplyForm";
import { showApplyException, showApplyOutcome } from "./showApplyOutcome";

const rowStyle: React.CSSProperties = { padding: "4px 0" };

export const ClaudeApplyPanel = memo(function ClaudeApplyPanel() {
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

  const status = statusFor(statuses, "claude_code");
  const { siteId, site, sites, selectSite, hasAnySite, hasEnabledSite } = useApplySiteSelection(
    status?.appliedSiteId,
  );

  const [modelId, setModelId] = useState<string | undefined>();
  const [claudeAuth, setClaudeAuth] = useState<ClaudeAuthKeyStyle>("anthropic_auth_token");
  const [opusModel, setOpusModel] = useState<string | undefined>();
  const [sonnetModel, setSonnetModel] = useState<string | undefined>();
  const [haikuModel, setHaikuModel] = useState<string | undefined>();
  const [effort, setEffort] = useState<ClaudeEffortLevel | undefined>();
  const [use1mContext, setUse1mContext] = useState(false);

  const models = siteId ? (modelsBySite[siteId] ?? []) : [];
  const modelsLoading = siteId ? Boolean(modelsLoadingBySite[siteId]) : false;

  useEffect(() => {
    if (!siteId) return;
    void listModels(siteId).catch(() => null);
  }, [siteId, listModels]);

  const lastHydrate = useRef<{ siteId: string; stamp: number | null } | null>(null);
  useEffect(() => {
    if (!site) {
      lastHydrate.current = null;
      setModelId(undefined);
      setOpusModel(undefined);
      setSonnetModel(undefined);
      setHaikuModel(undefined);
      setUse1mContext(false);
      return;
    }
    const stamp = status?.lastAppliedAt ?? null;
    const prev = lastHydrate.current;
    if (prev && prev.siteId === site.id && prev.stamp === stamp) return;
    const defaults = hydrateClaudeForm(site, status);
    setModelId(defaults.modelId);
    setOpusModel(defaults.opusModel);
    setSonnetModel(defaults.sonnetModel);
    setHaikuModel(defaults.haikuModel);
    setEffort(defaults.effort);
    setClaudeAuth(defaults.auth);
    setUse1mContext(defaults.use1mContext);
    lastHydrate.current = { siteId: site.id, stamp };
  }, [site, status]);

  const modelOptions = useMemo(
    () => buildModelOptions(models, [modelId, opusModel, sonnetModel, haikuModel]),
    [models, modelId, opusModel, sonnetModel, haikuModel],
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
      if (site.claudeAuthKeyStyle !== claudeAuth) {
        await updateSite(site.id, { claudeAuthKeyStyle: claudeAuth });
      }
      if (site.selectedModelId !== modelId) {
        await updateSite(site.id, { selectedModelId: modelId });
      }
      const result = await apply({
        siteId: site.id,
        targets: ["claude_code"],
        modelId,
        claudeAuthKeyStyle: claudeAuth,
        claudeOpusModelId: opusModel ?? null,
        claudeSonnetModelId: sonnetModel ?? null,
        claudeHaikuModelId: haikuModel ?? null,
        claudeEffortLevel: effort ?? null,
        claudeUse1mContext: use1mContext,
      });
      showApplyOutcome(
        modal,
        t,
        result.results.find((r) => r.target === "claude_code"),
      );
    } catch (e) {
      showApplyException(modal, t, e);
    }
  };

  const tool = toolFor(tools, "claude_code");

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
              onRevert={() => revert("claude_code")}
              onCleanupOrphan={() => cleanupOrphan("claude_code")}
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
                <div className="mt-1 text-xs opacity-50">{t("apply.defaultModelHint")}</div>
              </div>
            </Space>
          )}
        </SettingsGroup>

        {site && (
          <>
            <SettingsGroup title={t("apply.groupClaudeAuth")}>
              <div style={rowStyle}>
                <div className="mb-1 text-sm opacity-70">{t("sites.claudeAuth")}</div>
                <Select
                  className="w-full"
                  value={claudeAuth}
                  onChange={(v) => setClaudeAuth(v as ClaudeAuthKeyStyle)}
                  options={[
                    { value: "anthropic_auth_token", label: t("sites.authToken") },
                    { value: "anthropic_api_key", label: t("sites.authApiKey") },
                  ]}
                />
                <div className="mt-1 text-xs opacity-50">{t("sites.claudeAuthHint")}</div>
              </div>
            </SettingsGroup>

            <SettingsGroup title={t("apply.groupModelMap")}>
              <div className="mb-3 text-xs opacity-50">{t("apply.modelMapHint")}</div>
              {(
                [
                  { key: "opus", label: t("apply.aliasOpus"), value: opusModel, set: setOpusModel },
                  { key: "sonnet", label: t("apply.aliasSonnet"), value: sonnetModel, set: setSonnetModel },
                  { key: "haiku", label: t("apply.aliasHaiku"), value: haikuModel, set: setHaikuModel },
                ] as const
              ).map((row, idx) => (
                <div key={row.key}>
                  {idx > 0 && <Divider style={{ margin: "8px 0" }} />}
                  <div style={rowStyle} className="flex items-center justify-between gap-4">
                    <div className="min-w-0 flex-1">
                      <div className="text-sm">{row.label}</div>
                      <div className="font-mono text-xs opacity-50">
                        ANTHROPIC_DEFAULT_{row.key.toUpperCase()}_MODEL
                      </div>
                    </div>
                    <Select
                      style={{ minWidth: 220 }}
                      allowClear
                      showSearch
                      optionFilterProp="label"
                      placeholder={t("apply.aliasSameAsDefault")}
                      value={row.value}
                      options={modelOptions}
                      onChange={row.set}
                      loading={modelsLoading}
                    />
                  </div>
                </div>
              ))}
            </SettingsGroup>

            <SettingsGroup title={t("apply.groupContextWindow")}>
              <div style={rowStyle} className="flex items-center justify-between gap-4">
                <div>
                  <div>{t("apply.context1m")}</div>
                  <div className="text-xs opacity-50">{t("apply.context1mHint")}</div>
                </div>
                <Switch
                  aria-label={t("apply.context1m")}
                  checked={use1mContext}
                  onChange={setUse1mContext}
                />
              </div>
            </SettingsGroup>

            <SettingsGroup title={t("apply.groupEffort")}>
              <div style={rowStyle} className="flex items-center justify-between gap-4">
                <div>
                  <div>{t("apply.effortLevel")}</div>
                  <div className="text-xs opacity-50">{t("apply.effortHint")}</div>
                </div>
                <Select
                  style={{ minWidth: 160 }}
                  allowClear
                  value={effort}
                  onChange={(v) => setEffort(v as ClaudeEffortLevel | undefined)}
                  options={[
                    { value: "low", label: t("apply.effortLow") },
                    { value: "medium", label: t("apply.effortMedium") },
                    { value: "high", label: t("apply.effortHigh") },
                    { value: "max", label: t("apply.effortMax") },
                  ]}
                />
              </div>
            </SettingsGroup>
          </>
        )}
      </div>

      <ApplyFooter
        target="claude_code"
        loading={applying}
        disabled={!modelId}
        onApply={() => void handleApply()}
        onRestoreOfficial={() => restoreOfficial("claude_code")}
      />
    </div>
  );
});
