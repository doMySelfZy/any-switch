import { useCallback, useEffect, useState } from "react";
import { App, Button, Checkbox, Empty, Skeleton, Switch, Tooltip, theme } from "antd";
import { Plus, Trash2, Pencil, RefreshCw } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useApplyStore, useSiteStore, useUIStore } from "@/stores";
import { SiteFormModal, type SiteFormInitialValues } from "@/components/sites/SiteFormModal";
import { ManualModelModal } from "@/components/sites/ManualModelModal";
import { GoApplyButton } from "@/components/sites/GoApplyButton";
import { ModelPicker } from "@/components/sites/ModelPicker";
import { SiteAvatar } from "@/components/sites/SiteAvatar";
import { SiteListItem } from "@/components/sites/SiteListItem";
import { SiteDetailSkeleton } from "@/components/sites/SiteDetailSkeleton";
import { EmptyOnboarding } from "@/components/sites/EmptyOnboarding";
import { SiteRouteSwitcher } from "@/components/sites/SiteRouteSwitcher";
import { SiteQuotaRow } from "@/components/sites/SiteQuotaRow";
import type { Site } from "@/types/domain";
import { isAppError } from "@/lib/invoke";
import { quotaCacheKey } from "@/lib/quotaProbe";
import { useDeferredReady } from "@/hooks/useDeferredReady";
import { targetsAppliedForSite } from "@/components/apply/TargetStatusCard";

function protocolLabelKey(protocol: Site["protocol"]): string {
  return protocol === "anthropic" ? "sites.protocolAnthropic" : "sites.protocolOpenai";
}

