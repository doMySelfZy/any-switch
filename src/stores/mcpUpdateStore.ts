import { create } from "zustand";
import { invoke } from "@/lib/invoke";

export interface McpUpdateStatus {
  id: string;
  name: string;
  currentVersion: string | null;
  latestVersion: string | null;
  hasUpdate: boolean;
  lastCheckAt: number | null;
}

export interface McpUpdateState {
  updateStatuses: McpUpdateStatus[];
  checking: boolean;
  updating: Record<string, boolean>;
  lastCheckTime: number | null;

  // Getters
  hasAnyUpdate: () => boolean;
  updateCount: () => number;
  updatableServers: () => McpUpdateStatus[];
  getUpdateStatus: (id: string) => McpUpdateStatus | undefined;
  isUpdating: (id: string) => boolean;

  // Actions
  checkUpdates: () => Promise<void>;
  updateServer: (id: string) => Promise<string | undefined>;
  batchUpdate: (ids: string[]) => Promise<{ successes: string[]; failures: Array<[string, string]> }>;
  updateAll: () => Promise<{ successes: string[]; failures: Array<[string, string]> }>;
  clearUpdateStatus: (id: string) => void;
  reset: () => void;
}

export const useMcpUpdateStore = create<McpUpdateState>((set, get) => ({
  updateStatuses: [],
  checking: false,
  updating: {},
  lastCheckTime: null,

  // Getters
  hasAnyUpdate: () => {
    return get().updateStatuses.some((status: McpUpdateStatus) => status.hasUpdate);
  },

  updateCount: () => {
    return get().updateStatuses.filter((status: McpUpdateStatus) => status.hasUpdate).length;
  },

  updatableServers: () => {
    return get().updateStatuses.filter((status: McpUpdateStatus) => status.hasUpdate);
  },

  getUpdateStatus: (id: string) => {
    return get().updateStatuses.find((status: McpUpdateStatus) => status.id === id);
  },

  isUpdating: (id: string) => {
    return get().updating[id] || false;
  },

  // Actions
  checkUpdates: async () => {
    if (get().checking) return;

    set({ checking: true });
    try {
      const statuses = await invoke<McpUpdateStatus[]>('check_mcp_updates');
      set({ updateStatuses: statuses, lastCheckTime: Date.now() });
    } catch (error) {
      console.error('Failed to check MCP updates:', error);
      throw error;
    } finally {
      set({ checking: false });
    }
  },

  updateServer: async (id: string) => {
    const state = get();
    if (state.updating[id]) return;

    set({ updating: { ...state.updating, [id]: true } });
    try {
      const version = await invoke<string>('update_mcp_server', { id });

      // 更新本地状态
      const statuses = state.updateStatuses.map((s: McpUpdateStatus) => {
        if (s.id === id) {
          return {
            ...s,
            currentVersion: version,
            latestVersion: version,
            hasUpdate: false,
          };
        }
        return s;
      });
      set({ updateStatuses: statuses });

      return version;
    } catch (error) {
      console.error(`Failed to update MCP server ${id}:`, error);
      throw error;
    } finally {
      const newUpdating = { ...get().updating };
      delete newUpdating[id];
      set({ updating: newUpdating });
    }
  },

  batchUpdate: async (ids: string[]) => {
    const results = await invoke<Array<[string, { Ok?: string; Err?: string }]>>(
      'batch_update_mcp_servers',
      { ids }
    );

    const successes: string[] = [];
    const failures: Array<[string, string]> = [];
    const state = get();

    results.forEach(([id, result]) => {
      if ('Ok' in result) {
        successes.push(id);
      } else if ('Err' in result) {
        failures.push([id, result.Err!]);
      }
    });

    // 更新本地状态
    const statuses = state.updateStatuses.map((s: McpUpdateStatus) => {
      if (successes.includes(s.id)) {
        const result = results.find(([id]) => id === s.id);
        const version = result && 'Ok' in result[1] ? result[1].Ok! : s.currentVersion;
        return {
          ...s,
          currentVersion: version,
          latestVersion: version,
          hasUpdate: false,
        };
      }
      return s;
    });
    set({ updateStatuses: statuses });

    return { successes, failures };
  },

  updateAll: async () => {
    const ids: string[] = get().updatableServers().map((s: McpUpdateStatus) => s.id);
    if (ids.length === 0) return { successes: [], failures: [] };

    return get().batchUpdate(ids);
  },

  clearUpdateStatus: (id: string) => {
    const statuses = get().updateStatuses.filter((s: McpUpdateStatus) => s.id !== id);
    set({ updateStatuses: statuses });
  },

  reset: () => {
    set({
      updateStatuses: [],
      checking: false,
      updating: {},
      lastCheckTime: null,
    });
  },
}));

