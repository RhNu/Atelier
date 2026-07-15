/* eslint-disable react-perf/jsx-no-jsx-as-prop, react-perf/jsx-no-new-function-as-prop */
import { Eye } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";

import { AppButton, AppModal } from "@/components/ui";
import type { CompiledPromptDto, PromptChunkDto } from "@/types";

import {
  useCompilePromptPreviewMutation,
  useDeletePromptChunkMutation,
  useUpsertPromptChunkMutation,
} from "../data/useResourcesData";
import {
  blankChunkDraft,
  chunkToDraft,
  formatError,
  matchesSearch,
  normalizeChunkDraft,
  nullableText,
  type ChunkDraft,
} from "../resource-model";
import {
  CompiledPreview,
  EditorActions,
  EditorPanel,
  PreviewSlot,
  ResourceList,
  ResourceListButton,
  TextArea,
  TextInput,
} from "./ResourceEditorPrimitives";

export function ChunkWorkspace({
  chunks,
  pending,
  error,
  search,
  newRequest,
}: {
  chunks: ReadonlyArray<PromptChunkDto>;
  pending: boolean;
  error: string | null;
  search: string;
  newRequest: number;
}) {
  const filtered = useMemo(
    () => chunks.filter((chunk) => matchesSearch(search, chunk.key, chunk.content, chunk.category)),
    [chunks, search],
  );
  const [draft, setDraft] = useState<ChunkDraft>(blankChunkDraft());
  const upsertMutation = useUpsertPromptChunkMutation();
  const deleteMutation = useDeletePromptChunkMutation();
  const compileMutation = useCompilePromptPreviewMutation();
  const [preview, setPreview] = useState<CompiledPromptDto | null>(null);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [editorOpen, setEditorOpen] = useState(false);
  const previousNewRequest = useRef(newRequest);

  useEffect(() => {
    if (newRequest === previousNewRequest.current) return;
    previousNewRequest.current = newRequest;
    setDraft(blankChunkDraft());
    setPreview(null);
    setErrorMessage(null);
    setEditorOpen(true);
  }, [newRequest]);

  function save() {
    setErrorMessage(null);
    void upsertMutation
      .mutateAsync(normalizeChunkDraft(draft))
      .then((saved) => {
        setDraft(chunkToDraft(saved));
        setEditorOpen(false);
      })
      .catch((err: unknown) => setErrorMessage(formatError(err)));
  }
  function remove() {
    if (!draft.chunk_id) {
      return;
    }
    setErrorMessage(null);
    void deleteMutation
      .mutateAsync({ chunk_id: draft.chunk_id })
      .then(() => {
        setDraft(blankChunkDraft());
        setEditorOpen(false);
      })
      .catch((err: unknown) => setErrorMessage(formatError(err)));
  }
  function compile() {
    setErrorMessage(null);
    void compileMutation
      .mutateAsync({ prompt: draft.content, max_depth: 8 })
      .then(setPreview)
      .catch((err: unknown) => setErrorMessage(formatError(err)));
  }

  return (
    <>
      <ResourceList pending={pending} error={error} emptyTitle="No prompt chunks">
        {filtered.map((chunk) => (
          <ResourceListButton
            key={chunk.chunk_id}
            selected={draft.chunk_id === chunk.chunk_id}
            title={chunk.key}
            detail={chunk.category ?? "Uncategorized"}
            preview={chunk.preview}
            onClick={() => {
              setDraft(chunkToDraft(chunk));
              setPreview(null);
              setErrorMessage(null);
              setEditorOpen(true);
            }}
          />
        ))}
      </ResourceList>
      <AppModal
        open={editorOpen}
        title={draft.chunk_id ? "Edit prompt chunk" : "New prompt chunk"}
        onClose={() => setEditorOpen(false)}
      >
        <EditorPanel
          title="Prompt Chunk"
          subtitle="Reusable @chunk(...) source"
          error={errorMessage}
          actions={
            <EditorActions
              canDelete={Boolean(draft.chunk_id)}
              saving={upsertMutation.isPending}
              deleting={deleteMutation.isPending}
              onNew={() => {
                setDraft(blankChunkDraft());
                setPreview(null);
              }}
              onSave={save}
              onDelete={remove}
            />
          }
        >
          <TextInput
            label="Key"
            value={draft.key}
            onChange={(key) => setDraft({ ...draft, key })}
          />
          <TextInput
            label="Category"
            value={draft.category ?? ""}
            onChange={(category) => setDraft({ ...draft, category: nullableText(category) })}
          />
          <TextInput
            label="Description"
            value={draft.description ?? ""}
            onChange={(description) =>
              setDraft({ ...draft, description: nullableText(description) })
            }
          />
          <TextArea
            label="Content"
            value={draft.content}
            minRows="min-h-40"
            onChange={(content) => setDraft({ ...draft, content })}
          />
          <PreviewSlot resource={draft.preview} label="Chunk preview" />
          <AppButton variant="secondary" onClick={compile} disabled={compileMutation.isPending}>
            <Eye aria-hidden="true" className="size-4" />
            Compile preview
          </AppButton>
          <CompiledPreview preview={preview} />
        </EditorPanel>
      </AppModal>
    </>
  );
}
