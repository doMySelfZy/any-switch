import { Input, Typography, theme } from "antd";
import { useTranslation } from "react-i18next";
import type { ProxyHeader } from "@/types/proxy";

/** RFC 9110 field-name token，与 Rust 侧校验保持一致。 */
const HEADER_NAME_RE = /^[!#$%&'*+\-.^_`|~0-9A-Za-z]+$/;

/** 除 tab 外的控制字符都不允许，防止请求头注入。 */
// eslint-disable-next-line no-control-regex
const CONTROL_CHAR_RE = /[\u0000-\u0008\u000a-\u001f\u007f]/;

/** 与后端 `is_protected_header` 同一份名单。 */
const PROTECTED_NAMES = new Set([
  "host",
  "content-length",
  "transfer-encoding",
  "connection",
  "keep-alive",
  "te",
  "trailer",
  "upgrade",
  "proxy-authorization",
  "proxy-authenticate",
  "content-type",
]);

export interface ParsedProxyHeaders {
  headers?: ProxyHeader[];
  error?: string;
}

/**
 * 解析请求头 JSON 文本。空文本表示"不配置"，返回空数组而不是错误。
 *
 * 校验与后端同规则：非法名/值、受保护头、大小写归一后重名都在这里拦下，
 * 避免保存成功却在转发时被静默丢弃。
 */
export function parseProxyHeadersJson(raw: string): ParsedProxyHeaders {
  const trimmed = raw.trim();
  if (!trimmed) return { headers: [] };
  let parsed: unknown;
  try {
    parsed = JSON.parse(trimmed);
  } catch (error) {
    return { error: error instanceof Error ? error.message : String(error) };
  }
  if (typeof parsed !== "object" || parsed === null) {
    return { error: "JSON must be an object or an array of {name,value,enabled}" };
  }

  // 接受两种写法：`[{"name":"X-Foo","value":"bar","enabled":true}]`
  // 与更省事的 `{"X-Foo":"bar"}`（后者默认启用）。
  let items: ProxyHeader[];
  if (Array.isArray(parsed)) {
    items = parsed as ProxyHeader[];
  } else {
    items = Object.entries(parsed as Record<string, unknown>).map(([name, value]) => ({
      name,
      value: String(value),
      enabled: true,
    }));
  }

  const seen = new Set<string>();
  const headers: ProxyHeader[] = [];
  for (const item of items) {
    if (!item || typeof item !== "object" || typeof item.name !== "string") {
      return { error: "each entry needs a string `name`" };
    }
    const name = item.name.trim();
    const value = typeof item.value === "string" ? item.value : String(item.value ?? "");
    const enabled = item.enabled !== false;
    if (!name) return { error: "header name must not be empty" };
    if (!HEADER_NAME_RE.test(name)) {
      return { error: `header "${item.name}" is not a valid HTTP token` };
    }
    if (CONTROL_CHAR_RE.test(value)) {
      return { error: `header "${item.name}" value contains control characters` };
    }
    const lower = name.toLowerCase();
    if (PROTECTED_NAMES.has(lower)) {
      return { error: `header "${item.name}" is managed by the proxy and cannot be overridden` };
    }
    if (seen.has(lower)) {
      return { error: `header "${item.name}" duplicates another entry after case normalization` };
    }
    seen.add(lower);
    headers.push({ name, value, enabled });
  }
  return { headers };
}

interface Props {
  value: string;
  onChange: (next: string) => void;
  error: string | null;
}

/**
 * 站点级代理请求头编辑器。
 *
 * 只影响走本地代理的流量；占位符在转发时替换，不落库。
 */
export function ProxyHeaderEditor({ value, onChange, error }: Props) {
  const { t } = useTranslation();
  const { token } = theme.useToken();

  return (
    <div className="mt-3">
      <div className="mb-1 text-sm">{t("sites.proxyHeaders")}</div>
      <Typography.Text type="secondary" style={{ fontSize: 12 }}>
        {t("sites.proxyHeadersHint")}
      </Typography.Text>
      <Input.TextArea
        className="mt-2"
        rows={5}
        allowClear
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={'{\n  "x-opencode-session": "${SESSION}"\n}'}
        status={error ? "error" : undefined}
      />
      {error ? (
        <div style={{ color: token.colorError, fontSize: 12, marginTop: 4 }}>{error}</div>
      ) : (
        <Typography.Text type="secondary" style={{ fontSize: 12 }}>
          {t("sites.proxyHeadersPlaceholders")}
        </Typography.Text>
      )}
    </div>
  );
}
