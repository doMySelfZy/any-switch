import type { TFunction } from "i18next";
import { isAppError } from "./invoke";

/**
 * 指纹算法不兼容（错误码 `sync_algorithm_mismatch`）要求用户动手升级对端，
 * 后端原文是面向诊断的英文，必须映射到本地化文案（见 AGENTS.md 国际化章节）。
 * 其余错误维持"直接展示后端原文"的既有行为。
 */
export function webdavErrorMessage(error: unknown, t: TFunction, fallbackKey: string): string {
  if (isAppError(error)) {
    return error.code === "sync_algorithm_mismatch"
      ? t("settings.webdav.algorithmMismatch")
      : error.message;
  }
  return t(fallbackKey);
}
