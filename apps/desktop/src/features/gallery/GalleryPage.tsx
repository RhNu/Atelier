import { ImageIcon } from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";

import { AppButton, AppPanel, AppToolbar } from "../../components/ui";
import type { GalleryQueryDto } from "../../types";
import { GalleryDeleteConfirmation } from "./components/GalleryDeleteConfirmation";
import { GalleryFilters } from "./components/GalleryFilters";
import { GalleryGrid } from "./components/GalleryGrid";
import { GalleryInspector } from "./components/GalleryInspector";
import { useGalleryPageQuery, useGallerySettingsQuery } from "./data/useGalleryPageQuery";
import {
  PAGE_LIMIT,
  type SafetyFilter,
  type SourceFilter,
  matchesSafetyFilter,
} from "./gallery-utils";
import { useGalleryItemCommands } from "./useGalleryItemCommands";

export function GalleryPage() {
  const [offset, setOffset] = useState(0);
  const [artifactKind, setArtifactKind] = useState("all");
  const [sourceFilter, setSourceFilter] = useState<SourceFilter>("all");
  const [safetyFilter, setSafetyFilter] = useState<SafetyFilter>("all");
  const [selectedItemId, setSelectedItemId] = useState<string | null>(null);
  const [overrideValue, setOverrideValue] = useState("");

  const query = useMemo<GalleryQueryDto>(
    () => ({
      offset,
      limit: PAGE_LIMIT,
      artifact_kind: artifactKind === "all" ? null : artifactKind,
      source_kind: sourceFilter === "all" ? null : sourceFilter,
      manual_safety_override: null,
      safety_label: safetyFilter === "all" ? null : safetyFilter,
    }),
    [artifactKind, offset, safetyFilter, sourceFilter],
  );

  const galleryQuery = useGalleryPageQuery(query);
  const settingsQuery = useGallerySettingsQuery();
  const visibleItems = useMemo(
    () =>
      (galleryQuery.data?.items ?? []).filter((item) => matchesSafetyFilter(item, safetyFilter)),
    [galleryQuery.data?.items, safetyFilter],
  );

  const selectedItem = visibleItems.find((item) => item.item_id === selectedItemId) ?? null;
  const blurSensitive =
    settingsQuery.data?.frontend.gallery.blur_sensitive_images === true && !settingsQuery.isError;
  const total = galleryQuery.data?.total ?? 0;
  const canGoPrevious = offset > 0;
  const canGoNext = offset + PAGE_LIMIT < total;
  const clearSelection = useCallback(() => setSelectedItemId(null), []);
  const commands = useGalleryItemCommands({
    selectedItem,
    visibleItems,
    overrideValue,
    onDeleteSuccess: clearSelection,
  });

  useEffect(() => {
    setSelectedItemId((current) =>
      current && visibleItems.some((item) => item.item_id === current)
        ? current
        : (visibleItems[0]?.item_id ?? null),
    );
  }, [visibleItems]);

  useEffect(() => {
    setOverrideValue(selectedItem?.manual_safety_override ?? "");
  }, [selectedItem?.item_id, selectedItem?.manual_safety_override]);

  const resetPage = useCallback(() => setOffset(0), []);
  const changeArtifact = useCallback((value: string) => {
    setArtifactKind(value);
    setOffset(0);
  }, []);
  const changeSource = useCallback((value: SourceFilter) => {
    setSourceFilter(value);
    setOffset(0);
  }, []);
  const changeSafety = useCallback((value: SafetyFilter) => {
    setSafetyFilter(value);
    setOffset(0);
  }, []);
  const previousPage = useCallback(
    () => setOffset((current) => Math.max(0, current - PAGE_LIMIT)),
    [],
  );
  const nextPage = useCallback(() => setOffset((current) => current + PAGE_LIMIT), []);

  return (
    <div className="flex h-full min-h-0 flex-col">
      <AppToolbar>
        <div>
          <p className="text-xs font-semibold text-brand-200 uppercase">Gallery</p>
          <h1 className="text-lg font-semibold text-white">NovelAI Image Gallery</h1>
        </div>
        <div className="flex items-center gap-2 text-sm text-app-muted">
          <ImageIcon aria-hidden="true" className="size-4" />
          <span>{total} indexed</span>
        </div>
      </AppToolbar>

      <div className="grid min-h-0 flex-1 grid-cols-[minmax(0,1fr)_360px] divide-x divide-app-border">
        <AppPanel variant="section" className="flex min-h-0 flex-col overflow-hidden">
          <GalleryFilters
            artifactKind={artifactKind}
            sourceFilter={sourceFilter}
            safetyFilter={safetyFilter}
            offset={offset}
            onArtifactChange={changeArtifact}
            onSourceChange={changeSource}
            onSafetyChange={changeSafety}
            onResetPage={resetPage}
          />
          <div className="min-h-0 flex-1 overflow-auto p-3">
            <GalleryGrid
              isPending={galleryQuery.isPending}
              isError={galleryQuery.isError}
              error={galleryQuery.error}
              items={visibleItems}
              selectedItemId={selectedItemId}
              blurSensitive={blurSensitive}
              onSelect={setSelectedItemId}
            />
          </div>
          <footer className="flex items-center justify-between border-t border-app-border px-3 py-2 text-sm text-app-muted">
            <span>
              Page {Math.floor(offset / PAGE_LIMIT) + 1} of{" "}
              {Math.max(1, Math.ceil(total / PAGE_LIMIT))}
            </span>
            <div className="flex gap-2">
              <AppButton variant="secondary" onClick={previousPage} disabled={!canGoPrevious}>
                Previous
              </AppButton>
              <AppButton variant="secondary" onClick={nextPage} disabled={!canGoNext}>
                Next
              </AppButton>
            </div>
          </footer>
        </AppPanel>

        <GalleryInspector
          item={selectedItem}
          blurSensitive={blurSensitive}
          overrideValue={overrideValue}
          onOverrideChange={setOverrideValue}
          onApplyOverride={commands.applyOverride}
          onExport={commands.exportSelected}
          onSendToDirector={commands.sendToDirector}
          onDelete={commands.openDeleteConfirmation}
          applyingOverride={commands.applyingOverride}
          exporting={commands.exporting}
          deleting={commands.deleting}
          handoffPending={commands.handoffPending}
          commandError={commands.commandError}
        />
      </div>

      <GalleryDeleteConfirmation
        targetId={commands.deleteTargetId}
        target={commands.deleteTarget}
        deleting={commands.deleting}
        error={commands.deleteError}
        onClose={commands.closeDeleteConfirmation}
        onConfirm={commands.confirmDelete}
      />
    </div>
  );
}
