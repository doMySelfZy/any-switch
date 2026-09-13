import { create } from "zustand";
import { invoke } from "@/lib/invoke";
import type {
  AgentRules,
  AgentRulesApplyResult,
  AgentRulesTargetPath,
} from "@/types/rules";
import type { TargetKind } from "@/types/domain";

interface RulesState {
  body: string;
  targets: TargetKind[];
  updatedAt: number;
  loading: boolean;
  paths: AgentRulesTargetPath[];
  load: () => Promise<void>;
  loadPaths: () => Promise<void>;
  save: (body: string, targets: TargetKind[]) => Promise<AgentRulesApplyResult>;
}

export const useRulesStore = create<RulesState>((set) => ({
  body: "",
  targets: [],
  updatedAt: 0,
  loading: false,
  paths: [],

  load: async () => {
    set({ loading: true });
    try {
      const rules = await invoke<AgentRules>("get_agent_rules");
      set({
        body: rules.body,
        targets: rules.targets,
        updatedAt: rules.updatedAt,
        loading: false,
      });
    } catch (error) {
      console.error("Failed to load agent rules:", error);
      set({ loading: false });
    }
  },

  loadPaths: async () => {
    try {
      set({ paths: await invoke<AgentRulesTargetPath[]>("agent_rules_target_paths") });
    } catch (error) {
      console.error("Failed to load agent rules target paths:", error);
      set({ paths: [] });
    }
  },

  save: async (body, targets) => {
    const result = await invoke<AgentRulesApplyResult>("save_agent_rules", { body, targets });
    set({ body, targets, updatedAt: Date.now() });
    return result;
  },
}));

/** 测试与浏览器 mock 用：把 store 复位到初始状态。 */
export function resetRulesStore() {
  useRulesStore.setState({
    body: "",
    targets: [],
    updatedAt: 0,
    loading: false,
    paths: [],
  });
}
