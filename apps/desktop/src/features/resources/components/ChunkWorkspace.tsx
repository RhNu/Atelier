/* eslint-disable react-perf/jsx-no-new-function-as-prop */
import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

import type { PromptChunkDto } from "@/types";

import { matchesSearch, type ResourceViewMode } from "../resource-model";
import { ChunkEditorDialog } from "./ChunkEditorDialog";
import { ResourceList, ResourceListButton } from "./ResourceEditorPrimitives";

export function ChunkWorkspace({
  chunks,
  pending,
  error,
  search,
  newRequest,
  viewMode,
  categorySuggestions,
}: {
  chunks: ReadonlyArray<PromptChunkDto>;
  pending: boolean;
  error: string | null;
  search: string;
  newRequest: number;
  viewMode: ResourceViewMode;
  categorySuggestions: ReadonlyArray<string>;
}) {
  const { t } = useTranslation("resources");
  const [editorChunk, setEditorChunk] = useState<PromptChunkDto | null | undefined>(undefined);
  const previousNewRequest = useRef(newRequest);
  const filtered = useMemo(
    () => chunks.filter((chunk) => matchesSearch(search, chunk.key, chunk.content, chunk.category)),
    [chunks, search],
  );

  useEffect(() => {
    if (newRequest === previousNewRequest.current) return;
    previousNewRequest.current = newRequest;
    setEditorChunk(null);
  }, [newRequest]);

  return (
    <>
      <ResourceList
        pending={pending}
        error={error}
        emptyTitle={t("noPromptChunks")}
        viewMode={viewMode}
      >
        {filtered.map((chunk) => (
          <ResourceListButton
            key={chunk.chunk_id}
            selected={editorChunk?.chunk_id === chunk.chunk_id}
            title={chunk.key}
            detail={chunk.category ?? "Uncategorized"}
            description={chunk.description ?? chunk.content}
            preview={chunk.preview}
            viewMode={viewMode}
            onClick={() => setEditorChunk(chunk)}
          />
        ))}
      </ResourceList>
      {editorChunk !== undefined ? (
        <ChunkEditorDialog
          key={editorChunk?.chunk_id ?? `new-${newRequest}`}
          chunk={editorChunk}
          categorySuggestions={categorySuggestions}
          onClose={() => setEditorChunk(undefined)}
        />
      ) : null}
    </>
  );
}
