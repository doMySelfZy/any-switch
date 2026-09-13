import { create } from "zustand";
import { invoke } from "@/lib/invoke";
import type {
  LocalProxyRequestLogEntry,
  LocalProxyStatus,
  ProxyHeader,
} from "@/types/proxy";
import type { TargetKind } from "@/types/domain";

interface ProxyState {
  status: LocalProxyStatus | null;
  requests: LocalProxyRequestLogEntry[];
  loading: boolean;
  loadStatus: () => Promise<void>;
  loadRequests: (limit?: number) => Promise<void>;
  start: () => Promise<void>;
  stop: () => Promise<void>;
  setTakeover: (target: TargetKind, enabled: boolean) => Promise<void>;
  setPort: (port: number) => Promise<void>;
  clearRequests: () => Promise<void>;
  getSiteProxyHeaders: (siteId: string) => Promise<ProxyHeader[]>;
}

/**
 * 本地代理状态与动作。错误交给调用方（页面用 message/modal 展示），
 * 与 mcpStore 的约定一致。
 *
 * 轮询类读取（loadStatus/loadRequests）内部静默降级：后端在代理状态
 * 互斥锁被占用时会短暂返回 `local proxy state busy`，不该把未处理 rejection
 * 抛进控制台。
 */
export const useProxyStore = create<ProxyState>((set) => ({
  status: null,
  requests: [],
  loading: false,

  loadStatus: async () => {
    set({ loading: true });
    try {
      const status = await invoke<LocalProxyStatus>("local_proxy_status");
      set({ status, loading: false });
    } catch (error) {
      console.error("Failed to load local proxy status:", error);
      set({ loading: false });
    }
  },

  loadRequests: async (limit = 100) => {
    try {
      const requests = await invoke<LocalProxyRequestLogEntry[]>(
        "list_local_proxy_requests",
        { limit },
      );
      set({ requests });
    } catch (error) {
      console.error("Failed to load local proxy requests:", error);
    }
  },

  start: async () => {
    const status = await invoke<LocalProxyStatus>("start_local_proxy");
    set({ status });
  },

  stop: async () => {
    const status = await invoke<LocalProxyStatus>("stop_local_proxy");
    set({ status });
  },

  setTakeover: async (target: TargetKind, enabled: boolean) => {
    const status = await invoke<LocalProxyStatus>("set_local_proxy_takeover", {
      target,
      enabled,
    });
    set({ status });
  },

  setPort: async (port: number) => {
    const status = await invoke<LocalProxyStatus>("set_local_proxy_port", { port });
    set({ status });
  },

  clearRequests: async () => {
    try {
      await invoke("clear_local_proxy_requests");
      set({ requests: [] });
    } catch (error) {
      console.error("Failed to clear local proxy requests:", error);
    }
  },

  getSiteProxyHeaders: (siteId: string) =>
    invoke<ProxyHeader[]>("get_site_proxy_headers", { siteId }),
}));
