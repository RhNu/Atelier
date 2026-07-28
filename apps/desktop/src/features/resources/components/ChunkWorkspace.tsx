/* eslint-disable max-lines-per-function, react-perf/jsx-no-jsx-as-prop, react-perf/jsx-no-new-array-as-prop, react-perf/jsx-no-new-function-as-prop */
import { Eye } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

import { AppButton, AppModal, AppTabs } from "@/components/ui";
import type { CompiledPromptDto, PromptChunkDto } from "@/types";

import {
  useCompilePromptPreviewMutation,
  useDeletePromptChunkMutation,
  useImportResourcePreviewMutation,
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
  type ResourceViewMode,
} from "../resource-model";
import {
  CompiledPreview,
  EditorActions,
  EditorPanel,
  PromptTextArea,
  ResourceList,
  ResourceListButton,
  TextArea,
  TextInput,
} from "./ResourceEditorPrimitives";
import { ResourcePreviewEditor } from "./ResourcePreviewEditor";

export function ChunkWorkspace({
  chunks,
  pending,
  error,
  search,
  newRequest,
  viewMode,
}: {
  chunks: ReadonlyArray<PromptChunkDto>;
  pending: boolean;
  error: string | null;
  search: string;
  newRequest: number;
  viewMode: ResourceViewMode;
}) {
  const { t } = useTranslation("resources");
  const filtered = useMemo(
    () => chunks.filter((chunk) => matchesSearch(search, chunk.key, chunk.content, chunk.category)),
    [chunks, search],
  );
  const [draft, setDraft] = useState<ChunkDraft>(blankChunkDraft());
  const upsertMutation = useUpsertPromptChunkMutation();
  const deleteMutation = useDeletePromptChunkMutation();
  const compileMutation = useCompilePromptPreviewMutation();
  const previewMutation = useImportResourcePreviewMutation();
  const [preview, setPreview] = useState<CompiledPromptDto | null>(null);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [editorOpen, setEditorOpen] = useState(false);
  const [editorTab, setEditorTab] = useState("content");
  const previousNewRequest = useRef(newRequest);

  useEffect(() => {
    if (newRequest === previousNewRequest.current) return;
    previousNewRequest.current = newRequest;
    setDraft(blankChunkDraft());
    setPreview(null);
    setErrorMessage(null);
    setEditorOpen(true);
    setEditorTab("content");
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
    const chunkId = draft.chunk_id;
    setErrorMessage(null);
    void deleteMutation
      .mutateAsync({ chunk_id: chunkId })
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
      <ResourceList
        pending={pending}
        error={error}
        emptyTitle={t("noPromptChunks")}
        viewMode={viewMode}
      >
        {filtered.map((chunk) => (
          <ResourceListButton
            key={chunk.chunk_id}
            selected={draft.chunk_id === chunk.chunk_id}
            title={chunk.key}
            detail={chunk.category ?? "Uncategorized"}
            description={chunk.description ?? chunk.content}
            preview={chunk.preview}
            viewMode={viewMode}
            onClick={() => {
              setDraft(chunkToDraft(chunk));
              setPreview(null);
              setErrorMessage(null);
              setEditorOpen(true);
              setEditorTab("content");
            }}
          />
        ))}
      </ResourceList>
      <AppModal
        open={editorOpen}
        title={draft.chunk_id ? t("editPromptChunk") : t("newPromptChunk")}
        onClose={() => setEditorOpen(false)}
      >
        <EditorPanel
          title={t("promptChunk")}
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
          <AppTabs
            value={editorTab}
            label={t("editorSections")}
            tabs={[
              { value: "content", label: t("contentTab") },
              { value: "details", label: t("detailsTab") },
              { value: "preview", label: t("previewTab") },
            ]}
            onChange={setEditorTab}
          />
          {editorTab === "content" ? (
            <>
              <TextInput
                label={t("key")}
                value={draft.key}
                onChange={(key) => setDraft({ ...draft, key })}
              />
              <PromptTextArea
                label={t("content")}
                value={draft.content}
                minHeight={192}
                onChange={(content) => setDraft({ ...draft, content })}
              />
              <AppButton variant="secondary" onClick={compile} disabled={compileMutation.isPending}>
                <Eye aria-hidden="true" className="size-4" />
                {t("compilePreview")}
              </AppButton>
              <CompiledPreview preview={preview} />
            </>
          ) : null}
          {editorTab === "details" ? (
            <>
              <TextInput
                label={t("category")}
                value={draft.category ?? ""}
                onChange={(category) => setDraft({ ...draft, category: nullableText(category) })}
              />
              <TextArea
                label={t("description")}
                value={draft.description ?? ""}
                minRows="min-h-28"
                onChange={(description) =>
                  setDraft({ ...draft, description: nullableText(description) })
                }
              />
            </>
          ) : null}
          {editorTab === "preview" ? (
            <ResourcePreviewEditor
              resource={draft.preview}
              label={t("chunkPreview")}
              pending={previewMutation.isPending}
              error={previewMutation.isError ? formatError(previewMutation.error) : null}
              onImport={(source) =>
                void previewMutation
                  .mutateAsync(source)
                  .then(
                    (resource) =>
                      resource && setDraft((current) => ({ ...current, preview: resource })),
                  )
                  .catch((err: unknown) => setErrorMessage(formatError(err)))
              }
              onClear={() => setDraft({ ...draft, preview: null })}
            />
          ) : null}
        </EditorPanel>
      </AppModal>
    </>
  );
}
