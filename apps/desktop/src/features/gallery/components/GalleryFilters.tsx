import type { ChangeEvent } from "react";
import { useCallback } from "react";

import { AppButton, AppSelect } from "../../../components/ui";
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
        <p className="text-sm font-semibold text-white">{total} indexed images</p>
        <p className="text-xs text-app-muted">
          Hidden items are excluded unless the safety filter is set to Hidden.
        </p>
      </div>
      <label
        htmlFor="gallery-artifact-filter"
        className="grid gap-1 text-xs font-semibold text-app-muted"
      >
        Artifact
        <AppSelect
          id="gallery-artifact-filter"
          aria-label="Artifact filter"
          options={artifactOptions}
          value={artifactKind}
          onChange={handleArtifactChange}
        />
      </label>
      <label
        htmlFor="gallery-source-filter"
        className="grid gap-1 text-xs font-semibold text-app-muted"
      >
        Source
        <AppSelect
          id="gallery-source-filter"
          aria-label="Source filter"
          options={sourceOptions}
          value={sourceFilter}
          onChange={handleSourceChange}
        />
      </label>
      <label
        htmlFor="gallery-safety-filter"
        className="grid gap-1 text-xs font-semibold text-app-muted"
      >
        Safety
        <AppSelect
          id="gallery-safety-filter"
          aria-label="Safety filter"
          options={safetyFilterOptions}
          value={safetyFilter}
          onChange={handleSafetyChange}
        />
      </label>
      <AppButton variant="ghost" onClick={onResetPage} disabled={offset === 0}>
        Reset
      </AppButton>
    </div>
  );
}
