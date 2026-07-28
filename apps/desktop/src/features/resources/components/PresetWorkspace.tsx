/* eslint-disable max-lines, max-lines-per-function, react-perf/jsx-no-jsx-as-prop, react-perf/jsx-no-new-array-as-prop, react-perf/jsx-no-new-function-as-prop */
import { Eye } from "lucide-react";
import { useEffect, useMemo, useRef, useState, type Dispatch, type SetStateAction } from "react";
import { useTranslation } from "react-i18next";

import { AppButton, AppModal, AppTabs } from "@/components/ui";
import { generationUcPresetOptions } from "@/features/generation/model/generation-options";
import type { CompiledPromptDto, PromptPresetDto, PromptPresetKindDto } from "@/types";

import {
  useCompilePromptPreviewMutation,
  useDeletePromptPresetMutation,
  useImportResourcePreviewMutation,
  useUpsertPromptPresetMutation,
} from "../data/useResourcesData";
import {
  blankPresetDraft,
  formatError,
  matchesSearch,
  normalizePresetDraft,
  nullableText,
  presetPreviewSource,
  presetToDraft,
  type PresetDraft,
  type ResourceViewMode,
} from "../resource-model";
import {
  CheckboxField,
  CompiledPreview,
  EditorActions,
  EditorPanel,
  NumberInput,
  PromptTextArea,
  ResourceList,
  ResourceListButton,
  SelectField,
  TextInput,
} from "./ResourceEditorPrimitives";
import { ResourcePreviewEditor } from "./ResourcePreviewEditor";

