/* eslint-disable react-perf/jsx-no-jsx-as-prop, react-perf/jsx-no-new-array-as-prop, react-perf/jsx-no-new-function-as-prop */
import { Eye } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";

import { AppButton, AppModal, AppTabs } from "@/components/ui";
import type { CompiledPromptDto, ImageModelDto, PromptChunkDto } from "@/types";

import {
  blankChunkEditorDraft,
  chunkToEditorDraft,
  editorDraftToChunkRequest,
} from "../chunk-editor-model";
import {
  useCompilePromptPreviewMutation,
  useDeletePromptChunkMutation,
  useImportResourcePreviewMutation,
  useUpsertPromptChunkMutation,
} from "../data/useResourcesData";
import { formatError } from "../resource-model";
import { ModelBindingField, PreviewModelField } from "./ModelBindingField";
import {
  CategoryInput,
  CompiledPreview,
  EditorActions,
  EditorPanel,
  PromptTextArea,
  TextArea,
  TextInput,
} from "./ResourceEditorPrimitives";
import { ResourcePreviewEditor } from "./ResourcePreviewEditor";

type ChunkEditorDialogProps = {
  chunk: PromptChunkDto | null;
  categorySuggestions: ReadonlyArray<string>;
  onClose: () => void;
  defaultModel: ImageModelDto;
};

export function ChunkEditorDialog({
  chunk,
  categorySuggestions,
  onClose,
  defaultModel,
}: ChunkEditorDialogProps) {
  const { t } = useTranslation("resources");
  const [draft, setDraft] = useState(() =>
    chunk ? chunkToEditorDraft(chunk) : blankChunkEditorDraft(defaultModel),
  );
  const [tab, setTab] = useState("content");
  const [previewModel, setPreviewModel] = useState(chunk?.models[0] ?? defaultModel);
  const [preview, setPreview] = useState<CompiledPromptDto | null>(null);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const upsertMutation = useUpsertPromptChunkMutation();
  const deleteMutation = useDeletePromptChunkMutation();
  const compileMutation = useCompilePromptPreviewMutation();
  const previewMutation = useImportResourcePreviewMutation();

  function save() {
    setErrorMessage(null);
    void upsertMutation
      .mutateAsync(editorDraftToChunkRequest(draft))
      .then(onClose)
      .catch((error: unknown) => setErrorMessage(formatError(error)));
  }

  function remove() {
    if (!draft.chunkId) {
      return;
    }
    setErrorMessage(null);
    void deleteMutation
      .mutateAsync({ chunk_id: draft.chunkId })
      .then(onClose)
      .catch((error: unknown) => setErrorMessage(formatError(error)));
  }

  function compile() {
    setErrorMessage(null);
    void compileMutation
      .mutateAsync({ prompt: draft.content, max_depth: 8, model: previewModel })
      .then(setPreview)
      .catch((error: unknown) => setErrorMessage(formatError(error)));
  }

  function updateModels(models: ImageModelDto[]) {
    setDraft({ ...draft, models });
    if (!models.includes(previewModel)) setPreviewModel(models[0] ?? defaultModel);
  }

  function importPreview(source: "clipboard" | "file") {
    void previewMutation
      .mutateAsync(source)
      .then((resource) => resource && setDraft((current) => ({ ...current, preview: resource })))
      .catch((error: unknown) => setErrorMessage(formatError(error)));
  }

  return (
    <AppModal
      open
      density="compact"
      title={draft.chunkId ? t("editPromptChunk") : t("newPromptChunk")}
      onClose={onClose}
    >
      <EditorPanel
        error={errorMessage}
        actions={
          <EditorActions
            canDelete={Boolean(draft.chunkId)}
            saving={upsertMutation.isPending}
            deleting={deleteMutation.isPending}
            onSave={save}
            onDelete={remove}
          />
        }
      >
        <AppTabs
          value={tab}
          label={t("editorSections")}
          tabs={[
            { value: "content", label: t("contentTab") },
            { value: "details", label: t("detailsTab") },
            { value: "preview", label: t("previewTab") },
          ]}
          onChange={setTab}
        />
        {tab === "content" ? (
          <>
            <ModelBindingField models={draft.models} onChange={updateModels} />
            <PreviewModelField
              models={draft.models}
              value={previewModel}
              onChange={setPreviewModel}
            />
            <TextInput
              label={t("key")}
              value={draft.key}
              onChange={(key) => setDraft({ ...draft, key })}
            />
            <PromptTextArea
              label={t("content")}
              value={draft.content}
              minHeight={192}
              model={previewModel}
              onChange={(content) => setDraft({ ...draft, content })}
            />
            <AppButton variant="secondary" onClick={compile} disabled={compileMutation.isPending}>
              <Eye aria-hidden="true" className="size-4" />
              {t("compilePreview")}
            </AppButton>
            <CompiledPreview preview={preview} />
          </>
        ) : null}
        {tab === "details" ? (
          <>
            <CategoryInput
              label={t("category")}
              value={draft.category}
              suggestions={categorySuggestions}
              onChange={(category) => setDraft({ ...draft, category })}
            />
            <TextArea
              label={t("description")}
              value={draft.description}
              minRows="min-h-28"
              onChange={(description) => setDraft({ ...draft, description })}
            />
          </>
        ) : null}
        {tab === "preview" ? (
          <ResourcePreviewEditor
            resource={draft.preview}
            label={t("chunkPreview")}
            pending={previewMutation.isPending}
            error={previewMutation.isError ? formatError(previewMutation.error) : null}
            onImport={importPreview}
            onClear={() => setDraft({ ...draft, preview: null })}
          />
        ) : null}
      </EditorPanel>
    </AppModal>
  );
}
