import type { PromptChunkDto, ResourceRefDto, UpsertPromptChunkRequestDto } from "@/types";

import { nullableText } from "./resource-model";

export type ChunkEditorDraft = {
  chunkId: string | null;
  key: string;
  content: string;
  category: string;
  description: string;
  preview: ResourceRefDto | null;
};

export function blankChunkEditorDraft(): ChunkEditorDraft {
  return {
    chunkId: null,
    key: "",
    content: "",
    category: "",
    description: "",
    preview: null,
  };
}

export function chunkToEditorDraft(chunk: PromptChunkDto): ChunkEditorDraft {
  return {
    chunkId: chunk.chunk_id,
    key: chunk.key,
    content: chunk.content,
    category: chunk.category ?? "",
    description: chunk.description ?? "",
    preview: chunk.preview,
  };
}

export function editorDraftToChunkRequest(draft: ChunkEditorDraft): UpsertPromptChunkRequestDto {
  return {
    chunk_id: draft.chunkId,
    key: draft.key.trim(),
    content: draft.content,
    category: nullableText(draft.category),
    description: nullableText(draft.description),
    preview: draft.preview,
  };
}