export function PresetWorkspace({
  kind,
  presets,
  pending,
  error,
  search,
  newRequest,
  viewMode,
}: {
  kind: PromptPresetKindDto;
  presets: ReadonlyArray<PromptPresetDto>;
  pending: boolean;
  error: string | null;
  search: string;
  newRequest: number;
  viewMode: ResourceViewMode;
}) {
  const { t } = useTranslation("resources");
  const filtered = useMemo(
    () =>
      presets.filter((preset) =>
        matchesSearch(search, preset.name, preset.category, preset.description, preset.before),
      ),
    [presets, search],
  );
  const [draft, setDraft] = useState<PresetDraft>(blankPresetDraft(kind));
  const upsertMutation = useUpsertPromptPresetMutation();
  const deleteMutation = useDeletePromptPresetMutation();
  const compileMutation = useCompilePromptPreviewMutation();
  const previewMutation = useImportResourcePreviewMutation();
  const [preview, setPreview] = useState<CompiledPromptDto | null>(null);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [editorOpen, setEditorOpen] = useState(false);
  const previousNewRequest = useRef(newRequest);
  const mainPreset = kind === "main";

  useEffect(() => {
    if (newRequest === previousNewRequest.current) return;
    previousNewRequest.current = newRequest;
    setDraft(blankPresetDraft(kind));
    setPreview(null);
    setErrorMessage(null);
    setEditorOpen(true);
  }, [kind, newRequest]);

  function save() {
    setErrorMessage(null);
    void upsertMutation
      .mutateAsync(normalizePresetDraft(draft, kind))
      .then((saved) => {
        setDraft(presetToDraft(saved));
        setEditorOpen(false);
      })
      .catch((err: unknown) => setErrorMessage(formatError(err)));
  }
  function remove() {
    if (!draft.preset_id) {
      return;
    }
    const presetId = draft.preset_id;
    setErrorMessage(null);
    void deleteMutation
      .mutateAsync({ preset_id: presetId })
      .then(() => {
        setDraft(blankPresetDraft(kind));
        setEditorOpen(false);
      })
      .catch((err: unknown) => setErrorMessage(formatError(err)));
  }
  function compile() {
    setErrorMessage(null);
    void compileMutation
      .mutateAsync({ prompt: presetPreviewSource(draft), max_depth: 8 })
      .then(setPreview)
      .catch((err: unknown) => setErrorMessage(formatError(err)));
  }

  return (
    <>
      <ResourceList
        pending={pending}
        error={error}
        emptyTitle={t("noPromptPresets")}
        viewMode={viewMode}
      >
        {filtered.map((preset) => (
          <ResourceListButton
            key={preset.preset_id}
            selected={draft.preset_id === preset.preset_id}
            title={preset.name}
            detail={`${preset.enabled ? t("enabled") : t("disabled")} · ${preset.category ?? t("preset")}`}
            description={
              preset.description ??
              preset.replace ??
              [preset.before, preset.after].filter(Boolean).join(" … ")
            }
            preview={preset.preview}
            viewMode={viewMode}
            onClick={() => {
              setDraft(presetToDraft(preset));
              setPreview(null);
              setErrorMessage(null);
              setEditorOpen(true);
            }}
          />
        ))}
      </ResourceList>
      <AppModal
        open={editorOpen}
        title={
          draft.preset_id
            ? t("editPreset", { kind: t(mainPreset ? "mainPreset" : "characterPreset") })
            : t("newPreset", { kind: t(mainPreset ? "mainPreset" : "characterPreset") })
        }
        onClose={() => setEditorOpen(false)}
      >
        <EditorPanel
          title={mainPreset ? t("mainPreset") : t("characterPreset")}
          error={errorMessage}
          actions={
            <EditorActions
              canDelete={Boolean(draft.preset_id)}
              saving={upsertMutation.isPending}
              deleting={deleteMutation.isPending}
              onNew={() => {
                setDraft(blankPresetDraft(kind));
                setPreview(null);
              }}
              onSave={save}
              onDelete={remove}
            />
          }
        >
          <div className="grid grid-cols-2 gap-3">
            <TextInput
              label={t("name")}
              value={draft.name}
              onChange={(name) => setDraft({ ...draft, name })}
            />
            <NumberInput
              label={t("order")}
              value={draft.order}
              onChange={(order) => setDraft({ ...draft, order })}
            />
          </div>
          <CheckboxField
            label={t("enabled")}
            checked={draft.enabled}
            onChange={(enabled) => setDraft({ ...draft, enabled })}
          />
          <p className="-mt-2 text-xs text-app-muted">{t("enabledDescription")}</p>
          <TextInput
            label={t("category")}
            value={draft.category ?? ""}
            onChange={(category) => setDraft({ ...draft, category: nullableText(category) })}
          />
          <TextInput
            label={t("description")}
            value={draft.description ?? ""}
            onChange={(description) =>
              setDraft({ ...draft, description: nullableText(description) })
            }
          />
          <PresetFields
            draft={draft}
            setDraft={setDraft}
            mainPreset={mainPreset}
            preview={preview}
            compilePending={compileMutation.isPending}
            previewPending={previewMutation.isPending}
            previewError={previewMutation.isError ? formatError(previewMutation.error) : null}
            onCompile={compile}
            onImportPreview={(source) =>
              void previewMutation
                .mutateAsync(source)
                .then(
                  (resource) =>
                    resource && setDraft((current) => ({ ...current, preview: resource })),
                )
                .catch((err: unknown) => setErrorMessage(formatError(err)))
            }
          />
        </EditorPanel>
      </AppModal>
    </>
  );
}

