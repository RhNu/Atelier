/* eslint-disable max-lines */
import { ImageIcon } from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";

import { AppButton, AppModal, AppPanel, AppToolbar } from "../../components/ui";
import type { GalleryItemDto, GalleryQueryDto } from "../../types";
import { setDirectorHandoffInput } from "../director/state/director-handoff-store";
import { navigateToDirector } from "../director/state/navigate-to-director";
import { GalleryFilters } from "./components/GalleryFilters";
import { GalleryGrid } from "./components/GalleryGrid";
import { GalleryInspector } from "./components/GalleryInspector";
import {
  useGalleryPageQuery,
  useGallerySettingsQuery,
  useDeleteGalleryItemsMutation,
  useGalleryImageReferenceMutation,
  useSaveGalleryImageMutation,
  useSetGallerySafetyOverrideMutation,
} from "./data/useGalleryPageQuery";
import {
  PAGE_LIMIT,
  type SafetyFilter,
  type SourceFilter,
  formatError,
  matchesSafetyFilter,
  parseSafetyOverride,
  preferredExportAsset,
  suggestedGalleryExportFileName,
} from "./gallery-utils";

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

      <div className="grid min-h-0 flex-1 grid-cols-[minmax(0,1fr)_360px] gap-3 p-3">
        <AppPanel className="flex min-h-0 flex-col overflow-hidden">
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

function useGalleryItemCommands({
  selectedItem,
  visibleItems,
  overrideValue,
  onDeleteSuccess,
}: {
  selectedItem: GalleryItemDto | null;
  visibleItems: GalleryItemDto[];
  overrideValue: string;
  onDeleteSuccess: () => void;
}) {
  const [deleteTargetId, setDeleteTargetId] = useState<string | null>(null);
  const setSafetyOverrideMutation = useSetGallerySafetyOverrideMutation();
  const deleteGalleryItemsMutation = useDeleteGalleryItemsMutation();
  const saveImageMutation = useSaveGalleryImageMutation();
  const imageReferenceMutation = useGalleryImageReferenceMutation();
  const deleteTarget =
    visibleItems.find((item) => item.item_id === deleteTargetId) ??
    (deleteTargetId === selectedItem?.item_id ? selectedItem : null);
  const commandMutationError =
    setSafetyOverrideMutation.error ?? saveImageMutation.error ?? imageReferenceMutation.error;

  const resetCommandErrors = useCallback(() => {
    setSafetyOverrideMutation.reset();
    saveImageMutation.reset();
    deleteGalleryItemsMutation.reset();
    imageReferenceMutation.reset();
  }, [
    deleteGalleryItemsMutation,
    imageReferenceMutation,
    saveImageMutation,
    setSafetyOverrideMutation,
  ]);

  const applyOverride = useCallback(() => {
    if (selectedItem) {
      resetCommandErrors();
      setSafetyOverrideMutation.mutate({
        item_id: selectedItem.item_id,
        manual_safety_override: parseSafetyOverride(overrideValue),
      });
    }
  }, [overrideValue, resetCommandErrors, selectedItem, setSafetyOverrideMutation]);

  const exportSelected = useCallback(() => {
    if (selectedItem) {
      resetCommandErrors();
      const asset = preferredExportAsset(selectedItem);
      saveImageMutation.mutate({
        resource: asset.resource,
        suggested_file_name: suggestedGalleryExportFileName(selectedItem.item_id, asset.role),
      });
    }
  }, [resetCommandErrors, saveImageMutation, selectedItem]);

  const sendToDirector = useCallback(() => {
    if (selectedItem) {
      resetCommandErrors();
      imageReferenceMutation.mutate(
        { item_id: selectedItem.item_id, target: "director" },
        {
          onSuccess: (reference) => {
            setDirectorHandoffInput(reference.resource);
            navigateToDirector();
          },
        },
      );
    }
  }, [imageReferenceMutation, resetCommandErrors, selectedItem]);

  const openDeleteConfirmation = useCallback(() => {
    if (selectedItem) {
      resetCommandErrors();
      setDeleteTargetId(selectedItem.item_id);
    }
  }, [resetCommandErrors, selectedItem]);

  const closeDeleteConfirmation = useCallback(() => {
    if (!deleteGalleryItemsMutation.isPending) {
      setDeleteTargetId(null);
    }
  }, [deleteGalleryItemsMutation.isPending]);

  const confirmDelete = useCallback(() => {
    if (!deleteTargetId) {
      return;
    }

    resetCommandErrors();
    deleteGalleryItemsMutation.mutate(
      { item_ids: [deleteTargetId] },
      {
        onSuccess: () => {
          setDeleteTargetId(null);
          onDeleteSuccess();
        },
      },
    );
  }, [deleteGalleryItemsMutation, deleteTargetId, onDeleteSuccess, resetCommandErrors]);

  return {
    applyingOverride: setSafetyOverrideMutation.isPending,
    commandError: commandMutationError ? formatError(commandMutationError) : null,
    confirmDelete,
    closeDeleteConfirmation,
    deleteError: deleteGalleryItemsMutation.error
      ? formatError(deleteGalleryItemsMutation.error)
      : null,
    deleteTarget,
    deleteTargetId,
    deleting: deleteGalleryItemsMutation.isPending,
    exportSelected,
    exporting: saveImageMutation.isPending,
    handoffPending: imageReferenceMutation.isPending,
    openDeleteConfirmation,
    applyOverride,
    sendToDirector,
  };
}

function GalleryDeleteConfirmation({
  targetId,
  target,
  deleting,
  error,
  onClose,
  onConfirm,
}: {
  targetId: string | null;
  target: GalleryItemDto | null;
  deleting: boolean;
  error: string | null;
  onClose: () => void;
  onConfirm: () => void;
}) {
  return (
    <AppModal open={Boolean(targetId)} title="Delete gallery item" onClose={onClose}>
      <div className="grid gap-4">
        <div className="grid gap-2 text-sm text-app-text">
          <p>
            Delete <span className="font-semibold text-white">{targetId}</span> permanently from
            Gallery.
          </p>
          <p className="text-app-muted">
            This also removes linked output metadata and may delete unreferenced workspace image
            files from disk.
          </p>
          {target ? <p className="text-xs text-app-muted">Artifact: {target.artifact_id}</p> : null}
        </div>
        {error ? (
          <p className="border border-rose-500/50 bg-rose-950/40 px-3 py-2 text-sm text-rose-100">
            {error}
          </p>
        ) : null}
        <div className="flex justify-end gap-2">
          <AppButton variant="secondary" onClick={onClose} disabled={deleting}>
            Cancel
          </AppButton>
          <AppButton variant="danger" onClick={onConfirm} disabled={deleting}>
            Delete permanently
          </AppButton>
        </div>
      </div>
    </AppModal>
  );
}
