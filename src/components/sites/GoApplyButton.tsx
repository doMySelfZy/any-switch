import { useEffect, useState } from "react";
import { Button } from "antd";
import ClaudeCode from "@lobehub/icons/es/ClaudeCode";
import Codex from "@lobehub/icons/es/Codex";
import Pi from "@lobehub/icons/es/Pi";
import { Sparkles } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { ApplyTargetTab } from "@/stores";

const CYCLE_MS = 3000;
const FADE_MS = 180;
const TABS: ApplyTargetTab[] = ["claude_code", "codex", "pi", "prime"];

function nextTab(current: ApplyTargetTab): ApplyTargetTab {
  return TABS[(TABS.indexOf(current) + 1) % TABS.length];
}

interface Props {
  disabled?: boolean;
  onApply: (tab: ApplyTargetTab) => void;
}

export function GoApplyButton({ disabled, onApply }: Props) {
  const { t } = useTranslation();
  const [tab, setTab] = useState<ApplyTargetTab>("claude_code");
  const [leaving, setLeaving] = useState(false);

  useEffect(() => {
    const reduce =
      typeof window !== "undefined" &&
      window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    const id = window.setInterval(() => {
      if (reduce) {
        setTab(nextTab);
        return;
      }
      setLeaving(true);
      window.setTimeout(() => {
        setTab(nextTab);
        setLeaving(false);
      }, FADE_MS);
    }, CYCLE_MS);
    return () => window.clearInterval(id);
  }, []);

  const label =
    tab === "claude_code"
      ? t("sites.goApplyClaude")
      : tab === "codex"
        ? t("sites.goApplyCodex")
        : tab === "pi"
          ? t("sites.goApplyPi")
          : t("sites.goApplyPrime");
  const icon =
    tab === "claude_code" ? (
      <ClaudeCode size={14} />
    ) : tab === "codex" ? (
      <Codex size={14} />
    ) : tab === "pi" ? (
      <Pi size={14} />
    ) : (
      <Sparkles size={14} />
    );

  return (
    <Button type="primary" size="small" disabled={disabled} onClick={() => onApply(tab)}>
      <span
        className="go-apply-swap inline-flex items-center gap-1 whitespace-nowrap"
        data-leaving={leaving ? "true" : "false"}
      >
        {icon}
        {label}
      </span>
    </Button>
  );
}