function PresetFields({
  draft,
  setDraft,
  mainPreset,
  preview,
  compilePending,
  previewPending,
  previewError,
  onCompile,
  onImportPreview,
}: {
  draft: PresetDraft;
  setDraft: Dispatch<SetStateAction<PresetDraft>>;
  mainPreset: boolean;
  preview: CompiledPromptDto | null;
  compilePending: boolean;
  previewPending: boolean;
  previewError: string | null;
  onCompile: () => void;
  onImportPreview: (source: "clipboard" | "file") => void;
}) {
  const { t } = useTranslation("resources");
  const [tab, setTab] = useState("prompt");
  const positiveMode = draft.replace.trim() ? "replace" : "surround";
  const ucMode = draft.uc_replace.trim() ? "replace" : "surround";
  return (
    <>
      <AppTabs
        value={tab}
        label={t("editorSections")}
        tabs={[
          { value: "prompt", label: t("promptTab") },
          { value: "uc", label: t("ucTab") },
          ...(mainPreset ? [{ value: "overrides", label: t("overridesTab") }] : []),
          { value: "preview", label: t("previewTab") },
        ]}
        onChange={setTab}
      />
      {tab === "prompt" ? (
        <>
          <SelectField
            label={t("promptBehavior")}
            value={positiveMode}
            options={[
              { value: "surround", label: t("beforeAfterMode") },
              { value: "replace", label: t("replaceMode") },
            ]}
            onChange={(mode) =>
              setDraft(
                mode === "replace"
                  ? { ...draft, before: "", after: "", replace: draft.replace || draft.before }
                  : { ...draft, replace: "" },
              )
            }
          />
          {positiveMode === "replace" ? (
            <PromptTextArea
              label={t("replace")}
              value={draft.replace}
              onChange={(replace) => setDraft({ ...draft, replace })}
            />
          ) : (
            <div className="grid grid-cols-2 gap-3">
              <PromptTextArea
                label={t("before")}
                value={draft.before}
                onChange={(before) => setDraft({ ...draft, before })}
              />
              <PromptTextArea
                label={t("after")}
                value={draft.after}
                onChange={(after) => setDraft({ ...draft, after })}
              />
            </div>
          )}
        </>
      ) : null}
      {tab === "uc" ? (
        <>
          <SelectField
            label={t("promptBehavior")}
            value={ucMode}
            options={[
              { value: "surround", label: t("beforeAfterMode") },
              { value: "replace", label: t("replaceMode") },
            ]}
            onChange={(mode) =>
              setDraft(
                mode === "replace"
                  ? {
                      ...draft,
                      uc_before: "",
                      uc_after: "",
                      uc_replace: draft.uc_replace || draft.uc_before,
                    }
                  : { ...draft, uc_replace: "" },
              )
            }
          />
          {ucMode === "replace" ? (
            <PromptTextArea
              label={t("ucReplace")}
              value={draft.uc_replace}
              onChange={(uc_replace) => setDraft({ ...draft, uc_replace })}
            />
          ) : (
            <div className="grid grid-cols-2 gap-3">
              <PromptTextArea
                label={t("ucBefore")}
                value={draft.uc_before}
                onChange={(uc_before) => setDraft({ ...draft, uc_before })}
              />
              <PromptTextArea
                label={t("ucAfter")}
                value={draft.uc_after}
                onChange={(uc_after) => setDraft({ ...draft, uc_after })}
              />
            </div>
          )}
        </>
      ) : null}
      {tab === "overrides" && mainPreset ? (
        <div className="grid grid-cols-2 gap-3">
          <SelectField
            label={t("qualityOverride")}
            value={draft.quality_override ?? ""}
            options={[
              { value: "", label: t("inherit") },
              { value: "true", label: t("enabled") },
              { value: "false", label: t("disabled") },
            ]}
            onChange={(value) => setDraft({ ...draft, quality_override: nullableText(value) })}
          />
          <SelectField
            label={t("ucPresetOverride")}
            value={draft.uc_preset_override ?? ""}
            options={[
              { value: "", label: t("inherit") },
              ...generationUcPresetOptions.map((value) => ({
                value,
                label: value.replaceAll("_", " "),
              })),
            ]}
            onChange={(value) => setDraft({ ...draft, uc_preset_override: nullableText(value) })}
          />
        </div>
      ) : null}
      {tab === "preview" ? (
        <>
          <ResourcePreviewEditor
            resource={draft.preview}
            label={t("presetPreview")}
            pending={previewPending}
            error={previewError}
            onImport={onImportPreview}
            onClear={() => setDraft({ ...draft, preview: null })}
          />
          <AppButton variant="secondary" onClick={onCompile} disabled={compilePending}>
            <Eye aria-hidden="true" className="size-4" />
            {t("compilePresetFields")}
          </AppButton>
          <CompiledPreview preview={preview} />
        </>
      ) : null}
    </>
  );
}
