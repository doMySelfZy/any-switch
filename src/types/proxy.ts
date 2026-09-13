import type { TargetKind } from "@/types/domain";

/**
 * 站点级请求头覆盖。`value` 支持 `${API_KEY}` / `${SESSION}` / `${UUID}` 占位符，
 * 只在转发时替换（不落库、不进日志）。
 */
export interface ProxyHeader {
  name: string;
  value: string;
  enabled: boolean;
}

/** 单个目标的接管状态；`clientBaseUrl` 是将会写进客户端配置的地址。 */
export interface LocalProxyTargetStatus {
  target: TargetKind;
  takeover: boolean;
  siteId: string | null;
  siteName: string | null;
  clientBaseUrl: string | null;
}

export interface LocalProxyStatus {
  running: boolean;
  address: string;
  port: number;
  /** 路径口令：属于本机凭据，UI 只展示截断形式。 */
  pathToken: string;
  startedAt: number | null;
  uptimeSeconds: number;
  totalRequests: number;
  successRequests: number;
  failedRequests: number;
  activeConnections: number;
  lastError: string | null;
  targets: LocalProxyTargetStatus[];
}

/** 请求日志条目：刻意不含请求头与请求体，`path` 已剥掉口令段。 */
export interface LocalProxyRequestLogEntry {
  id: number;
  at: number;
  target: string;
  method: string;
  path: string;
  status: number;
  durationMs: number;
  error: string | null;
}
