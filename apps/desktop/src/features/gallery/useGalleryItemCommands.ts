import { useCallback, useState } from "react";

import type { GalleryItemDto } from "../../types";
import { setDirectorHandoffInput } from "../director/state/director-handoff-store";
import { navigateToDirector } from "../director/state/navigate-to-director";
import {
  useDeleteGalleryItemsMutation,
  useGalleryImageReferenceMutation,
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
