import ClaudeCode from "@lobehub/icons/es/ClaudeCode";
import Codex from "@lobehub/icons/es/Codex";
import Pi from "@lobehub/icons/es/Pi";
import { Bot, Sparkles } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { SkillTarget } from "@/types/domain";

export const SKILL_TARGETS: SkillTarget[] = ["agents", "claude_code", "codex", "pi", "prime"];

export function SkillTargetIcon({ target, size = 14 }: { target: SkillTarget; size?: number }) {
  if (target === "agents") return <Bot size={size} />;
  if (target === "claude_code") return <ClaudeCode size={size} />;
  if (target === "codex") return <Codex size={size} />;
  if (target === "pi") return <Pi size={size} />;
  return <Sparkles size={size} data-icon="prime" />;
}

export function SkillTargetLabel({ target }: { target: SkillTarget }) {
  const { t } = useTranslation();
  return (
    <span className="inline-flex items-center gap-2">
      <SkillTargetIcon target={target} />
      {t(skillTargetLabelKey(target))}
    </span>
  );
}

export function skillTargetLabelKey(target: SkillTarget): string {
  return `skills.target.${target}`;
}
