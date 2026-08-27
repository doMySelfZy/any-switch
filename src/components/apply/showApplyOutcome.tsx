import type { ReactNode } from "react";
import type { TFunction } from "i18next";
import type { ApplyTargetResult, TargetKind } from "@/types/domain";
import { isAppError } from "@/lib/invoke";

interface ModalApi {
  success: (config: {
    centered?: boolean;
    title: ReactNode;
    content?: ReactNode;
    okText?: string;
  }) => void;
  error: (config: {
    centered?: boolean;
    title: ReactNode;
    content?: ReactNode;
    okText?: string;
  }) => void;
}

export function applyResultBodyKey(
  target: TargetKind,
): "apply.resultClaudeOk" | "apply.resultCodexOk" | "apply.resultPiOk" {
  if (target === "claude_code") return "apply.resultClaudeOk";
  return target === "codex" ? "apply.resultCodexOk" : "apply.resultPiOk";
}

export function showApplyOutcome(
  modal: ModalApi,
  t: TFunction,
  result: ApplyTargetResult | undefined,
) {
  if (!result) {
    modal.error({
      centered: true,
      title: t("apply.failed"),
      content: t("apply.failedHint"),
      okText: t("common.confirm"),
    });
    return;
  }

  if (result.ok) {
    modal.success({
      centered: true,
      title: t("apply.success"),
      content: (
        <div>
          <div>{t(applyResultBodyKey(result.target))}</div>
          <div className="mt-2">{t("apply.restartHint")}</div>
        </div>
      ),
      okText: t("common.confirm"),
    });
    return;
  }

  modal.error({
    centered: true,
    title: t("apply.failed"),
    content: t("apply.failedHint"),
    okText: t("common.confirm"),
  });
}

export function showApplyException(modal: ModalApi, t: TFunction, error: unknown) {
  const content = isAppError(error) ? t(`errors.${error.code}`) : String(error);
  modal.error({
    centered: true,
    title: t("apply.failed"),
    content,
    okText: t("common.confirm"),
  });
}
