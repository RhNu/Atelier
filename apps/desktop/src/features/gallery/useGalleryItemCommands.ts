import { useCallback, useState } from "react";

import type { GalleryItemDto, ImageExportFormatDto } from "@/types";

import { setDirectorHandoffInput } from "../director/state/director-handoff-store";
import { navigateToDirector } from "../director/state/navigate-to-director";
import {
  useDeleteGalleryItemsMutation,
  useCopyGalleryImageMutation,
  useGalleryImageReferenceMutation,
  useRescanGallerySafetyMutation,
  useSaveGalleryImageMutation,
  useSetGallerySafetyOverrideMutation,
} from "./data/useGalleryPageQuery";
import {
  formatError,
  parseSafetyOverride,
  preferredExportAsset,
  suggestedGalleryExportFileName,
} from "./gallery-utils";

export function useGalleryItemCommands({
  selectedItem,
  visibleItems,
  overrideValue,
  onDeleteSuccess,
}: {
  selectedItem: GalleryItemDto | null;
  visibleItems: GalleryItemDto[];
  overrideValue: string;
  onDeleteSuccess: (deletedItemIds: string[]) => void;
}) {
  const [deleteTargetIds, setDeleteTargetIds] = useState<string[]>([]);
  const setSafetyOverrideMutation = useSetGallerySafetyOverrideMutation();
  const deleteGalleryItemsMutation = useDeleteGalleryItemsMutation();
  const saveImageMutation = useSaveGalleryImageMutation();
  const copyImageMutation = useCopyGalleryImageMutation();
  const imageReferenceMutation = useGalleryImageReferenceMutation();
  const rescanSafetyMutation = useRescanGallerySafetyMutation();
  const deleteTargets = resolveDeleteTargets(deleteTargetIds, visibleItems, selectedItem);
  const commandMutationError =
    setSafetyOverrideMutation.error ??
    copyImageMutation.error ??
    saveImageMutation.error ??
    imageReferenceMutation.error ??
    rescanSafetyMutation.error;
  const resetCommandErrors = useCallback(() => {
    setSafetyOverrideMutation.reset();
    copyImageMutation.reset();
    saveImageMutation.reset();
    deleteGalleryItemsMutation.reset();
    imageReferenceMutation.reset();
    rescanSafetyMutation.reset();
  }, [
    deleteGalleryItemsMutation,
    copyImageMutation,
    imageReferenceMutation,
    saveImageMutation,
    setSafetyOverrideMutation,
    rescanSafetyMutation,
  ]);
  const applyOverride = useSafetyOverride(
    selectedItem,
    overrideValue,
    resetCommandErrors,
    setSafetyOverrideMutation,
  );

  const rescanSafety = useSafetyRescan(selectedItem, resetCommandErrors, rescanSafetyMutation);

  const copySelected = useCallback(
    (format: ImageExportFormatDto) => {
      if (selectedItem) {
        resetCommandErrors();
        const asset = preferredExportAsset(selectedItem);
        copyImageMutation.mutate({
          resource: asset.resource,
          format,
        });
      }
    },
    [copyImageMutation, resetCommandErrors, selectedItem],
  );

  const exportSelected = useCallback(
    (format: ImageExportFormatDto) => {
      if (selectedItem) {
        resetCommandErrors();
        const asset = preferredExportAsset(selectedItem);
        saveImageMutation.mutate({
          resource: asset.resource,
          format,
          suggested_file_name: suggestedGalleryExportFileName(selectedItem.indexed_at_ms, format),
        });
      }
    },
    [resetCommandErrors, saveImageMutation, selectedItem],
  );

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
      setDeleteTargetIds([selectedItem.item_id]);
    }
  }, [resetCommandErrors, selectedItem]);

  const openBatchDeleteConfirmation = useCallback(
    (itemIds: string[]) => {
      if (itemIds.length > 0) {
        resetCommandErrors();
        setDeleteTargetIds(itemIds);
      }
    },
    [resetCommandErrors],
  );

  const closeDeleteConfirmation = useCallback(() => {
    if (!deleteGalleryItemsMutation.isPending) {
      setDeleteTargetIds([]);
    }
  }, [deleteGalleryItemsMutation.isPending]);

  const confirmDelete = useCallback(() => {
    if (deleteTargetIds.length === 0) {
      return;
    }

    resetCommandErrors();
    deleteGalleryItemsMutation.mutate(
      { item_ids: deleteTargetIds },
      {
        onSuccess: () => {
          const deletedItemIds = deleteTargetIds;
          setDeleteTargetIds([]);
          onDeleteSuccess(deletedItemIds);
        },
      },
    );
  }, [deleteGalleryItemsMutation, deleteTargetIds, onDeleteSuccess, resetCommandErrors]);

  return {
    applyingOverride: setSafetyOverrideMutation.isPending,
    rescanningSafety: rescanSafetyMutation.isPending,
    commandError: commandMutationError ? formatError(commandMutationError) : null,
    confirmDelete,
    closeDeleteConfirmation,
    copying: copyImageMutation.isPending,
    copySelected,
    deleteError: deleteGalleryItemsMutation.error
      ? formatError(deleteGalleryItemsMutation.error)
      : null,
    deleteTargetIds,
    deleteTargets,
    deleting: deleteGalleryItemsMutation.isPending,
    exportSelected,
    exporting: saveImageMutation.isPending,
    handoffPending: imageReferenceMutation.isPending,
    openDeleteConfirmation,
    openBatchDeleteConfirmation,
    applyOverride,
    rescanSafety,
    sendToDirector,
  };
}

function useSafetyRescan(
  selectedItem: GalleryItemDto | null,
  resetCommandErrors: () => void,
  mutation: ReturnType<typeof useRescanGallerySafetyMutation>,
) {
  return useCallback(() => {
    if (selectedItem) {
      resetCommandErrors();
      mutation.mutate({ item_ids: [selectedItem.item_id] });
    }
  }, [mutation, resetCommandErrors, selectedItem]);
}

function useSafetyOverride(
  selectedItem: GalleryItemDto | null,
  overrideValue: string,
  resetCommandErrors: () => void,
  mutation: ReturnType<typeof useSetGallerySafetyOverrideMutation>,
) {
  return useCallback(() => {
    if (selectedItem) {
      resetCommandErrors();
      mutation.mutate({
        item_id: selectedItem.item_id,
        manual_safety_override: parseSafetyOverride(overrideValue),
      });
    }
  }, [mutation, overrideValue, resetCommandErrors, selectedItem]);
}

function resolveDeleteTargets(
  targetIds: string[],
  visibleItems: GalleryItemDto[],
  selectedItem: GalleryItemDto | null,
) {
  return targetIds.flatMap((targetId) => {
    const target =
      visibleItems.find((item) => item.item_id === targetId) ??
      (targetId === selectedItem?.item_id ? selectedItem : null);
    return target ? [target] : [];
  });
}
