import i18n from "i18next";
import { initReactI18next } from "react-i18next";

import type { FrontendLanguageDto } from "@/types";

import { resources } from "./resources";

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
  await i18n.changeLanguage(language);
  if (typeof document !== "undefined") document.documentElement.lang = language;
}

if (!i18n.isInitialized) {
  void i18n.use(initReactI18next).init({
    resources,
    lng: resolveSystemLanguage(),
    fallbackLng: "en",
    supportedLngs: ["en", "zh-CN"],
    defaultNS: "common",
    ns: Object.keys(resources.en),
    interpolation: { escapeValue: false },
    returnNull: false,
  });
  if (typeof document !== "undefined") document.documentElement.lang = resolveSystemLanguage();
}

export { i18n };
