import { invoke } from "@/lib/invoke";
import type {
  SiteProtocol,
  SiteThinkingPreset,
  ThinkingLevel,
  ThinkingTarget,
} from "@/types/domain";

export const STANDARD_THINKING_LEVELS: ThinkingLevel[] = [
  "off",
  "minimal",
  "low",
  "medium",
  "high",
];
export const EXTENDED_THINKING_LEVELS: ThinkingLevel[] = ["xhigh", "max"];

export function emptyThinkingPreset(
  siteId: string,
  target: ThinkingTarget,
): SiteThinkingPreset {
  return { siteId, target, defaultLevel: null, extended: {}, models: {} };
}

export function allowsExtendedLevels(
  target: ThinkingTarget,
  protocol: SiteProtocol,
  forceAdaptive: boolean,
): boolean {
  if (protocol === "openai_compatible") return true;
  if (target === "pi" && protocol === "anthropic") return forceAdaptive;
  return false;
}

export function thinkingValidationKey(
  preset: SiteThinkingPreset,
  protocol: SiteProtocol,
  defaultModelId: string | undefined,
): string | null {
  const level = preset.defaultLevel;
  if (!level) return null;
  if (!defaultModelId) return "apply.thinkingNeedReasoning";
  const cfg = preset.models[defaultModelId];
  if (!cfg?.reasoning) return "apply.thinkingNeedReasoning";
  if (level !== "xhigh" && level !== "max") return null;
  if (preset.target === "prime" && protocol === "anthropic") {
    return "apply.thinkingPrimeAnthropicNoExtended";
  }
  if (!allowsExtendedLevels(preset.target, protocol, Boolean(cfg.forceAdaptiveThinking))) {
    return "apply.thinkingNeedAdaptive";
  }
  const mapped = cfg.thinkingLevelMap?.[level]?.trim() || preset.extended[level]?.trim();
  if (!mapped) return "apply.thinkingNeedMapping";
  return null;
}

export async function getSiteThinkingPreset(
  siteId: string,
  target: ThinkingTarget,
): Promise<SiteThinkingPreset> {
  return invoke<SiteThinkingPreset>("get_site_thinking_preset", { siteId, target });
}

export async function saveSiteThinkingPreset(
  preset: SiteThinkingPreset,
): Promise<SiteThinkingPreset> {
  return invoke<SiteThinkingPreset>("save_site_thinking_preset", { preset });
}
