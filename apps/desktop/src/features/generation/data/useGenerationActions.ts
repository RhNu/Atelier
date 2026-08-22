/* eslint-disable max-lines */
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { runLoggedAction } from "@/app/logger";
import {
  desktopApi,
  galleryApi,
  generationApi,
  historyApi,
  promptApi,
  queryKeys,
  resourceApi,
  settingsApi,
  vibeApi,
} from "@/platform/atelier";
import type {
  CompileGenerationPromptRequestDto,
  GalleryImageReferenceRequestDto,
  ImageModelDto,
  ModelCapabilitiesDto,
  ImageResourceKindDto,
  GenerationEstimateRequestDto,
  ListPromptPresetsRequestDto,
  ListVibeDocumentsRequestDto,
  PromptPresetPageDto,
  ResourceRefDto,
  SubscriptionSummaryDto,
  RerunGenerationHistoryItemRequestDto,
  RerunGenerationHistoryBatchRequestDto,
  SaveResourceImageRequestDto,
  SaveResourceImagesZipRequestDto,
  SubmitGenerationBatchRequestDto,
} from "@/types";

import type { GenerationDraft } from "../model/generation-draft";
import {
  buildGenerationEstimateCacheKey,
  buildGenerationEstimateRequest,
  generationDraftToDto,
} from "../model/generation-draft";

type PickImageResourcesRequest = {
  kind: ImageResourceKindDto;
  extensions?: string[];
};

type EnsureVibeEncodingFromResourceRequest = {
  resource: ResourceRefDto;
  model: ImageModelDto;
  informationExtracted: number;
};

export type EnsuredVibeEncodingFromResource = {
  encoding: ResourceRefDto;
  sourceSha256: string;
  created: boolean;
};

export function useGenerationSettingsQuery() {
  return useQuery({
    queryKey: queryKeys.settings.workspace(),
    queryFn: () => settingsApi.get(),
  });
}

export function useGenerationDraftQuery() {
  return useQuery({
    queryKey: queryKeys.generation.draft(),
    queryFn: () => generationApi.getDraft(),
    retry: false,
  });
}

export function useSaveGenerationDraftMutation() {
  return useMutation({
    mutationFn: (draft: GenerationDraft) =>
      generationApi.saveDraft({ draft: generationDraftToDto(draft) }),
  });
}

export function useClearGenerationDraftMutation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: () => generationApi.clearDraft(),
    onSuccess: () => {
      queryClient.setQueryData(queryKeys.generation.draft(), null);
    },
  });
}

export function useGenerationEstimateQuery(
  draft: GenerationDraft | null,
  subscription: SubscriptionSummaryDto | null | undefined,
  capabilities?: ModelCapabilitiesDto,
) {
  const request = draft
    ? buildGenerationEstimateRequest(draft, { subscription, capabilities })
    : null;
  const estimateKey = draft
    ? buildGenerationEstimateCacheKey(draft, { subscription, capabilities })
    : null;

  return useQuery({
    queryKey: estimateKey
      ? queryKeys.generation.estimate(estimateKey)
      : queryKeys.generation.estimate(null),
    queryFn: () => {
      if (!request) {
        throw new Error("generation estimate request is required");
      }
      return generationApi.estimate(request satisfies GenerationEstimateRequestDto);
    },
    enabled: Boolean(request),
    retry: false,
  });
}

export function usePickImageResourcesMutation() {
  return useMutation({
    mutationFn: ({ kind, extensions = [] }: PickImageResourcesRequest) =>
      desktopApi.pickAndImportImageResources(kind, { extensions }),
  });
}

export function useEnsureVibeEncodingFromResourceMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({
      resource,
      model,
      informationExtracted,
    }: EnsureVibeEncodingFromResourceRequest): Promise<EnsuredVibeEncodingFromResource> =>
      runLoggedAction(
        "Ensure generation Vibe encoding",
        async () => {
          const image = await resourceApi.image({ resource });
          const sourceSha256 = await sha256Base64(image.image_base64);
          const ensured = await vibeApi.ensureEncoding({
            vibe_id: resource.id,
            source_sha256: sourceSha256,
            image: image.image_base64,
            model,
            information_extracted: informationExtracted,
          });
          return {
            encoding: ensured.resource,
            sourceSha256,
            created: ensured.created,
          };
        },
        { resourceId: resource.id },
      ),
    onSuccess: async (ensured) => {
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: queryKeys.vibe.root() }),
        ensured.created
          ? queryClient.invalidateQueries({ queryKey: queryKeys.account.activeSummary() })
          : Promise.resolve(),
      ]);
    },
  });
}

export function useSubmitGenerationMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (request: SubmitGenerationBatchRequestDto) => generationApi.submitBatch(request),
    onSuccess: async () => {
      await invalidateGenerationWorkbench(queryClient);
    },
  });
}

export function useRerunGenerationMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (request: RerunGenerationHistoryItemRequestDto) =>
      historyApi.rerunGeneration(request),
    onSuccess: async () => {
      await invalidateGenerationWorkbench(queryClient);
    },
  });
}