export function SitesPage() {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const { modal, message } = App.useApp();
  const sites = useSiteStore((s) => s.sites);
  const modelsBySite = useSiteStore((s) => s.modelsBySite);
  const loading = useSiteStore((s) => s.loading);
  const hydrated = useSiteStore((s) => s.hydrated);
  const fetchingModels = useSiteStore((s) => s.fetchingModels);
  const loadSites = useSiteStore((s) => s.loadSites);
  const listModels = useSiteStore((s) => s.listModels);
  const fetchModels = useSiteStore((s) => s.fetchModels);
  const probeQuota = useSiteStore((s) => s.probeQuota);
  const quotaBySite = useSiteStore((s) => s.quotaBySite);
  const quotaCacheKeyBySite = useSiteStore((s) => s.quotaCacheKeyBySite);
  const quotaLoadingBySite = useSiteStore((s) => s.quotaLoadingBySite);
  const setSelectedModel = useSiteStore((s) => s.setSelectedModel);
  const deleteSite = useSiteStore((s) => s.deleteSite);
  const updateSite = useSiteStore((s) => s.updateSite);
  const loadStatus = useApplyStore((s) => s.loadStatus);
  const revert = useApplyStore((s) => s.revert);
  const selectedSiteId = useUIStore((s) => s.selectedSiteId);
  const setSelectedSiteId = useUIStore((s) => s.setSelectedSiteId);
  const setPage = useUIStore((s) => s.setPage);
  const setApplyTab = useUIStore((s) => s.setApplyTab);
  const setApplyPrefillSiteId = useUIStore((s) => s.setApplyPrefillSiteId);

  const [formOpen, setFormOpen] = useState(false);
  const [editing, setEditing] = useState<Site | null>(null);
  const [formInitial, setFormInitial] = useState<SiteFormInitialValues | null>(null);
  const [manualOpen, setManualOpen] = useState(false);
  const pendingSiteForm = useUIStore((s) => s.pendingSiteForm);
  const setPendingSiteForm = useUIStore((s) => s.setPendingSiteForm);

  useEffect(() => {
    // Soft when already hydrated so revisiting sites page doesn't flash loading.
    void loadSites({ soft: useSiteStore.getState().hydrated });
  }, [loadSites]);

  useEffect(() => {
    if (!pendingSiteForm) return;
    setEditing(null);
    setFormInitial({
      name: pendingSiteForm.name,
      baseUrls: pendingSiteForm.baseUrls,
      apiKey: pendingSiteForm.apiKey,
      protocol: pendingSiteForm.protocol,
      notes: pendingSiteForm.notes,
      capabilities: pendingSiteForm.hasCapabilityParams ? pendingSiteForm.capabilities : undefined,
    });
    setFormOpen(true);
    setPendingSiteForm(null);
  }, [pendingSiteForm, setPendingSiteForm]);

  const openCreateForm = () => {
    setEditing(null);
    setFormInitial(null);
    setFormOpen(true);
  };

  const selected = sites.find((s) => s.id === selectedSiteId) ?? sites[0] ?? null;

  useEffect(() => {
    if (selected && selected.id !== selectedSiteId) {
      setSelectedSiteId(selected.id);
    }
  }, [selected, selectedSiteId, setSelectedSiteId]);

  useEffect(() => {
    setManualOpen(false);
  }, [selected?.id]);

  // Defer heavy detail mount one frame after site switch so sidebar highlight paints first.
  const detailReady = useDeferredReady(selected?.id ?? null);

  useEffect(() => {
    if (!selectedSiteId) return;
    void listModels(selectedSiteId).then((models) => {
      const site = useSiteStore.getState().sites.find((s) => s.id === selectedSiteId);
      if (site && !site.selectedModelId && models[0]) {
        void setSelectedModel(selectedSiteId, models[0].modelId);
      }
    });
  }, [selectedSiteId, listModels, setSelectedModel]);

  useEffect(() => {
    if (!selected?.id || !selected.hasKey) return;
    void probeQuota(selected.id).catch(() => undefined);
  }, [selected?.id, selected?.baseUrl, selected?.keyPrefix, selected?.hasKey, probeQuota]);

  const handleFetchModels = useCallback(
    async (site: Site) => {
      try {
        const result = await fetchModels(site.id);
        if (!site.selectedModelId && result.models[0]) {
          await setSelectedModel(site.id, result.models[0].modelId);
        }
        message.success(t("sites.fetchModelsSuccess", { count: result.models.length }));
      } catch (e) {
        message.error(isAppError(e) ? e.message : String(e));
      }
    },
    [fetchModels, setSelectedModel, message, t],
  );

  const handleRefreshQuota = useCallback(async () => {
    if (!selected) return;
    try {
      const result = await probeQuota(selected.id, { force: true });
      if (result.status !== "available") {
        message.warning(t("sites.quotaRefreshFailed"));
      }
    } catch (e) {
      message.warning(isAppError(e) ? e.message : t("sites.quotaRefreshFailed"));
    }
  }, [selected, probeQuota, message, t]);

  const handleSiteSaved = useCallback(
    (site: Site, isCreate: boolean) => {
      setSelectedSiteId(site.id);
      if (isCreate) {
        void handleFetchModels(site);
      }
    },
    [setSelectedSiteId, handleFetchModels],
  );

  const disableSite = async (site: Site, clear: boolean) => {
    try {
      if (clear) {
        const targets = targetsAppliedForSite(useApplyStore.getState().statuses, site.id);
        for (const kind of targets) {
          await revert(kind);
        }
      }
      await updateSite(site.id, { enabled: false });
      message.success(clear ? t("sites.disableClearedSuccess") : t("sites.disableSuccess"));
    } catch (e) {
      message.error(isAppError(e) ? e.message : String(e));
    }
  };

  const handleEnabledChange = async (site: Site, enabled: boolean) => {
    if (enabled) {
      try {
        await updateSite(site.id, { enabled: true });
      } catch (e) {
        message.error(isAppError(e) ? e.message : String(e));
      }
      return;
    }

    try {
      await loadStatus({ force: true });
    } catch (e) {
      message.error(isAppError(e) ? e.message : String(e));
      return;
    }

    const targets = targetsAppliedForSite(useApplyStore.getState().statuses, site.id);
    if (targets.length === 0) {
      await disableSite(site, false);
      return;
    }

    const targetLabels = targets
      .map((kind) =>
        kind === "claude_code"
          ? t("apply.targetClaude")
          : kind === "codex"
            ? t("apply.targetCodex")
            : t("apply.targetPi"),
      )
      .join(t("common.listSep"));

    const dlg = modal.confirm({
      centered: true,
      title: t("sites.disableAppliedTitle"),
      content: t("sites.disableAppliedHint", { targets: targetLabels }),
      footer: (
        <div className="flex justify-end gap-2">
          <Button onClick={() => dlg.destroy()}>{t("common.cancel")}</Button>
          <Button
            onClick={() => {
              dlg.destroy();
              void disableSite(site, false);
            }}
          >
            {t("sites.disableSkip")}
          </Button>
          <Button
            type="primary"
            danger
            onClick={() => {
              dlg.destroy();
              void disableSite(site, true);
            }}
          >
            {t("sites.disableClear")}
          </Button>
        </div>
      ),
    });
  };

  const handleDelete = (site: Site) => {
    let cleanup = false;
    modal.confirm({
      title: t("sites.deleteConfirm", { name: site.name }),
      centered: true,
      content: (
        <div className="mt-2">
          <Checkbox
            onChange={(e) => {
              cleanup = e.target.checked;
            }}
          >
            <div>
              <div>{t("sites.cleanupTargets")}</div>
              <div className="text-xs opacity-60" style={{ whiteSpace: "normal" }}>
                {t("sites.cleanupTargetsHint")}
              </div>
            </div>
          </Checkbox>
        </div>
      ),
      okText: t("common.confirm"),
      cancelText: t("common.cancel"),
      okButtonProps: { danger: true },
      onOk: async () => {
        try {
          await deleteSite(site.id, cleanup);
          if (selectedSiteId === site.id) setSelectedSiteId(null);
          message.success(t("sites.deleteSuccess"));
        } catch (e) {
          message.error(isAppError(e) ? e.message : String(e));
        }
      },
    });
  };

  // First load with no cache: lightweight skeleton list instead of blank freeze.
  if (loading && !hydrated) {
    return (
      <div className="flex h-full min-h-0">
        <div
          className="flex w-64 shrink-0 flex-col border-r p-3"
          style={{ borderColor: token.colorBorderSecondary }}
        >
          <Skeleton active paragraph={{ rows: 6 }} title={{ width: "50%" }} />
        </div>
        <div className="min-w-0 flex-1 p-4">
          <SiteDetailSkeleton />
        </div>
      </div>
    );
  }

  if (!loading && sites.length === 0) {
    return (
      <>
        <EmptyOnboarding onAdd={openCreateForm} />
        <SiteFormModal
          open={formOpen}
          site={editing}
          initialValues={editing ? null : formInitial}
          onClose={() => {
            setFormOpen(false);
            setFormInitial(null);
          }}
          onSaved={handleSiteSaved}
        />
      </>
    );
  }

  const models = selected ? (modelsBySite[selected.id] ?? []) : [];
  const modelsCached = selected
    ? Object.prototype.hasOwnProperty.call(modelsBySite, selected.id)
    : false;
  // Sidebar paints first; detail waits one frame + model list cache for the selected site.
  const showDetailSkeleton = Boolean(selected) && (!detailReady || !modelsCached);

  return (
    <div className="flex h-full min-h-0">
      <div
        className="flex w-64 shrink-0 flex-col border-r"
        style={{ borderColor: token.colorBorderSecondary }}
      >
        <div className="flex items-center justify-between p-3">
          <span className="font-medium">{t("sites.title")}</span>
          <Button
            color="default"
            size="small"
            icon={<Plus size={14} />}
            onClick={openCreateForm}
          >
            {t("sites.add")}
          </Button>
        </div>
        <div className="scroll-y flex flex-1 flex-col gap-2 px-2 pb-2">
          {sites.map((site) => (
            <SiteListItem
              key={site.id}
              site={site}
              active={selected?.id === site.id}
              onSelect={() => setSelectedSiteId(site.id)}
              onEdit={() => {
                setEditing(site);
                setFormOpen(true);
              }}
              onDelete={() => handleDelete(site)}
            />
          ))}
        </div>
      </div>

      <div className="flex h-full min-h-0 min-w-0 flex-1 flex-col overflow-hidden p-4">
        {selected ? (
          showDetailSkeleton ? (
            <SiteDetailSkeleton />
          ) : (
            <div className="flex h-full min-h-0 flex-1 flex-col">
              <div className="mb-4 flex shrink-0 items-start justify-between gap-3">
                <div className="flex min-w-0 flex-1 items-center gap-2.5">
                  <SiteAvatar
                    siteId={selected.id}
                    name={selected.name}
                    baseUrl={selected.baseUrl}
                    size={32}
                  />
                  <span className="min-w-0 truncate text-base font-medium">{selected.name}</span>
                  <div className="shrink-0">
                    <GoApplyButton
                      disabled={!selected.selectedModelId || !selected.enabled}
                      onApply={(tab) => {
                        setSelectedSiteId(selected.id);
                        setApplyPrefillSiteId(selected.id);
                        setApplyTab(tab);
                        setPage("apply");
                      }}
                    />
                  </div>
                </div>
                <div className="flex shrink-0 items-center gap-1">
                  <Switch
                    size="small"
                    checked={selected.enabled}
                    checkedChildren={t("sites.enabled")}
                    unCheckedChildren={t("sites.disabled")}
                    onChange={(v) => void handleEnabledChange(selected, v)}
                  />
                  <Tooltip title={t("sites.fetchModels")}>
                    <Button
                      type="text"
                      size="small"
                      loading={fetchingModels}
                      icon={<RefreshCw size={14} />}
                      onClick={() => void handleFetchModels(selected)}
                      aria-label={t("sites.fetchModels")}
                    />
                  </Tooltip>
                  <Tooltip title={t("sites.edit")}>
                    <Button
                      type="text"
                      size="small"
                      icon={<Pencil size={14} />}
                      onClick={() => {
                        setEditing(selected);
                        setFormOpen(true);
                      }}
                      aria-label={t("sites.edit")}
                    />
                  </Tooltip>
                  <Tooltip title={t("sites.delete")}>
                    <Button
                      type="text"
                      size="small"
                      danger
                      icon={<Trash2 size={14} />}
                      onClick={() => handleDelete(selected)}
                      aria-label={t("sites.delete")}
                    />
                  </Tooltip>
                </div>
              </div>

              <div className="mb-4 shrink-0 space-y-2 text-sm">
                <div className="flex gap-2">
                  <span className="w-28 shrink-0 opacity-50">{t("sites.baseUrl")}</span>
                  <div className="min-w-0 flex-1">
                    <SiteRouteSwitcher site={selected} />
                  </div>
                </div>
                <div className="flex gap-2">
                  <span className="w-28 shrink-0 opacity-50">{t("sites.keyPrefix")}</span>
                  <span>{selected.keyPrefix || "—"}</span>
                </div>
                <SiteQuotaRow
                  quota={
                    quotaCacheKeyBySite[selected.id] === quotaCacheKey(selected)
                      ? (quotaBySite[selected.id] ?? null)
                      : null
                  }
                  loading={Boolean(quotaLoadingBySite[selected.id])}
                  refreshing={
                    Boolean(quotaLoadingBySite[selected.id]) &&
                    quotaBySite[selected.id]?.status === "available"
                  }
                  onRefresh={() => void handleRefreshQuota()}
                />
                <div className="flex gap-2">
                  <span className="w-28 shrink-0 opacity-50">{t("sites.protocol")}</span>
                  <span>{t(protocolLabelKey(selected.protocol))}</span>
                </div>
                {selected.notes && (
                  <div className="flex gap-2">
                    <span className="w-28 shrink-0 opacity-50">{t("sites.notes")}</span>
                    <span className="min-w-0 break-words">{selected.notes}</span>
                  </div>
                )}
              </div>

              <ModelPicker
                site={selected}
                models={models}
                onAddManual={() => setManualOpen(true)}
                onFetch={() => handleFetchModels(selected)}
              />
            </div>
          )
        ) : (
          <Empty description={t("sites.emptyTitle")} />
        )}
      </div>

      <SiteFormModal
        open={formOpen}
        site={editing}
        initialValues={editing ? null : formInitial}
        onClose={() => {
          setFormOpen(false);
          setFormInitial(null);
        }}
        onSaved={handleSiteSaved}
      />
      <ManualModelModal open={manualOpen} site={selected} onClose={() => setManualOpen(false)} />
    </div>
  );
}
