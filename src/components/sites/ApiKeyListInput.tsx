import type { ReactNode } from "react";
import { Form, Input, Spin, Tooltip, theme } from "antd";
import type { FormListFieldData } from "antd";
import { Copy, Minus, Plus, Zap } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { Site } from "@/types/domain";
import { siteApiKeys } from "@/lib/siteApiKey";

const CONTROL =
  "inline-flex h-8 w-8 shrink-0 items-center justify-center rounded-md transition-colors hover:bg-black/[0.06] disabled:pointer-events-none";

export interface ApiKeyDraft {
  id?: string;
  label?: string;
  apiKey?: string;
}

export function normalizeApiKeyDrafts(
  rows: unknown,
): { id: string | null; label: string | null; apiKey: string }[] {
  if (!Array.isArray(rows)) return [];
  return rows.flatMap((row) => {
    if (!row || typeof row !== "object") return [];
    const draft = row as ApiKeyDraft;
    const apiKey = String(draft.apiKey ?? "").trim();
    if (!apiKey) return [];
    const label = String(draft.label ?? "").trim();
    const id = String(draft.id ?? "").trim();
    return [{ id: id || null, label: label || null, apiKey }];
  });
}

export async function loadSiteKeyDrafts(
  site: Site,
  getSiteApiKey: (id: string, apiKeyId?: string) => Promise<string>,
): Promise<ApiKeyDraft[]> {
  const keys = siteApiKeys(site);
  const summaries = [...keys.filter((key) => key.isActive), ...keys.filter((key) => !key.isActive)];
  if (summaries.length === 0) {
    if (!site.hasKey) return [{ label: "", apiKey: "" }];
    return [{ id: site.activeApiKeyId ?? undefined, apiKey: await getSiteApiKey(site.id) }];
  }
  return Promise.all(
    summaries.map(async (key) => ({
      id: key.id,
      label: key.label,
      apiKey: await getSiteApiKey(site.id, key.id),
    })),
  );
}

export function ApiKeyListInput({
  disabled,
  onCopy,
  onTest,
  testingIndex,
}: {
  disabled?: boolean;
  onCopy?: (index: number) => void;
  onTest?: (index: number) => void;
  testingIndex?: number | null;
}) {
  const { t } = useTranslation();

  return (
    <Form.List
      name="apiKeys"
      rules={[
        {
          validator: async (_, rows: ApiKeyDraft[]) => {
            if (!normalizeApiKeyDrafts(rows).length) {
              return Promise.reject(new Error(t("sites.apiKey")));
            }
          },
        },
      ]}
    >
      {(fields, { add, remove }, { errors }) => (
        <div className="flex flex-col gap-2">
          {fields.map((field, index) => (
            <ApiKeyRow
              key={field.key}
              field={field}
              canRemove={fields.length > 1}
              disabled={disabled}
              extra={
                onCopy || onTest ? (
                  <KeyProbeActions
                    disabled={disabled}
                    testing={testingIndex === field.name}
                    onCopy={onCopy ? () => onCopy(field.name) : undefined}
                    onTest={onTest ? () => onTest(field.name) : undefined}
                  />
                ) : null
              }
              onAdd={() => add({ label: "", apiKey: "" }, index + 1)}
              onRemove={() => remove(field.name)}
            />
          ))}
          <Form.ErrorList errors={errors} />
        </div>
      )}
    </Form.List>
  );
}

function KeyProbeActions({
  disabled,
  testing,
  onCopy,
  onTest,
}: {
  disabled?: boolean;
  testing?: boolean;
  onCopy?: () => void;
  onTest?: () => void;
}) {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  return (
    <>
      {onCopy ? (
        <Tooltip title={t("common.copy")}>
          <button
            type="button"
            className={CONTROL}
            style={{ color: token.colorTextSecondary }}
            aria-label={t("common.copy")}
            disabled={disabled}
            onClick={onCopy}
          >
            <Copy size={14} className="block" />
          </button>
        </Tooltip>
      ) : null}
      {onTest ? (
        <Tooltip title={t("sites.testKey")}>
          <button
            type="button"
            className={CONTROL}
            style={{ color: token.colorTextSecondary }}
            aria-label={t("sites.testKey")}
            disabled={disabled || testing}
            onClick={onTest}
          >
            {testing ? <Spin size="small" /> : <Zap size={14} className="block" />}
          </button>
        </Tooltip>
      ) : null}
    </>
  );
}

function ApiKeyRow({
  field,
  canRemove,
  disabled,
  extra,
  onAdd,
  onRemove,
}: {
  field: FormListFieldData;
  canRemove: boolean;
  disabled?: boolean;
  extra?: ReactNode;
  onAdd: () => void;
  onRemove: () => void;
}) {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const { key: _key, name, ...item } = field;

  return (
    <div className="flex items-center gap-1.5">
      <Form.Item {...item} name={[name, "id"]} hidden>
        <Input />
      </Form.Item>
      <Form.Item {...item} name={[name, "label"]} noStyle>
        <Input
          className="!w-24"
          allowClear
          disabled={disabled}
          placeholder={t("sites.keyNamePlaceholder")}
        />
      </Form.Item>
      <div className="min-w-0 flex-1">
        <Form.Item
          {...item}
          name={[name, "apiKey"]}
          noStyle
          rules={[{ required: true, message: t("sites.apiKey") }]}
        >
          <Input placeholder="sk-..." allowClear autoComplete="off" disabled={disabled} />
        </Form.Item>
      </div>
      {extra}
      <button
        type="button"
        className={CONTROL}
        style={{ color: token.colorTextSecondary }}
        aria-label={t("sites.addKey")}
        disabled={disabled}
        onClick={onAdd}
      >
        <Plus size={14} className="block" />
      </button>
      <button
        type="button"
        className={CONTROL}
        style={{
          color: token.colorTextSecondary,
          opacity: canRemove ? 1 : 0.35,
        }}
        aria-label={t("sites.removeApiKey")}
        disabled={disabled || !canRemove}
        onClick={onRemove}
      >
        <Minus size={14} className="block" />
      </button>
    </div>
  );
}
