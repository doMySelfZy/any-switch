import { create } from "zustand";
import { invoke } from "@/lib/invoke";
import type {
  MarketplaceSkill,
  Skill,
  SkillDetail,
  SkillTarget,
} from "@/types/domain";

export type SkillMarketplaceSource = "skills.sh" | "github";

let marketplaceRequest = 0;

function listSkills(): Promise<Skill[]> {
  return invoke<Skill[]>("list_skills");
}

interface SkillState {
  skills: Skill[];
  marketplaceSkills: MarketplaceSkill[];
  selectedSkill: SkillDetail | null;
  loading: boolean;
  marketplaceLoading: boolean;
  hydrated: boolean;
  loadSkills: (force?: boolean) => Promise<void>;
  getSkill: (target: SkillTarget, sourcePath: string) => Promise<SkillDetail>;
  clearSelectedSkill: () => void;
  setSkillEnabled: (
    target: SkillTarget,
    sourcePath: string,
    enabled: boolean,
  ) => Promise<void>;
  installSkill: (source: string, target: SkillTarget) => Promise<string>;
  uninstallSkill: (target: SkillTarget, sourcePath: string) => Promise<void>;
  searchMarketplace: (query: string, source: SkillMarketplaceSource) => Promise<void>;
}

export const useSkillStore = create<SkillState>((set, get) => ({
  skills: [],
  marketplaceSkills: [],
  selectedSkill: null,
  loading: false,
  marketplaceLoading: false,
  hydrated: false,

  loadSkills: async (force = false) => {
    if (get().hydrated && !force) return;
    set({ loading: true });
    try {
      const skills = await listSkills();
      set({ skills, hydrated: true });
    } finally {
      set({ loading: false });
    }
  },

  getSkill: async (target, sourcePath) => {
    const detail = await invoke<SkillDetail>("get_skill", { target, sourcePath });
    set({ selectedSkill: detail });
    return detail;
  },

  clearSelectedSkill: () => set({ selectedSkill: null }),

  setSkillEnabled: async (target, sourcePath, enabled) => {
    await invoke("set_skill_enabled", { target, sourcePath, enabled });
    set({ skills: await listSkills(), hydrated: true });
  },

  installSkill: async (source, target) => {
    const name = await invoke<string>("install_skill", { source, target });
    set({ skills: await listSkills(), hydrated: true });
    return name;
  },

  uninstallSkill: async (target, sourcePath) => {
    await invoke("uninstall_skill", { target, sourcePath });
    set({ skills: await listSkills(), hydrated: true });
  },

  searchMarketplace: async (query, source) => {
    const request = ++marketplaceRequest;
    set({ marketplaceLoading: true });
    try {
      const marketplaceSkills = await invoke<MarketplaceSkill[]>(
        "search_skill_marketplace",
        { query, source },
      );
      if (request === marketplaceRequest) set({ marketplaceSkills });
    } finally {
      if (request === marketplaceRequest) set({ marketplaceLoading: false });
    }
  },
}));
