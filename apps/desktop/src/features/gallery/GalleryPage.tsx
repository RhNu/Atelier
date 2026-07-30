import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

import { AppButton, AppPanel } from "@/components/ui";
import type { GalleryQueryDto } from "@/types";

import { GalleryDeleteConfirmation } from "./components/GalleryDeleteConfirmation";
import { GalleryFilters } from "./components/GalleryFilters";
import { GalleryGrid } from "./components/GalleryGrid";
import { GalleryInspector } from "./components/GalleryInspector";
import {
  useGalleryItemDetailQuery,
  useGalleryPageQuery,
  useGallerySettingsQuery,
} from "./data/useGalleryPageQuery";
import {
  PAGE_LIMIT,
  type SafetyFilter,
  type SourceFilter,
  matchesSafetyFilter,
} from "./gallery-utils";
import { useGalleryItemCommands } from "./useGalleryItemCommands";
import { useGallerySelection } from "./useGallerySelection";

function GalleryPagination({
  offset,
  total,
  onPrevious,
  onNext,
}: {
  offset: number;
  total: number;
  onPrevious: () => void;
  onNext: () => void;
}) {
  const { t } = useTranslation("gallery");
  return (
    <footer className="flex items-center justify-between border-t border-app-border px-3 py-2 text-sm text-app-muted">
      <span>
        {t("pageOf", {
          page: Math.floor(offset / PAGE_LIMIT) + 1,
          total: Math.max(1, Math.ceil(total / PAGE_LIMIT)),
        })}
      </span>
      <div className="flex gap-2">
        <AppButton variant="secondary" onClick={onPrevious} disabled={offset === 0}>
          {t("previous")}
        </AppButton>
        <AppButton variant="secondary" onClick={onNext} disabled={offset + PAGE_LIMIT >= total}>
          {t("next")}
        </AppButton>
      </div>
    </footer>
  );
}

export function GalleryPage() {
  const [offset, setOffset] = useState(0);
  const [artifactKind, setArtifactKind] = useState("all");
  const [sourceFilter, setSourceFilter] = useState<SourceFilter>("all");
  const [safetyFilter, setSafetyFilter] = useState<SafetyFilter>("all");
  const [selectedItemId, setSelectedItemId] = useState<string | null>(null);
  const [overrideValue, setOverrideValue] = useState("");
  const query = useMemo<GalleryQueryDto>(
    () => buildGalleryQuery(offset, artifactKind, sourceFilter, safetyFilter),
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
  const selectedItemDetailQuery = useGalleryItemDetailQuery(selectedItem?.item_id ?? null);
  const blurSensitive =
    settingsQuery.data?.frontend.gallery.blur_sensitive_images === true && !settingsQuery.isError;
  const total = galleryQuery.data?.total ?? 0;
  const selection = useGallerySelection(visibleItems);
  const clearSelection = selection.clear;
  const handleDeleteSuccess = useCallback(
    (deletedItemIds: string[]) => {
      clearSelection();
      setSelectedItemId((current) =>
        current && deletedItemIds.includes(current) ? null : current,
      );
    },
    [clearSelection],
  );
  const commands = useGalleryItemCommands({
    selectedItem,
    visibleItems,
    overrideValue,
    onDeleteSuccess: handleDeleteSuccess,
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
  const openBatchDeleteConfirmation = commands.openBatchDeleteConfirmation;
  const deleteSelectedItems = useCallback(
    () => openBatchDeleteConfirmation(selection.selectedIds),
    [openBatchDeleteConfirmation, selection.selectedIds],
  );

  return (
    <div className="flex h-full min-h-0 flex-col">
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
              selectedItemIds={selection.selectedItemIds}
              blurSensitive={blurSensitive}
              onSelect={setSelectedItemId}
              onToggleSelection={selection.toggleItem}
              onToggleAll={selection.toggleAll}
              onDeleteSelected={deleteSelectedItems}
              deleting={commands.deleting}
            />
          </div>
          <GalleryPagination
            offset={offset}
            total={total}
            onPrevious={previousPage}
            onNext={nextPage}
          />
        </AppPanel>

        <GalleryInspector
          item={selectedItem}
          metadataJson={selectedItemDetailQuery.data?.embedded_metadata_json ?? null}
          items={visibleItems}
          blurSensitive={blurSensitive}
          overrideValue={overrideValue}
          onOverrideChange={setOverrideValue}
          onApplyOverride={commands.applyOverride}
          onRescanSafety={commands.rescanSafety}
          onCopy={commands.copySelected}
          onExport={commands.exportSelected}
          onSendToDirector={commands.sendToDirector}
          onDelete={commands.openDeleteConfirmation}
          applyingOverride={commands.applyingOverride}
          rescanningSafety={commands.rescanningSafety}
          copying={commands.copying}
          exporting={commands.exporting}
          deleting={commands.deleting}
          handoffPending={commands.handoffPending}
          commandError={commands.commandError}
          onSelectItem={setSelectedItemId}
        />
      </div>

      <GalleryDeleteConfirmation
        targetIds={commands.deleteTargetIds}
        targets={commands.deleteTargets}
        deleting={commands.deleting}
        error={commands.deleteError}
        onClose={commands.closeDeleteConfirmation}
        onConfirm={commands.confirmDelete}
      />
    </div>
  );
}

function buildGalleryQuery(
  offset: number,
  artifactKind: string,
  sourceFilter: SourceFilter,
  safetyFilter: SafetyFilter,
): GalleryQueryDto {
  return {
    offset,
    limit: PAGE_LIMIT,
    artifact_kind: artifactKind === "all" ? null : artifactKind,
    source_kind: sourceFilter === "all" ? null : sourceFilter,
    manual_safety_override: null,
    safety_label: safetyFilter === "all" ? null : safetyFilter,
  };
}