export function useDeleteRunHistoryMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (runIds: string[]) => historyApi.deleteItems({ run_ids: runIds }),
    onSuccess: async () => {
      await invalidateGenerationWorkbench(queryClient);
    },
  });
}

export function useRerunGenerationBatchMutation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (request: RerunGenerationHistoryBatchRequestDto) =>
      historyApi.rerunGenerationBatch(request),
    onSuccess: async () => invalidateGenerationWorkbench(queryClient),
  });
}

export function useDeleteGenerationBatchesMutation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (batchIds: string[]) => historyApi.deleteGenerationBatches({ batch_ids: batchIds }),
    onSuccess: async () => invalidateGenerationWorkbench(queryClient),
  });
}

export function useSaveResourceImageMutation() {
  return useMutation({
    mutationFn: (request: SaveResourceImageRequestDto) => desktopApi.saveResourceImage(request),
  });
}

export function useSaveResourceImagesZipMutation() {
  return useMutation({
    mutationFn: (request: SaveResourceImagesZipRequestDto) =>
      desktopApi.saveResourceImagesZip(request),
  });
}

export function useGalleryImageReferenceMutation() {
  return useMutation({
    mutationFn: (request: GalleryImageReferenceRequestDto) => galleryApi.imageReference(request),
  });
}

export function usePauseGenerationMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: () => generationApi.pause(),
    onSuccess: async () => {
      await invalidateGenerationWorkbench(queryClient);
    },
  });
}

export function useResumeGenerationMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: () => generationApi.resume(),
    onSuccess: async () => {
      await invalidateGenerationWorkbench(queryClient);
    },
  });
}

export function useStopGenerationMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: () => generationApi.stop(),
    onSuccess: async () => {
      await invalidateGenerationWorkbench(queryClient);
    },
  });
}

export function useCompilePromptMutation() {
  return useMutation({
    mutationFn: (request: CompileGenerationPromptRequestDto) =>
      promptApi.compileGenerationPreview(request),
  });
}

export function useResourceImageQuery(resource: ResourceRefDto | null) {
  return useQuery({
    queryKey: resource ? queryKeys.resource.image(resource) : ["resource", "image", null],
    queryFn: () => {
      if (!resource) {
        throw new Error("resource is required");
      }
      return resourceApi.image({ resource });
    },
    enabled: Boolean(resource),
  });
}

export function useReleaseImportedImagesMutation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (resources: ResourceRefDto[]) => resourceApi.releaseImportedImages({ resources }),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: queryKeys.resource.root() });
    },
  });
}

export function useVibeDocumentsQuery(query: ListVibeDocumentsRequestDto, enabled = true) {
  return useQuery({
    queryKey: queryKeys.vibe.list(query),
    queryFn: () => vibeApi.listDocuments(query),
    enabled,
  });
}

export function usePromptPresetsQuery(query: ListPromptPresetsRequestDto) {
  return useQuery({
    queryKey: queryKeys.prompt.presets(query),
    queryFn: () => promptApi.listPresets(query) satisfies Promise<PromptPresetPageDto>,
  });
}

export function useImportVibeDocumentsMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: () => desktopApi.pickAndImportVibeDocuments({ extensions: [] }),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: queryKeys.vibe.root() });
    },
  });
}

export function useExportVibeDocumentMutation() {
  return useMutation({
    mutationFn: (vibeIds: string[]) =>
      vibeApi.saveDocument({
        vibe_ids: vibeIds,
        format: vibeIds.length === 1 ? "naiv4vibe" : "naiv4vibebundle",
      }),
  });
}

async function invalidateGenerationWorkbench(queryClient: ReturnType<typeof useQueryClient>) {
  await runLoggedAction("Refresh generation workbench", () =>
    Promise.all([
      queryClient.invalidateQueries({ queryKey: queryKeys.generation.root() }),
      queryClient.invalidateQueries({ queryKey: queryKeys.history.root() }),
      queryClient.invalidateQueries({ queryKey: queryKeys.gallery.root() }),
      queryClient.invalidateQueries({ queryKey: queryKeys.resource.root() }),
    ]),
  );
}

async function sha256Base64(value: string): Promise<string> {
  const bytes = base64ToBytes(value);
  const subtle = globalThis.crypto?.subtle;
  if (!subtle) {
    return fallbackHash(bytes);
  }
  const input = new ArrayBuffer(bytes.byteLength);
  new Uint8Array(input).set(bytes);
  const digest = await subtle.digest("SHA-256", input);
  return bytesToHex(new Uint8Array(digest));
}

function base64ToBytes(value: string): Uint8Array {
  const binary = globalThis.atob(value);
  return Uint8Array.from(binary, (char) => char.charCodeAt(0));
}

function bytesToHex(bytes: Uint8Array): string {
  return [...bytes].map((byte) => byte.toString(16).padStart(2, "0")).join("");
}

function fallbackHash(bytes: Uint8Array): string {
  let hash = 0x811c9dc5;
  for (const byte of bytes) {
    hash ^= byte;
    hash = Math.imul(hash, 0x01000193);
  }
  return hash.toString(16).padStart(8, "0");
}
