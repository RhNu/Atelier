import { useCallback, useEffect, useRef, useState } from "react";

import { resourceApi, uniqueImportedImageResources } from "../../../platform/atelier";
import type { ResourceRefDto, SaveResourceImagesZipEntryDto } from "../../../types";
import { setDirectorHandoffInput } from "../../director/state/director-handoff-store";
import { navigateToDirector } from "../../director/state/navigate-to-director";
import {
  useDeleteGenerationBatchesMutation,
  useDeleteRunHistoryMutation,
  useEnsureVibeEncodingFromResourceMutation,
  useExportVibeDocumentMutation,
  useGalleryImageReferenceMutation,
  useImportVibeDocumentsMutation,
  usePickImageResourcesMutation,
  useRerunGenerationBatchMutation,
  useRerunGenerationMutation,
  useReleaseImportedImagesMutation,
  useSaveResourceImageMutation,
  useSaveResourceImagesZipMutation,
} from "../data/useGenerationActions";
import { formatGenerationError } from "../generation-page-utils";
import { createGenerationRunIds, type GenerationDraft } from "../model/generation-draft";
import type {
  GenerationBatchView,
  GenerationRequestUnit,
  GenerationSampleSlot,
} from "../model/generation-preview-model";

type SetError = (error: string | null) => void;
type GenerationPageActionsOptions = {
  draft: GenerationDraft | null;
  batch: GenerationBatchView | null;
  selectedRequest: GenerationRequestUnit | null;
  selectedSample: GenerationSampleSlot | null;
  onBatchDeleted: () => void;
  onRequestDeleted: () => void;
};

export function useGenerationPageActions(options: GenerationPageActionsOptions) {
  const [error, setError] = useState<string | null>(null);
  const history = useGenerationHistoryActions({ ...options, setError });
  const inputs = useGenerationInputActions({ draft: options.draft, setError });
  return { error, ...history, ...inputs };
}

function useGenerationHistoryActions({
  batch,
  selectedRequest,
  selectedSample,
  onBatchDeleted,
  onRequestDeleted,
  setError,
}: Omit<GenerationPageActionsOptions, "draft"> & { setError: SetError }) {
  const rerunMutation = useRerunGenerationMutation();
  const rerunBatchMutation = useRerunGenerationBatchMutation();
  const deleteMutation = useDeleteRunHistoryMutation();
  const deleteBatchMutation = useDeleteGenerationBatchesMutation();
  const saveMutation = useSaveResourceImageMutation();
  const zipMutation = useSaveResourceImagesZipMutation();
  const referenceMutation = useGalleryImageReferenceMutation();

  const handleRerunRequest = useCallback(() => {
    if (!selectedRequest?.runId || rerunMutation.isPending) return;
    const ids = createGenerationRunIds(1);
    const jobId = ids.jobIds[0];
    if (!jobId) return;
    setError(null);
    void rerunMutation
      .mutateAsync({ run_id: selectedRequest.runId, batch_id: ids.batchId, job_id: jobId })
      .catch((cause: unknown) => setError(formatGenerationError(cause)));
  }, [rerunMutation, selectedRequest?.runId, setError]);

  const handleRerunBatch = useCallback(() => {
    if (!batch?.requests.length || rerunBatchMutation.isPending) return;
    const ids = createGenerationRunIds(batch.requests.length);
    setError(null);
    void rerunBatchMutation
      .mutateAsync({
        source_batch_id: batch.batchId,
        batch_id: ids.batchId,
        job_ids: ids.jobIds,
      })
      .catch((cause: unknown) => setError(formatGenerationError(cause)));
  }, [batch, rerunBatchMutation, setError]);

  const handleDeleteRequest = useCallback(() => {
    if (!selectedRequest?.runId || deleteMutation.isPending) return;
    setError(null);
    void deleteMutation
      .mutateAsync([selectedRequest.runId])
      .then(onRequestDeleted)
      .catch((cause: unknown) => setError(formatGenerationError(cause)));
  }, [deleteMutation, onRequestDeleted, selectedRequest?.runId, setError]);

  const handleDeleteBatch = useCallback(() => {
    if (!batch || deleteBatchMutation.isPending) return;
    setError(null);
    void deleteBatchMutation
      .mutateAsync([batch.batchId])
      .then(onBatchDeleted)
      .catch((cause: unknown) => setError(formatGenerationError(cause)));
  }, [batch, deleteBatchMutation, onBatchDeleted, setError]);

  const handleSaveSample = useCallback(() => {
    if (!selectedSample?.resource || !selectedRequest) return;
    setError(null);
    void saveMutation
      .mutateAsync({
        resource: selectedSample.resource,
        suggested_file_name: stableSampleName(selectedRequest, selectedSample),
      })
      .catch((cause: unknown) => setError(formatGenerationError(cause)));
  }, [saveMutation, selectedRequest, selectedSample, setError]);

  const handleSendSampleToDirector = useCallback(() => {
    if (!selectedSample?.resource || !selectedSample.galleryItemId) return;
    setError(null);
    void referenceMutation
      .mutateAsync({ item_id: selectedSample.galleryItemId, target: "director" })
      .then((reference) => handoffToDirector(reference.resource))
      .catch((cause: unknown) => setError(formatGenerationError(cause)));
  }, [referenceMutation, selectedSample, setError]);

  const handleExportRequest = useCallback(() => {
    if (!selectedRequest || zipMutation.isPending) return;
    exportZip(
      zipMutation,
      generationZipEntries([selectedRequest]),
      `request-${padIndex(selectedRequest.requestIndex)}`,
      setError,
    );
  }, [selectedRequest, setError, zipMutation]);

  const handleExportBatch = useCallback(() => {
    if (!batch || zipMutation.isPending) return;
    exportZip(zipMutation, generationZipEntries(batch.requests), batch.batchId, setError);
  }, [batch, setError, zipMutation]);

  return {
    handleDeleteBatch,
    handleDeleteRequest,
    handleExportBatch,
    handleExportRequest,
    handleRerunBatch,
    handleRerunRequest,
    handleSaveSample,
    handleSendSampleToDirector,
    deletePending: deleteMutation.isPending || deleteBatchMutation.isPending,
    exportPending: saveMutation.isPending,
    zipPending: zipMutation.isPending,
    handoffPending: referenceMutation.isPending,
    rerunPending: rerunMutation.isPending || rerunBatchMutation.isPending,
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

export function generationZipEntries(
  requests: ReadonlyArray<GenerationRequestUnit>,
): SaveResourceImagesZipEntryDto[] {
  return requests.flatMap((request) =>
    request.samples.flatMap((sample) =>
      sample.resource
        ? [{ resource: sample.resource, file_name: stableSampleName(request, sample) }]
        : [],
    ),
  );
}

function stableSampleName(
  request: Pick<GenerationRequestUnit, "requestIndex">,
  sample: Pick<GenerationSampleSlot, "sampleIndex">,
): string {
  return `request-${padIndex(request.requestIndex)}_sample-${padIndex(sample.sampleIndex)}`;
}

function padIndex(index: number): string {
  return String(index + 1).padStart(2, "0");
}

function exportZip(
  mutation: ReturnType<typeof useSaveResourceImagesZipMutation>,
  entries: SaveResourceImagesZipEntryDto[],
  suggestedName: string,
  setError: SetError,
): void {
  if (entries.length === 0) return;
  setError(null);
  void mutation
    .mutateAsync({ entries, suggested_file_name: suggestedName })
    .catch((cause: unknown) => setError(formatGenerationError(cause)));
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
