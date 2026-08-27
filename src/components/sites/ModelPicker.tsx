import { useEffect, useMemo, useState } from "react";
import { App, Button, Checkbox, Input, theme, Tooltip } from "antd";
import { CheckSquare, FlaskConical, Plus, RefreshCw, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { Site, SiteModel } from "@/types/domain";
import { useSiteStore } from "@/stores";
import { isAppError } from "@/lib/invoke";
import { groupModelsByPrefix } from "@/lib/modelPrefix";
import { ModelCountBadge, ModelTag } from "@/components/sites/ModelTag";
import { TestModelsModal } from "@/components/sites/TestModelsModal";

interface Props {
  site: Site;
  models: SiteModel[];
  /** Hide the fetch button when parent already places it in the header. */
  showFetchButton?: boolean;
  onAddManual?: () => void;
  onFetch?: () => void | Promise<void>;
}

interface ModelFetchErrorPresentation {
  messageKey: string;
  details?: string;
}

function modelFetchErrorPresentation(error: string): ModelFetchErrorPresentation {
  if (error === "unauthorized") {
    return { messageKey: "sites.modelListUnauthorized" };
  }
  if (error === "timeout" || error === "request timed out") {
    return { messageKey: "errors.timeout" };
  }
  const knownCode = ["network", "not_found", "invalid_response", "ssl"].find(
    (code) => code === error,
  );
  if (knownCode) {
    return { messageKey: `errors.${knownCode}` };
  }
  const httpStatus = error.match(/\bHTTP\s+([45]\d\d)\b/i)?.[1];
  if (httpStatus) {
    return {
      messageKey: "sites.modelListFetchTechnicalDetails",
      details: `HTTP ${httpStatus}`,
    };
  }
  return { messageKey: "sites.modelListFetchRetry" };
}

export function ModelPicker({
  site,
  models: modelsProp,
  showFetchButton = false,
  onAddManual,
  onFetch,
}: Props) {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const { message, modal } = App.useApp();
  const fetchModels = useSiteStore((s) => s.fetchModels);
  const setSelectedModel = useSiteStore((s) => s.setSelectedModel);
  const deleteModel = useSiteStore((s) => s.deleteModel);
  const clearModels = useSiteStore((s) => s.clearModels);
  const fetching = useSiteStore((s) => s.fetchingModels);
  const [query, setQuery] = useState("");
  const [clearing, setClearing] = useState(false);
  const [selecting, setSelecting] = useState(false);
  const [selectedIds, setSelectedIds] = useState<Set<string>>(() => new Set());
  const [deletingSelected, setDeletingSelected] = useState(false);
  const [testOpen, setTestOpen] = useState(false);
  const modelsFromStore = useSiteStore((s) => s.modelsBySite[site.id]);
  const models = modelsFromStore ?? modelsProp;
  const fetchError = site.lastModelFetchError
    ? modelFetchErrorPresentation(site.lastModelFetchError)
    : null;

  // Default primary model to the first fetched model.
  useEffect(() => {
    if (site.selectedModelId || models.length === 0) return;
    void setSelectedModel(site.id, models[0].modelId);
  }, [site.id, site.selectedModelId, models, setSelectedModel]);

  useEffect(() => {
    setSelecting(false);
    setSelectedIds(new Set());
    setTestOpen(false);
  }, [site.id]);

  useEffect(() => {
    if (models.length === 0) {
      setSelecting(false);
      setSelectedIds(new Set());
      return;
    }
    const valid = new Set(models.map((m) => m.modelId));
    setSelectedIds((prev) => {
      let changed = false;
      const next = new Set<string>();
      for (const id of prev) {
        if (valid.has(id)) next.add(id);
        else changed = true;
      }
      return changed ? next : prev;
    });
  }, [models]);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return models;
    return models.filter((m) => {
      const name = (m.displayName || m.modelId).toLowerCase();
      return name.includes(q) || m.modelId.toLowerCase().includes(q);
    });
  }, [models, query]);

  const groups = useMemo(() => groupModelsByPrefix(filtered), [filtered]);

  const handleFetch = async () => {
    if (onFetch) {
      await onFetch();
      return;
    }
    try {
      const result = await fetchModels(site.id);
      if (!site.selectedModelId && result.models[0]) {
        await setSelectedModel(site.id, result.models[0].modelId);
      }
    } catch (e) {
      message.error(
        isAppError(e)
          ? e.code === "unauthorized"
            ? t("sites.modelListUnauthorized")
            : t(`errors.${e.code}`)
          : t("sites.modelListFetchRetry"),
      );
    }
  };

  const handleDelete = async (modelId: string) => {
    try {
      await deleteModel(site.id, modelId);
    } catch (e) {
      message.error(isAppError(e) ? e.message : String(e));
    }
  };

  const toggleSelecting = () => {
    setSelecting((on) => !on);
    setSelectedIds(new Set());
  };

  const toggleSelected = (modelId: string) => {
    setSelectedIds((prev) => {
      const next = new Set(prev);
      if (next.has(modelId)) next.delete(modelId);
      else next.add(modelId);
      return next;
    });
  };

  const handleDeleteSelected = () => {
    const ids = Array.from(selectedIds);
    if (ids.length === 0) return;
    modal.confirm({
      title: t("sites.deleteSelectedModelsConfirm", { count: ids.length }),
      content: t("sites.deleteSelectedModelsHint"),
      centered: true,
      okText: t("common.confirm"),
      cancelText: t("common.cancel"),
      okButtonProps: { danger: true },
      onOk: async () => {
        setDeletingSelected(true);
        try {
          for (const id of ids) {
            await deleteModel(site.id, id);
          }
          setSelectedIds(new Set());
          message.success(t("sites.deleteSelectedModelsSuccess", { count: ids.length }));
        } catch (e) {
          message.error(isAppError(e) ? e.message : String(e));
          throw e;
        } finally {
          setDeletingSelected(false);
        }
      },
    });
  };

  const selectedCount = selectedIds.size;
  const showActionBar = selecting && selectedCount > 0;

  const handleClear = () => {
    modal.confirm({
      title: t("sites.clearModelsConfirm"),
      content: t("sites.clearModelsHint"),
      centered: true,
      okText: t("common.confirm"),
      cancelText: t("common.cancel"),
      okButtonProps: { danger: true },
      onOk: async () => {
        setClearing(true);
        try {
          await clearModels(site.id);
          message.success(t("sites.clearModelsSuccess"));
        } catch (e) {
          message.error(isAppError(e) ? e.message : String(e));
          throw e;
        } finally {
          setClearing(false);
        }
      },
    });
  };

  return (
    <div
      className="flex h-full min-h-0 flex-1 flex-col gap-3"
      style={{
        ["--model-tag-bg" as string]: token.colorFillTertiary,
        ["--model-tag-bg-hover" as string]: token.colorFillSecondary,
        ["--model-tag-border" as string]: token.colorBorderSecondary,
        ["--model-tag-border-hover" as string]: token.colorBorder,
        ["--model-tag-fg" as string]: token.colorText,
        ["--model-tag-selected-bg" as string]: token.colorPrimaryBg,
        ["--model-tag-selected-bg-hover" as string]: token.colorPrimaryBgHover,
        ["--model-tag-selected-border" as string]: token.colorPrimary,
        ["--model-tag-selected-border-hover" as string]: token.colorPrimaryHover,
        ["--model-tag-selected-fg" as string]: token.colorPrimary,
        ["--model-tag-close-hover" as string]: token.colorFillSecondary,
        ["--model-tag-close-active" as string]: token.colorFill,
        ["--model-count-bg" as string]: token.colorFillTertiary,
        ["--model-count-border" as string]: token.colorBorderSecondary,
        ["--model-count-fg" as string]: token.colorTextSecondary,
      }}
    >
      <div className="flex shrink-0 flex-wrap items-center gap-2">
        <span className="text-sm font-medium" style={{ color: token.colorText }}>
          {t("sites.selectModel")}
        </span>
        {site.selectedModelId && (
          <span className="text-xs font-normal" style={{ color: token.colorTextSecondary }}>
            {t("sites.currentModel", { model: site.selectedModelId })}
          </span>
        )}
        {onAddManual && (
          <Button size="small" icon={<Plus size={12} />} onClick={onAddManual}>
            {t("sites.manualModel")}
          </Button>
        )}
        <Button
          size="small"
          type={selecting ? "primary" : "default"}
          icon={<CheckSquare size={12} />}
          disabled={models.length === 0}
          onClick={toggleSelecting}
        >
          {selecting ? t("sites.multiSelectModelsDone") : t("sites.multiSelectModels")}
        </Button>
        <Tooltip title={!site.hasKey ? t("sites.testNoKey") : undefined}>
          <span>
            <Button
              size="small"
              icon={<FlaskConical size={12} />}
              disabled={models.length === 0 || !site.hasKey}
              onClick={() => setTestOpen(true)}
            >
              {t("sites.testModels")}
            </Button>
          </span>
        </Tooltip>
        <Button
          size="small"
          danger
          icon={<Trash2 size={12} />}
          disabled={models.length === 0}
          loading={clearing}
          onClick={handleClear}
        >
          {t("sites.clearModels")}
        </Button>
        {showFetchButton && (
          <Button size="small" loading={fetching} onClick={() => void handleFetch()}>
            {fetching ? t("sites.fetching") : t("sites.fetchModels")}
          </Button>
        )}
      </div>

      {site.lastModelFetchLatencyMs != null && (
        <div className="shrink-0 text-xs" style={{ color: token.colorTextSecondary }}>
          {t("sites.latency", { ms: site.lastModelFetchLatencyMs })}
          {models.length > 0 ? ` · ${t("sites.modelCount", { count: models.length })}` : ""}
        </div>
      )}

      {site.lastModelFetchError && (
        <div
          className="shrink-0 text-xs"
          style={{ color: token.colorError }}
          role="alert"
        >
          <div className="font-medium">{t("sites.modelListFetchFailed")}</div>
          <div>
            {fetchError?.details
              ? t(fetchError.messageKey, { details: fetchError.details })
              : t(fetchError?.messageKey ?? "sites.modelListFetchRetry")}
          </div>
        </div>
      )}

      <div className="shrink-0">
        <Input.Search
          allowClear
          placeholder={t("sites.modelSearch")}
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
      </div>

      <div className="relative min-h-0 flex-1">
        <div
          data-model-list
          className="scroll-y h-full min-h-0"
          style={{ minHeight: 160, paddingBottom: showActionBar ? 56 : undefined }}
        >
          {filtered.length === 0 ? (
            <div className="flex h-full min-h-[160px] flex-col items-center justify-center gap-3 px-4 py-6 text-center">
              <span className="text-sm" style={{ color: token.colorTextSecondary }}>
                {models.length === 0 ? t("sites.noModels") : t("sites.noModelsMatch")}
              </span>
              {models.length === 0 && (
                <Button
                  type="primary"
                  size="small"
                  icon={<RefreshCw size={14} />}
                  loading={fetching}
                  onClick={() => void handleFetch()}
                >
                  {fetching ? t("sites.fetching") : t("sites.fetchModels")}
                </Button>
              )}
            </div>
          ) : (
            <div className="flex flex-col gap-3">
              {groups.map((group) => (
                <div key={group.prefix} data-model-group={group.prefix}>
                  <div className="mb-1.5 flex items-center gap-1.5">
                    <span
                      className="text-xs font-medium"
                      style={{ color: token.colorTextTertiary }}
                    >
                      {group.prefix}
                    </span>
                    <ModelCountBadge>
                      {t("sites.modelCount", { count: group.models.length })}
                    </ModelCountBadge>
                  </div>
                  <div className="flex flex-wrap content-start gap-2">
                    {group.models.map((m) => {
                      const selected = site.selectedModelId === m.modelId;
                      const picked = selecting && selectedIds.has(m.modelId);
                      const label = m.displayName || m.modelId;
                      return (
                        <ModelTag
                          key={m.id || m.modelId}
                          title={m.modelId}
                          selected={selected}
                          picked={picked}
                          closable={selecting}
                          onClick={() => {
                            if (selecting) {
                              toggleSelected(m.modelId);
                              return;
                            }
                            void setSelectedModel(site.id, m.modelId);
                          }}
                          onClose={() => {
                            void handleDelete(m.modelId);
                          }}
                        >
                          {selecting && (
                            <Checkbox
                              checked={picked}
                              onClick={(e) => e.stopPropagation()}
                              onChange={() => toggleSelected(m.modelId)}
                            />
                          )}
                          {label}
                        </ModelTag>
                      );
                    })}
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
        {showActionBar && (
          <div
            data-model-multi-actions
            className="pointer-events-none absolute inset-x-0 bottom-0 z-10 flex justify-center px-2 pb-2"
          >
            <div
              className="pointer-events-auto flex items-center gap-3 rounded-full px-3 py-1.5"
              style={{
                backgroundColor: token.colorBgElevated,
                border: `1px solid ${token.colorBorderSecondary}`,
                boxShadow: token.boxShadowSecondary,
              }}
            >
              <span className="text-xs" style={{ color: token.colorTextSecondary }}>
                {t("sites.selectedModelCount", { count: selectedCount })}
              </span>
              <Button
                danger
                type="primary"
                size="small"
                loading={deletingSelected}
                onClick={handleDeleteSelected}
              >
                {t("sites.deleteSelectedModels", { count: selectedCount })}
              </Button>
            </div>
          </div>
        )}
      </div>
      <TestModelsModal
        open={testOpen}
        site={site}
        models={models}
        onClose={() => setTestOpen(false)}
      />
    </div>
  );
}
