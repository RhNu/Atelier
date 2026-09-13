import type {
  ImageModelDto,
  PromptChunkDto,
  ResourceRefDto,
  UpsertPromptChunkRequestDto,
} from "@/types";

import { nullableText } from "./resource-model";

export type ChunkEditorDraft = {
  chunkId: string | null;
  folderId: string | null;
  key: string;
  displayName: string;
  aliases: string[];
  content: string;
  folderPath: string;
  description: string;
  preview: ResourceRefDto | null;
  models: ImageModelDto[];
};

export function blankChunkEditorDraft(
  model: ImageModelDto = "nai-diffusion-4-5-full",
  folderId: string | null = null,
  folderPath = "",
): ChunkEditorDraft {
  return {
    chunkId: null,
    folderId,
    key: "",
    displayName: "",
    aliases: [],
    content: "",
    folderPath,
    description: "",
    preview: null,
    models: [model],
  };
}

export function chunkToEditorDraft(chunk: PromptChunkDto): ChunkEditorDraft {
  return {
    chunkId: chunk.chunk_id,
    folderId: chunk.folder_id,
    key: chunk.identifier,
    displayName: chunk.display_name,
    aliases: [...chunk.aliases],
    content: chunk.content,
    folderPath: parentPath(chunk.path),
    description: chunk.description ?? "",
    preview: chunk.preview,
    models: [...chunk.models],
  };
}

export function editorDraftToChunkRequest(draft: ChunkEditorDraft): UpsertPromptChunkRequestDto {
  return {
    chunk_id: draft.chunkId,
    path: [draft.folderPath.trim(), draft.key.trim()].filter(Boolean).join("/"),
    folder_id: draft.folderId,
    display_name: draft.displayName.trim() || draft.key.trim(),
    aliases: draft.aliases,
    content: draft.content,
    description: nullableText(draft.description),
    preview: draft.preview,
    models: draft.models,
  };
}

function parentPath(path: string): string {
  return path.includes("/") ? path.slice(0, path.lastIndexOf("/")) : "";
}
