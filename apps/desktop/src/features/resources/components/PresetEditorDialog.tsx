/* eslint-disable react-perf/jsx-no-jsx-as-prop, react-perf/jsx-no-new-function-as-prop */
import { useState } from "react";
import { useTranslation } from "react-i18next";

import { AppModal } from "@/components/ui";
import type {
  CompiledPromptDto,
  ImageModelDto,
  PromptPresetDto,
  PromptPresetKindDto,
} from "@/types";

import {
  useCompilePromptPreviewMutation,
  useDeletePromptPresetMutation,
  useImportResourcePreviewMutation,
  useUpsertPromptPresetMutation,
} from "../data/useResourcesData";
import {
  blankPresetEditorDraft,
  editorDraftToUpsertRequest,
  presetPreviewSource,
  presetToEditorDraft,
} from "../preset-editor-model";
import { formatError } from "../resource-model";
import { ModelBindingField, PreviewModelField } from "./ModelBindingField";
import { PresetPromptFields } from "./PresetPromptFields";
import { EditorActions, EditorPanel, TextArea, TextInput } from "./ResourceEditorPrimitives";

type PresetEditorDialogProps = {
  kind: PromptPresetKindDto;
  preset: PromptPresetDto | null;
  onClose: () => void;
  defaultModel: ImageModelDto;
  initialFolderId?: string | null;
  initialFolderPath?: string;
};

export function PresetEditorDialog({
  kind,
  preset,
  onClose,
  defaultModel,
  initialFolderId = null,
  initialFolderPath = "",
}: PresetEditorDialogProps) {
  const { t } = useTranslation("resources");
  const [draft, setDraft] = useState(() =>
    preset
      ? presetToEditorDraft(preset)
      : blankPresetEditorDraft(kind, defaultModel, initialFolderId, initialFolderPath),
  );
  const [preview, setPreview] = useState<CompiledPromptDto | null>(null);
  const [previewModel, setPreviewModel] = useState(preset?.models[0] ?? defaultModel);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const upsertMutation = useUpsertPromptPresetMutation();
  const deleteMutation = useDeletePromptPresetMutation();
  const compileMutation = useCompilePromptPreviewMutation();
  const previewMutation = useImportResourcePreviewMutation();
  const mainPreset = kind === "main";

  function save() {
    setErrorMessage(null);
    void upsertMutation
      .mutateAsync(editorDraftToUpsertRequest(draft, kind))
      .then(onClose)
      .catch((error: unknown) => setErrorMessage(formatError(error)));
  }

  function remove() {
    if (!draft.presetId) {
      return;
    }
    setErrorMessage(null);
    void deleteMutation
      .mutateAsync({ preset_id: draft.presetId })
      .then(onClose)
      .catch((error: unknown) => setErrorMessage(formatError(error)));
  }

  function compile() {
    setErrorMessage(null);
    void compileMutation
      .mutateAsync({
        prompt: presetPreviewSource(draft),
        max_depth: 8,
        model: previewModel,
      })
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
      title={
        draft.presetId
          ? t("editPreset", { kind: t(mainPreset ? "mainPreset" : "characterPreset") })
          : t("newPreset", { kind: t(mainPreset ? "mainPreset" : "characterPreset") })
      }
      onClose={onClose}
    >
      <EditorPanel
        error={errorMessage}
        actions={
          <EditorActions
            canDelete={Boolean(draft.presetId)}
            saving={upsertMutation.isPending}
            deleting={deleteMutation.isPending}
            onSave={save}
            onDelete={remove}
          />
        }
      >
        <ModelBindingField models={draft.models} onChange={updateModels} />
        <PreviewModelField models={draft.models} value={previewModel} onChange={setPreviewModel} />
        <TextInput
          label={t("identifier")}
          value={draft.identifier}
          onChange={(identifier) => setDraft({ ...draft, identifier })}
        />
        <TextInput
          label={t("displayNameOptional")}
          value={draft.name}
          onChange={(name) => setDraft({ ...draft, name })}
        />
        <TextInput
          label={t("aliases")}
          value={draft.aliases.join(", ")}
          onChange={(aliases) =>
            setDraft({
              ...draft,
              aliases: aliases
                .split(",")
                .map((alias) => alias.trim())
                .filter(Boolean),
            })
          }
        />
        <ReadOnlyFolder path={draft.folderPath} label={t("folder")} />
        <TextArea
          label={t("description")}
          value={draft.description}
          minRows="min-h-24"
          onChange={(description) => setDraft({ ...draft, description })}
        />
        <PresetPromptFields
          draft={draft}
          previewModel={previewModel}
          preview={preview}
          compilePending={compileMutation.isPending}
          previewPending={previewMutation.isPending}
          previewError={previewMutation.isError ? formatError(previewMutation.error) : null}
          onChange={setDraft}
          onCompile={compile}
          onImportPreview={importPreview}
        />
      </EditorPanel>
    </AppModal>
  );
}

function ReadOnlyFolder({ path, label }: { path: string; label: string }) {
  return (
    <div className="grid gap-1 text-xs font-semibold text-app-muted uppercase">
      {label}
      <div className="flex h-9 items-center border border-app-border bg-black/10 px-3 text-sm font-normal text-app-text normal-case">
        {path || "/"}
      </div>
    </div>
  );
}
