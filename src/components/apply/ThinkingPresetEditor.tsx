import { useMemo, useState } from "react";
import { Collapse, Input, Select, Skeleton, Switch } from "antd";
import { useTranslation } from "react-i18next";
import { SettingsGroup } from "@/components/settings/SettingsGroup";
import {
  EXTENDED_THINKING_LEVELS,
  STANDARD_THINKING_LEVELS,
  allowsExtendedLevels,
} from "@/lib/thinkingPreset";
import type {
  ModelThinkingConfig,
  SiteModel,
  SiteProtocol,
  SiteThinkingPreset,
  ThinkingLevel,
} from "@/types/domain";

const rowStyle: React.CSSProperties = { padding: "4px 0" };

interface Props {
  preset: SiteThinkingPreset;
  protocol: SiteProtocol;
  models: SiteModel[];
  defaultModelId?: string;
  loading?: boolean;
  onChange: (next: SiteThinkingPreset) => void;
}

function modelCfg(
  preset: SiteThinkingPreset,
  id: string,
): ModelThinkingConfig {
  return preset.models[id] ?? { reasoning: false };
}

export function ThinkingPresetEditor({
  preset,
  protocol,
  models,
  defaultModelId,
  loading,
  onChange,
}: Props) {
  const { t } = useTranslation();
  const [query, setQuery] = useState("");
  const defaultCfg = defaultModelId ? modelCfg(preset, defaultModelId) : undefined;
  const extendedOk = allowsExtendedLevels(
    preset.target,
    protocol,
    Boolean(defaultCfg?.forceAdaptiveThinking),
  );
  const catalog = useMemo(() => {
    if (!defaultModelId || models.some((model) => model.modelId === defaultModelId)) {
      return models;
    }
    return [
      ...models,
      {
        id: defaultModelId,
        siteId: preset.siteId,
        modelId: defaultModelId,
        displayName: defaultModelId,
        ownedBy: null,
        raw: null,
      } satisfies SiteModel,
    ];
  }, [defaultModelId, models, preset.siteId]);
  const visibleModels = useMemo(() => {
    const q = query.trim().toLowerCase();
    return catalog.filter((model) => {
      if (!q) return true;
      return (
        model.modelId.toLowerCase().includes(q) ||
        model.displayName.toLowerCase().includes(q)
      );
    });
  }, [catalog, query]);

  const setModel = (id: string, patch: Partial<ModelThinkingConfig>) => {
    const current = modelCfg(preset, id);
    const nextCfg: ModelThinkingConfig = { ...current, ...patch };
    if (nextCfg.reasoning && !current.reasoning) {
      nextCfg.thinkingLevelMap = {
        ...nextCfg.thinkingLevelMap,
        ...(preset.extended.xhigh?.trim()
          ? { xhigh: preset.extended.xhigh.trim() }
          : {}),
        ...(preset.extended.max?.trim() ? { max: preset.extended.max.trim() } : {}),
      };
    }
    if (!nextCfg.reasoning) nextCfg.forceAdaptiveThinking = false;
    onChange({ ...preset, models: { ...preset.models, [id]: nextCfg } });
  };

  const setExtended = (key: "xhigh" | "max", value: string) => {
    const trimmed = value.trim();
    const extended = { ...preset.extended, [key]: trimmed || null };
    const modelsNext = { ...preset.models };
    for (const [id, cfg] of Object.entries(modelsNext)) {
      if (!cfg.reasoning) continue;
      const map = { ...cfg.thinkingLevelMap };
      if (trimmed) map[key] = trimmed;
      else delete map[key];
      modelsNext[id] = { ...cfg, thinkingLevelMap: map };
    }
    onChange({ ...preset, extended, models: modelsNext });
  };

  const levelOptions = [
    { value: "", label: t("apply.thinkingUnchanged") },
    ...STANDARD_THINKING_LEVELS.map((level) => ({
      value: level,
      label: t(`apply.thinkingLevel_${level}`),
    })),
    ...(extendedOk || preset.defaultLevel === "xhigh" || preset.defaultLevel === "max"
      ? EXTENDED_THINKING_LEVELS.map((level) => ({
          value: level,
          label: t(`apply.thinkingLevel_${level}`),
          disabled: !extendedOk,
        }))
      : []),
  ];

  return (
    <SettingsGroup title={t("apply.groupThinking")}>
      {loading ? (
        <Skeleton active paragraph={{ rows: 4 }} title={false} />
      ) : (
        <>
          <div style={rowStyle}>
            <div className="mb-1 text-sm opacity-70">{t("apply.thinkingDefault")}</div>
            <Select
              className="w-full"
              value={preset.defaultLevel ?? ""}
              options={levelOptions}
              onChange={(value) =>
                onChange({
                  ...preset,
                  defaultLevel: (value || null) as ThinkingLevel | null,
                })
              }
            />
            <div className="mt-1 text-xs opacity-50">{t("apply.thinkingDefaultHint")}</div>
          </div>

          <Collapse
            size="small"
            className="mt-3"
            items={[
              {
                key: "extended",
                label: t("apply.thinkingAdvanced"),
                children: (
                  <div className="flex flex-col gap-3">
                    <div>
                      <div className="mb-1 text-sm opacity-70">{t("apply.thinkingXhighValue")}</div>
                      <Input
                        allowClear
                        value={preset.extended.xhigh ?? ""}
                        placeholder="xhigh"
                        onChange={(event) => setExtended("xhigh", event.target.value)}
                      />
                    </div>
                    <div>
                      <div className="mb-1 text-sm opacity-70">{t("apply.thinkingMaxValue")}</div>
                      <Input
                        allowClear
                        value={preset.extended.max ?? ""}
                        placeholder="max"
                        onChange={(event) => setExtended("max", event.target.value)}
                      />
                    </div>
                    <div className="text-xs opacity-50">{t("apply.thinkingAdvancedHint")}</div>
                  </div>
                ),
              },
            ]}
          />

          <div className="mt-4 mb-1 text-sm opacity-70">{t("apply.thinkingModels")}</div>
          <Input
            allowClear
            className="mb-2"
            value={query}
            placeholder={t("apply.thinkingSearchModels")}
            onChange={(event) => setQuery(event.target.value)}
          />
          <div className="max-h-56 overflow-y-auto">
            {visibleModels.length === 0 ? (
              <div className="text-xs opacity-50">{t("sites.noModels")}</div>
            ) : (
              visibleModels.map((model) => {
                const cfg = modelCfg(preset, model.modelId);
                return (
                  <div key={model.modelId} className="border-t border-[var(--border-color)] py-2">
                    <div className="flex items-center justify-between gap-4">
                      <div className="min-w-0">
                        <div className="truncate text-sm">{model.displayName || model.modelId}</div>
                        {model.displayName && model.displayName !== model.modelId ? (
                          <div className="truncate text-xs opacity-50">{model.modelId}</div>
                        ) : null}
                      </div>
                      <Switch
                        checked={cfg.reasoning}
                        onChange={(checked) => setModel(model.modelId, { reasoning: checked })}
                      />
                    </div>
                    {preset.target === "pi" && protocol === "anthropic" && cfg.reasoning ? (
                      <div className="mt-2 flex items-center justify-between gap-4">
                        <div>
                          <div className="text-sm">{t("apply.thinkingAdaptive")}</div>
                          <div className="text-xs opacity-50">{t("apply.thinkingAdaptiveHint")}</div>
                        </div>
                        <Switch
                          checked={Boolean(cfg.forceAdaptiveThinking)}
                          onChange={(checked) =>
                            setModel(model.modelId, { forceAdaptiveThinking: checked })
                          }
                        />
                      </div>
                    ) : null}
                  </div>
                );
              })
            )}
          </div>
        </>
      )}
    </SettingsGroup>
  );
}
