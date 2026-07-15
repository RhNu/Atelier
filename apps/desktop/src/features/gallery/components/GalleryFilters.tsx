import type { ChangeEvent } from "react";
import { useCallback, useMemo } from "react";
import { useTranslation } from "react-i18next";

import { AppButton, AppSelect } from "@/components/ui";

import type { SafetyFilter, SourceFilter } from "../gallery-utils";
import {
  artifactOptions,
  parseSafetyFilter,
  parseSourceFilter,
  safetyFilterOptions,
  sourceOptions,
} from "../gallery-utils";

type GalleryFiltersProps = {
  artifactKind: string;
  sourceFilter: SourceFilter;
  safetyFilter: SafetyFilter;
  total: number;
  offset: number;
  onArtifactChange: (value: string) => void;
  onSourceChange: (value: SourceFilter) => void;
  onSafetyChange: (value: SafetyFilter) => void;
  onResetPage: () => void;
};

export function GalleryFilters({
  artifactKind,
  sourceFilter,
  safetyFilter,
  total,
  offset,
  onArtifactChange,
  onSourceChange,
  onSafetyChange,
  onResetPage,
}: GalleryFiltersProps) {
  const { t } = useTranslation("gallery");
  const { t: translateCommon } = useTranslation("common");
  const localizedArtifacts = useMemo(
    () => artifactOptions.map((option) => ({ ...option, label: t(option.labelKey) })),
    [t],
  );
  const localizedSources = useMemo(
    () => sourceOptions.map((option) => ({ ...option, label: t(option.labelKey) })),
    [t],
  );
  const localizedSafety = useMemo(
    () => safetyFilterOptions.map((option) => ({ ...option, label: t(option.labelKey) })),
    [t],
  );
  const handleArtifactChange = useCallback(
    (event: ChangeEvent<HTMLSelectElement>) => onArtifactChange(event.target.value),
    [onArtifactChange],
  );
  const handleSourceChange = useCallback(
    (event: ChangeEvent<HTMLSelectElement>) =>
      onSourceChange(parseSourceFilter(event.target.value)),
    [onSourceChange],
  );
  const handleSafetyChange = useCallback(
    (event: ChangeEvent<HTMLSelectElement>) =>
      onSafetyChange(parseSafetyFilter(event.target.value)),
    [onSafetyChange],
  );

  return (
    <div className="grid gap-3 border-b border-app-border p-3 lg:grid-cols-[1fr_180px_180px_180px_auto]">
      <div className="min-w-0">
        <p className="text-sm font-semibold text-white">{t("indexedImages", { count: total })}</p>
        <p className="text-xs text-app-muted">{t("hiddenHint")}</p>
      </div>
      <label
        htmlFor="gallery-artifact-filter"
        className="grid gap-1 text-xs font-semibold text-app-muted"
      >
        {t("artifact")}
        <AppSelect
          id="gallery-artifact-filter"
          aria-label={t("artifactFilter")}
          options={localizedArtifacts}
          value={artifactKind}
          onChange={handleArtifactChange}
        />
      </label>
      <label
        htmlFor="gallery-source-filter"
        className="grid gap-1 text-xs font-semibold text-app-muted"
      >
        {t("source")}
        <AppSelect
          id="gallery-source-filter"
          aria-label={t("sourceFilter")}
          options={localizedSources}
          value={sourceFilter}
          onChange={handleSourceChange}
        />
      </label>
      <label
        htmlFor="gallery-safety-filter"
        className="grid gap-1 text-xs font-semibold text-app-muted"
      >
        {t("safety")}
        <AppSelect
          id="gallery-safety-filter"
          aria-label={t("safetyFilter")}
          options={localizedSafety}
          value={safetyFilter}
          onChange={handleSafetyChange}
        />
      </label>
      <AppButton variant="ghost" onClick={onResetPage} disabled={offset === 0}>
        {translateCommon("reset")}
      </AppButton>
    </div>
  );
}
