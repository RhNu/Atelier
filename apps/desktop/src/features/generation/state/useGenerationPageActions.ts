import { useCallback, useEffect, useRef, useState } from "react";

import { resourceApi, uniqueImportedImageResources } from "../../../platform/atelier";
import type { ResourceRefDto, RunHistoryItemDto, RunHistoryOutputDto } from "../../../types";
import { setDirectorHandoffInput } from "../../director/state/director-handoff-store";
import { navigateToDirector } from "../../director/state/navigate-to-director";
import {
  useDeleteRunHistoryMutation,
  useEnsureVibeEncodingFromResourceMutation,
  useExportVibeDocumentMutation,
  useGalleryImageReferenceMutation,
  useImportVibeDocumentsMutation,
  usePickImageResourcesMutation,
  useRerunGenerationMutation,
  useReleaseImportedImagesMutation,
  useSaveResourceImageMutation,
} from "../data/useGenerationActions";
import { formatGenerationError } from "../generation-page-utils";
import { createGenerationRunIds, type GenerationDraft } from "../model/generation-draft";
import type { GenerationPreview } from "./generation-event-store";

type SetError = (error: string | null) => void;
type GenerationPageActionsOptions = {
  draft: GenerationDraft | null;
  activePreview: GenerationPreview | null;
  selectedHistoryItem: RunHistoryItemDto | null;
  selectHistoryItem: (itemId: string | null) => void;
};

export function useGenerationPageActions(options: GenerationPageActionsOptions) {
  const [error, setError] = useState<string | null>(null);
  const history = useGenerationHistoryActions({ ...options, setError });
  const inputs = useGenerationInputActions({ draft: options.draft, setError });
  return { error, ...history, ...inputs };
}

function useGenerationHistoryActions({
  activePreview,
  selectedHistoryItem,
  selectHistoryItem,
  setError,
}: Omit<GenerationPageActionsOptions, "draft"> & { setError: SetError }) {
  const rerunMutation = useRerunGenerationMutation();
  const deleteMutation = useDeleteRunHistoryMutation();
  const saveMutation = useSaveResourceImageMutation();
  const referenceMutation = useGalleryImageReferenceMutation();

  const handleRerunSelected = useCallback(() => {
    if (!selectedHistoryItem || rerunMutation.isPending) return;
    const ids = createGenerationRunIds(1);
    const jobId = ids.jobIds[0];
    if (!jobId) return;
    setError(null);
    void rerunMutation
      .mutateAsync({ run_id: selectedHistoryItem.run_id, batch_id: ids.batchId, job_id: jobId })
      .catch((cause: unknown) => setError(formatGenerationError(cause)));
  }, [rerunMutation, selectedHistoryItem, setError]);

  const handleDeleteSelected = useCallback(() => {
    if (!selectedHistoryItem || deleteMutation.isPending) return;
    setError(null);
    void deleteMutation
      .mutateAsync([selectedHistoryItem.run_id])
      .then(() => selectHistoryItem(null))
      .catch((cause: unknown) => setError(formatGenerationError(cause)));
  }, [deleteMutation, selectHistoryItem, selectedHistoryItem, setError]);

  const exportOutput = useCallback(
    (output: RunHistoryOutputDto | null, suggestedName: string) => {
      if (!output || saveMutation.isPending) return;
      setError(null);
      void saveMutation
        .mutateAsync({ resource: output.resource, suggested_file_name: suggestedName })
        .catch((cause: unknown) => setError(formatGenerationError(cause)));
    },
    [saveMutation, setError],
  );
  const handleExportSelected = useCallback(() => {
    exportOutput(
      preferredHistoryOutput(selectedHistoryItem),
      selectedHistoryItem ? `${selectedHistoryItem.run_id}-sample` : "generation",
    );
  }, [exportOutput, selectedHistoryItem]);

  const sendOutputToDirector = useCallback(
    (output: RunHistoryOutputDto | null) => {
      if (!output?.item_id || referenceMutation.isPending) return;
      setError(null);
      void referenceMutation
        .mutateAsync({ item_id: output.item_id, target: "director" })
        .then((reference) => handoffToDirector(reference.resource))
        .catch((cause: unknown) => setError(formatGenerationError(cause)));
    },
    [referenceMutation, setError],
  );
  const handleSendSelectedToDirector = useCallback(() => {
    sendOutputToDirector(preferredHistoryOutput(selectedHistoryItem));
  }, [selectedHistoryItem, sendOutputToDirector]);

  const handleSavePreview = useCallback(() => {
    if (activePreview?.kind !== "resource") return;
    setError(null);
    void saveMutation
      .mutateAsync({
        resource: activePreview.resource,
        suggested_file_name: `${activePreview.jobId}-sample-${activePreview.sampleIndex}`,
      })
      .catch((cause: unknown) => setError(formatGenerationError(cause)));
  }, [activePreview, saveMutation, setError]);
  const handleSendPreviewToDirector = useCallback(() => {
    if (activePreview?.kind !== "resource" || !activePreview.galleryItemId) return;
    setError(null);
    void referenceMutation
      .mutateAsync({ item_id: activePreview.galleryItemId, target: "director" })
      .then((reference) => handoffToDirector(reference.resource))
      .catch((cause: unknown) => setError(formatGenerationError(cause)));
  }, [activePreview, referenceMutation, setError]);

  return {
    handleDeleteSelected,
    handleExportSelected,
    handleRerunSelected,
    handleSavePreview,
    handleSendPreviewToDirector,
    handleSendSelectedToDirector,
    deletePending: deleteMutation.isPending,
    exportPending: saveMutation.isPending,
    handoffPending: referenceMutation.isPending,
    rerunPending: rerunMutation.isPending,
  };
}

