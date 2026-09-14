/* eslint-disable react-perf/jsx-no-new-function-as-prop */
import { useEffect, useMemo, useRef, useState, type DragEvent, type ReactNode } from "react";
import { useTranslation } from "react-i18next";

import type { ImageModelDto, PromptChunkDto } from "@/types";

import { matchesSearch, type ResourceViewMode } from "../resource-model";
import { useSelectedPromptResource } from "../selected-prompt-resources";
import { ChunkEditorDialog } from "./ChunkEditorDialog";
import { ResourceList, ResourceListButton } from "./ResourceEditorPrimitives";

export function ChunkWorkspace({
  chunks,
  pending,
  error,
  search,
  newRequest,
  viewMode,
  defaultModel,
  onResourceDragStart,
  folderItems,
  currentFolderId = null,
  currentFolderPath = "",
}: {
  chunks: ReadonlyArray<PromptChunkDto>;
  pending: boolean;
  error: string | null;
  search: string;
  newRequest: number;
  viewMode: ResourceViewMode;
  defaultModel: ImageModelDto;
  onResourceDragStart?: (event: DragEvent, resourceId: string) => void;
  folderItems?: ReactNode;
  currentFolderId?: string | null;
  currentFolderPath?: string;
}) {
  const { t } = useTranslation("resources");
  const [editorChunk, setEditorChunk] = useState<PromptChunkDto | null | undefined>(undefined);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  useSelectedPromptResource(
    chunks.some((item) => item.chunk_id === selectedId) ? selectedId : null,
  );
  const previousNewRequest = useRef(newRequest);
  const filtered = useMemo(
    () =>
      chunks.filter((chunk) =>
        matchesSearch(search, chunk.path, chunk.display_name, chunk.content, ...chunk.aliases),
      ),
    [chunks, search],
  );

  useEffect(() => {
    if (newRequest === previousNewRequest.current) return;
    previousNewRequest.current = newRequest;
    setEditorChunk(null);
    setSelectedId(null);
  }, [newRequest]);

  return (
    <>
      <ResourceList
        pending={pending}
        error={error}
        emptyTitle={t("noPromptChunks")}
        folderItems={folderItems}
        viewMode={viewMode}
      >
        {filtered.map((chunk) => (
          <ResourceListButton
            key={chunk.chunk_id}
            selected={selectedId === chunk.chunk_id}
            title={chunk.display_name}
            detail={chunk.path}
            description={chunk.description ?? chunk.content}
            preview={chunk.preview}
            viewMode={viewMode}
            onDragStart={
              onResourceDragStart
                ? (event) => onResourceDragStart(event, chunk.chunk_id)
                : undefined
            }
            onClick={() => {
              setSelectedId(chunk.chunk_id);
              setEditorChunk(chunk);
            }}
          />
        ))}
      </ResourceList>
      {editorChunk !== undefined ? (
        <ChunkEditorDialog
          key={editorChunk?.chunk_id ?? `new-${newRequest}`}
          chunk={editorChunk}
          defaultModel={defaultModel}
          initialFolderId={currentFolderId}
          initialFolderPath={currentFolderPath}
          onClose={() => setEditorChunk(undefined)}
        />
      ) : null}
    </>
  );
}
