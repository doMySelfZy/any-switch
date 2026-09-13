import { create } from "zustand";
import { invoke } from "@/lib/invoke";
import type {
  McpApplyResult,
  McpSaveResult,
  McpServer,
  McpServerInput,
  McpServerSummary,
  RegistrySearchResult,
} from "@/types/mcp";
import type { TargetKind } from "@/types/domain";

interface McpState {
  servers: McpServerSummary[];
  loading: boolean;
  loadServers: () => Promise<void>;
  getServer: (id: string) => Promise<McpServer>;
  saveServer: (input: McpServerInput) => Promise<McpSaveResult>;
  deleteServer: (id: string) => Promise<McpApplyResult>;
  applyServers: (targets: TargetKind[]) => Promise<McpApplyResult>;
  searchRegistry: (
    query: string,
    options?: { cursor?: string | null; localOnly?: boolean },
  ) => Promise<RegistrySearchResult>;
}

export const useMcpStore = create<McpState>((set) => ({
  servers: [],
  loading: false,

  loadServers: async () => {
    set({ loading: true });
    try {
      const servers = await invoke<McpServerSummary[]>("list_mcp_servers");
      set({ servers, loading: false });
    } catch (error) {
      console.error("Failed to load MCP servers:", error);
      set({ loading: false });
    }
  },

  getServer: async (id: string) => invoke<McpServer>("get_mcp_server", { id }),

  saveServer: async (input: McpServerInput) => {
    const result = await invoke<McpSaveResult>("save_mcp_server", { input });
    const servers = await invoke<McpServerSummary[]>("list_mcp_servers");
    set({ servers });
    return result;
  },

  deleteServer: async (id: string) => {
    const result = await invoke<McpApplyResult>("delete_mcp_server", { id });
    const servers = await invoke<McpServerSummary[]>("list_mcp_servers");
    set({ servers });
    return result;
  },

  applyServers: async (targets: TargetKind[]) =>
    invoke<McpApplyResult>("apply_mcp_servers", { targets }),

  searchRegistry: async (query, options) =>
    invoke<RegistrySearchResult>("search_mcp_registry", {
      query,
      cursor: options?.cursor ?? null,
      localOnly: options?.localOnly ?? true,
    }),
}));
