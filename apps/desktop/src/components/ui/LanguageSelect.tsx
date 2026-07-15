import { useCallback, type ChangeEvent } from "react";
import { useTranslation } from "react-i18next";

import type { FrontendLanguageDto } from "@/types";

type LanguageSelectProps = {
  value: FrontendLanguageDto;
  disabled?: boolean;
  onChange: (language: FrontendLanguageDto) => void;
  compact?: boolean;
};

export function LanguageSelect({ value, disabled, onChange, compact }: LanguageSelectProps) {
  const { t } = useTranslation("common");
  const handleChange = useCallback(
    (event: ChangeEvent<HTMLSelectElement>) => {
      const language = parseFrontendLanguage(event.target.value);
      if (language) onChange(language);
    },
    [onChange],
  );

  return (
    <label className={compact ? "flex items-center gap-2" : "grid gap-1.5"}>
      <span className={compact ? "sr-only" : "text-xs font-semibold text-app-muted"}>
        {t("language")}
      </span>
      <select
        aria-label={t("language")}
        value={value}
        disabled={disabled}
        onChange={handleChange}
        className="h-8 border border-app-border bg-app-surface px-2 text-xs text-app-text outline-none focus:border-brand-400 disabled:opacity-50"
      >
        <option value="system">{t("languageSystem")}</option>
        <option value="en">{t("languageEnglish")}</option>
        <option value="zh-CN">{t("languageChinese")}</option>
      </select>
    </label>
  );
}

function parseFrontendLanguage(value: string): FrontendLanguageDto | null {
  switch (value) {
    case "system":
    case "en":
    case "zh-CN":
      return value;
    default:
      return null;
  }
}
