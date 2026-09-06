import { useEffect, useState } from "react";
import {
  emptyThinkingPreset,
  getSiteThinkingPreset,
} from "@/lib/thinkingPreset";
import type { SiteThinkingPreset, ThinkingTarget } from "@/types/domain";

export function useThinkingPreset(siteId: string | null | undefined, target: ThinkingTarget) {
  const [preset, setPreset] = useState<SiteThinkingPreset>(() =>
    emptyThinkingPreset(siteId ?? "", target),
  );
  const [loading, setLoading] = useState(Boolean(siteId));

  useEffect(() => {
    if (!siteId) {
      setPreset(emptyThinkingPreset("", target));
      setLoading(false);
      return;
    }
    let cancelled = false;
    setLoading(true);
    setPreset(emptyThinkingPreset(siteId, target));
    void getSiteThinkingPreset(siteId, target)
      .then((next) => {
        if (!cancelled) setPreset({ ...next, siteId, target });
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [siteId, target]);

  return { preset, setPreset, loading };
}
