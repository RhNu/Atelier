import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import {
  desktopApi,
  promptApi,
  queryKeys,
  resourceApi,
  settingsApi,
  vibeApi,
} from "@/platform/atelier";
import type {
  CompilePromptRequestDto,
  DeletePromptChunkRequestDto,
  DeletePromptPresetRequestDto,
  ExportVibeDocumentRequestDto,
  ListPromptChunksRequestDto,
  ListPromptPresetsRequestDto,
  ListVibeDocumentsRequestDto,
  RenameVibeDocumentRequestDto,
  ResourceRefDto,
  SetVibeDocumentHiddenRequestDto,
  UpsertPromptChunkRequestDto,
  UpsertPromptPresetRequestDto,
  VibeModelDto,
} from "@/types";

type EnsureVibeEncodingFromSourceRequest = {
  vibeId: string;
  sourceImage: ResourceRefDto;
};

export function usePromptChunksQuery(request: ListPromptChunksRequestDto) {
  return useQuery({
    queryKey: queryKeys.prompt.chunks(request),
    queryFn: () => promptApi.listChunks(request),
  });
}

export function usePromptPresetsQuery(request: ListPromptPresetsRequestDto) {
  return useQuery({
    queryKey: queryKeys.prompt.presets(request),
    queryFn: () => promptApi.listPresets(request),
  });
}

export function useVibeDocumentsQuery(request: ListVibeDocumentsRequestDto) {
  return useQuery({
    queryKey: queryKeys.vibe.list(request),
    queryFn: () => vibeApi.listDocuments(request),
  });
}

export function useResourceImageQuery(resource: ResourceRefDto | null) {
  return useQuery({
    queryKey: resource
      ? queryKeys.resource.image(resource)
      : queryKeys.resource.image({ id: "", variant_id: null }),
    queryFn: () => {
      if (!resource) {
        throw new Error("resource is required");
      }
      return resourceApi.image({ resource });
    },
    enabled: Boolean(resource),
  });
}

export function useImportResourcePreviewMutation() {
  return useMutation({
    mutationFn: async (source: "clipboard" | "file") => {
      if (source === "clipboard") {
        return (await desktopApi.importClipboardImageResource("source_image")).resource;
      }
      const [selected, ...unused] = await desktopApi.pickAndImportImageResources("source_image", {
        extensions: [],
      });
      if (unused.length > 0) {
        await resourceApi.releaseImportedImages({
          resources: unused.map((item) => item.resource),
        });
      }
      return selected?.resource ?? null;
    },
  });
}

export function useReleaseResourcePreviewsMutation() {
  return useMutation({
    mutationFn: (resources: ResourceRefDto[]) => resourceApi.releaseImportedImages({ resources }),
  });
}

export function useUpsertPromptChunkMutation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (request: UpsertPromptChunkRequestDto) => promptApi.upsertChunk(request),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: queryKeys.prompt.root() });
    },
  });
}

export function useDeletePromptChunkMutation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (request: DeletePromptChunkRequestDto) => promptApi.deleteChunk(request),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: queryKeys.prompt.root() });
    },
  });
}

export function useUpsertPromptPresetMutation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (request: UpsertPromptPresetRequestDto) => promptApi.upsertPreset(request),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: queryKeys.prompt.root() });
    },
  });
}

export function useDeletePromptPresetMutation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (request: DeletePromptPresetRequestDto) => promptApi.deletePreset(request),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: queryKeys.prompt.root() });
    },
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

export function useImportEmbeddedPngVibeDocumentsMutation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: () => desktopApi.pickAndImportEmbeddedPngVibeDocuments({ extensions: [] }),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: queryKeys.vibe.root() });
    },
  });
}

export function useExportVibeDocumentMutation() {
  return useMutation({
    mutationFn: (request: ExportVibeDocumentRequestDto) => vibeApi.saveDocument(request),
  });
}

export function useRenameVibeDocumentMutation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (request: RenameVibeDocumentRequestDto) => vibeApi.renameDocument(request),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: queryKeys.vibe.root() });
    },
  });
}

export function useSetVibeDocumentHiddenMutation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (request: SetVibeDocumentHiddenRequestDto) => vibeApi.setDocumentHidden(request),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: queryKeys.vibe.root() });
    },
  });
}

export function useCompilePromptPreviewMutation() {
  return useMutation({
    mutationFn: (request: CompilePromptRequestDto) => promptApi.compilePreview(request),
  });
}

export function useEnsureVibeEncodingFromSourceMutation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async ({ vibeId, sourceImage }: EnsureVibeEncodingFromSourceRequest) => {
      const settings = await settingsApi.get();
      const image = await resourceApi.image({ resource: sourceImage });
      const sourceSha256 = await sha256Base64(image.image_base64);
      return vibeApi.ensureEncoding({
        vibe_id: vibeId,
        source_sha256: sourceSha256,
        image: image.image_base64,
        model: settings.generation.model as VibeModelDto,
        information_extracted: 1,
      });
    },
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: queryKeys.vibe.root() });
      await queryClient.invalidateQueries({ queryKey: queryKeys.resource.root() });
    },
  });
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
