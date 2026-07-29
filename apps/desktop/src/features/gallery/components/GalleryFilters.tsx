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
    (value: string) => onArtifactChange(value),
    [onArtifactChange],
  );
  const handleSourceChange = useCallback(
    (value: string) => onSourceChange(parseSourceFilter(value)),
    [onSourceChange],
  );
  const handleSafetyChange = useCallback(
    (value: string) => onSafetyChange(parseSafetyFilter(value)),
    [onSafetyChange],
  );

  return (
    <div className="flex flex-wrap items-end justify-end gap-3 border-b border-app-border p-3">
      <label
        htmlFor="gallery-artifact-filter"
        className="grid w-[180px] gap-1 text-xs font-semibold text-app-muted"
      >
        {t("artifact")}
        <AppSelect
          id="gallery-artifact-filter"
          aria-label={t("artifactFilter")}
          options={localizedArtifacts}
          value={artifactKind}
          onValueChange={handleArtifactChange}
        />
      </label>
      <label
        htmlFor="gallery-source-filter"
        className="grid w-[180px] gap-1 text-xs font-semibold text-app-muted"
      >
        {t("source")}
        <AppSelect
          id="gallery-source-filter"
          aria-label={t("sourceFilter")}
          options={localizedSources}
          value={sourceFilter}
          onValueChange={handleSourceChange}
        />
      </label>
      <label
        htmlFor="gallery-safety-filter"
        className="grid w-[180px] gap-1 text-xs font-semibold text-app-muted"
      >
        {t("safety")}
        <AppSelect
          id="gallery-safety-filter"
          aria-label={t("safetyFilter")}
          options={localizedSafety}
          value={safetyFilter}
          onValueChange={handleSafetyChange}
        />
      </label>
      <AppButton variant="ghost" className="h-9" onClick={onResetPage} disabled={offset === 0}>
        {translateCommon("reset")}
      </AppButton>
    </div>
  );
}
