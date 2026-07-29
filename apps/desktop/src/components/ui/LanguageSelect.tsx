import { useCallback, useMemo } from "react";
import { useTranslation } from "react-i18next";

import type { FrontendLanguageDto } from "@/types";

import { AppSelect } from "./AppSelect";

type LanguageSelectProps = {
  value: FrontendLanguageDto;
  disabled?: boolean;
  onChange: (language: FrontendLanguageDto) => void;
  compact?: boolean;
};

export function LanguageSelect({ value, disabled, onChange, compact }: LanguageSelectProps) {
  const { t } = useTranslation("common");
  const options = useMemo(
    () => [
      { value: "system", label: t("languageSystem") },
      { value: "en", label: t("languageEnglish") },
      { value: "zh-CN", label: t("languageChinese") },
    ],
    [t],
  );
  const handleChange = useCallback(
    (value: string) => {
      const language = parseFrontendLanguage(value);
      if (language) onChange(language);
    },
    [onChange],
  );

  return (
    <label className={compact ? "flex items-center gap-2" : "grid gap-1.5"}>
      <span className={compact ? "sr-only" : "text-xs font-semibold text-app-muted"}>
        {t("language")}
      </span>
      <AppSelect
        aria-label={t("language")}
        value={value}
        disabled={disabled}
        onValueChange={handleChange}
        className="h-8 px-2 pr-7 text-xs"
        options={options}
      />
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
