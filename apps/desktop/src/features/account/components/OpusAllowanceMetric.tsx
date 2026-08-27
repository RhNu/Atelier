import { useTranslation } from "react-i18next";

import type { V5UsageStatusDto } from "@/types";

import { formatOpusAllowance } from "./opus-allowance";

export function OpusAllowanceMetric({ usage }: { usage: V5UsageStatusDto }) {
  const { t } = useTranslation("generation");
  const formatted = formatOpusAllowance(usage, (key, options) => t(key, options));
  const valueClass = formatted.tone === "warning" ? "text-amber-200" : "text-app-text";
  return (
    <>
      <span className="text-app-muted"> · {t("opusAllowance")} </span>
      <strong className={valueClass}>{formatted.text}</strong>
    </>
  );
}