function useGenerationInputActions({
  draft,
  setError,
}: {
  draft: GenerationDraft | null;
  setError: SetError;
}) {
  const imageMutation = usePickImageResourcesMutation();
  const ensureVibeMutation = useEnsureVibeEncodingFromResourceMutation();
  const releaseImagesMutation = useReleaseImportedImagesMutation();
  const importVibeMutation = useImportVibeDocumentsMutation();
  const exportVibeMutation = useExportVibeDocumentMutation();
  const latestDraft = useRef(draft);
  latestDraft.current = draft;
  useEffect(
    () => () => {
      if (!latestDraft.current) return;
      const resources = uniqueImportedImageResources(
        generationDraftInputResources(latestDraft.current),
      );
      if (resources.length > 0) {
        void resourceApi.releaseImportedImages({ resources }).catch(() => undefined);
      }
    },
    [],
  );
  const handleImportVibeDocuments = useCallback(() => {
    setError(null);
    void importVibeMutation
      .mutateAsync()
      .catch((cause: unknown) => setError(formatGenerationError(cause)));
  }, [importVibeMutation, setError]);
  const handlePickImageResources = useCallback(
    async (kind: "source_image" | "reference_image") => {
      const imported = await imageMutation.mutateAsync({ kind, extensions: [] });
      return imported.map((item) => item.resource);
    },
    [imageMutation],
  );
  const handleReleaseImageResources = useCallback(
    async (resources: ReadonlyArray<ResourceRefDto | null>) => {
      const imported = uniqueImportedImageResources(resources);
      if (imported.length === 0) return;
      await releaseImagesMutation.mutateAsync(imported);
    },
    [releaseImagesMutation],
  );
  const handlePickVibeEncoding = useCallback(async () => {
    if (!draft) return null;
    const [imported, ...unused] = await imageMutation.mutateAsync({
      kind: "control_net_image",
      extensions: [],
    });
    await handleReleaseImageResources(unused.map((item) => item.resource));
    if (!imported) return null;
    try {
      return await ensureVibeMutation.mutateAsync({
        resource: imported.resource,
        model: draft.model,
        informationExtracted: 1,
      });
    } finally {
      await handleReleaseImageResources([imported.resource]);
    }
  }, [draft, ensureVibeMutation, handleReleaseImageResources, imageMutation]);
  const handleExportVibeDocument = useCallback(
    (vibeId: string) => {
      setError(null);
      void exportVibeMutation
        .mutateAsync([vibeId])
        .catch((cause: unknown) => setError(formatGenerationError(cause)));
    },
    [exportVibeMutation, setError],
  );
  return {
    handleExportVibeDocument,
    handleImportVibeDocuments,
    handlePickImageResources,
    handlePickVibeEncoding,
    handleReleaseImageResources,
    imageImportPending: imageMutation.isPending,
    vibeEnsurePending: ensureVibeMutation.isPending,
    vibeExportPending: exportVibeMutation.isPending,
    vibeImportPending: importVibeMutation.isPending,
  };
}

function preferredHistoryOutput(item: RunHistoryItemDto | null): RunHistoryOutputDto | null {
  if (!item) return null;
  return (
    item.outputs.find((output) => output.asset_role === "original") ??
    item.outputs.find((output) => output.asset_role === "primary") ??
    item.outputs[0] ??
    null
  );
}

function generationDraftInputResources(draft: GenerationDraft): ResourceRefDto[] {
  return [
    ...(draft.i2i ? [draft.i2i.image, draft.i2i.mask] : []),
    ...draft.preciseReferences.map((reference) => reference.image),
    ...draft.vibe.slots.map((slot) => slot.sourceImage),
  ].filter((resource): resource is ResourceRefDto => resource !== null);
}

function handoffToDirector(resource: Parameters<typeof setDirectorHandoffInput>[0]): void {
  setDirectorHandoffInput(resource);
  navigateToDirector();
}
