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
          onChange={handleArtifactChange}
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
          onChange={handleSourceChange}
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
          onChange={handleSafetyChange}
        />
      </label>
      <AppButton variant="ghost" className="h-9" onClick={onResetPage} disabled={offset === 0}>
        {translateCommon("reset")}
      </AppButton>
    </div>
  );
}
