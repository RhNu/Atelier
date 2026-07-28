import i18n from "i18next";
import { initReactI18next } from "react-i18next";

import type { FrontendLanguageDto } from "@/types";

import { describeError, frontendLogger, reportBackgroundPromise } from "../app/logger";
import { resources } from "./locales";

export type ResolvedLanguage = "en" | "zh-CN";

export function resolveSystemLanguage(languages?: readonly string[]): ResolvedLanguage {
  const candidates = languages ?? (typeof navigator === "undefined" ? [] : navigator.languages);
  return candidates.some((language) => language.toLowerCase().startsWith("zh")) ? "zh-CN" : "en";
}

export function resolveLanguagePreference(preference: FrontendLanguageDto): ResolvedLanguage {
  return preference === "system" ? resolveSystemLanguage() : preference;
}

export async function applyLanguagePreference(preference: FrontendLanguageDto): Promise<void> {
  const language = resolveLanguagePreference(preference);
  frontendLogger.debug("Applying language preference", { preference, language });
  try {
    await i18n.changeLanguage(language);
    if (typeof document !== "undefined") document.documentElement.lang = language;
    frontendLogger.info("Language preference applied", { language });
  } catch (error: unknown) {
    frontendLogger.error("Applying language preference failed", {
      language,
      error: describeError(error),
    });
    throw error;
  }
}

if (!i18n.isInitialized) {
  reportBackgroundPromise(
    i18n.use(initReactI18next).init({
      resources,
      lng: resolveSystemLanguage(),
      fallbackLng: "en",
      supportedLngs: ["en", "zh-CN"],
      defaultNS: "common",
      ns: Object.keys(resources.en),
      interpolation: { escapeValue: false },
      returnNull: false,
    }),
    "Initialize translations",
  );
  if (typeof document !== "undefined") document.documentElement.lang = resolveSystemLanguage();
}

export { i18n };
