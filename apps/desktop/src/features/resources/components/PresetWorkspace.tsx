/* eslint-disable max-lines-per-function, react-perf/jsx-no-jsx-as-prop, react-perf/jsx-no-new-function-as-prop */
import { Eye } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";

import { AppButton, AppModal } from "../../../components/ui";
import type { CompiledPromptDto, PromptPresetDto, PromptPresetKindDto } from "../../../types";
import {
  useCompilePromptPreviewMutation,
  useDeletePromptPresetMutation,
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
} from "../resource-model";
import {
  CheckboxField,
  CompiledPreview,
  EditorActions,
  EditorPanel,
  NumberInput,
  PreviewSlot,
  ResourceList,
  ResourceListButton,
  TextArea,
  TextInput,
} from "./ResourceEditorPrimitives";

export function PresetWorkspace({
  kind,
  presets,
  pending,
  error,
  search,
  newRequest,
}: {
  kind: PromptPresetKindDto;
  presets: ReadonlyArray<PromptPresetDto>;
  pending: boolean;
  error: string | null;
  search: string;
  newRequest: number;
}) {
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
    setErrorMessage(null);
    void deleteMutation
      .mutateAsync({ preset_id: draft.preset_id })
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
      <ResourceList pending={pending} error={error} emptyTitle="No prompt presets">
        {filtered.map((preset) => (
          <ResourceListButton
            key={preset.preset_id}
            selected={draft.preset_id === preset.preset_id}
            title={preset.name}
            detail={`${preset.enabled ? "Enabled" : "Disabled"} · ${preset.category ?? "Preset"}`}
            preview={preset.preview}
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
            ? `Edit ${mainPreset ? "main" : "character"} preset`
            : `New ${mainPreset ? "main" : "character"} preset`
        }
        onClose={() => setEditorOpen(false)}
      >
        <EditorPanel
          title={mainPreset ? "Main Preset" : "Character Preset"}
          subtitle={
            mainPreset ? "Applies global prompt and UC overrides" : "Applies one character prompt"
          }
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
              label="Name"
              value={draft.name}
              onChange={(name) => setDraft({ ...draft, name })}
            />
            <NumberInput
              label="Order"
              value={draft.order}
              onChange={(order) => setDraft({ ...draft, order })}
            />
          </div>
          <CheckboxField
            label="Enabled"
            checked={draft.enabled}
            onChange={(enabled) => setDraft({ ...draft, enabled })}
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
          <PresetFields draft={draft} setDraft={setDraft} mainPreset={mainPreset} />
          <PreviewSlot resource={draft.preview} label="Preset preview" />
          <AppButton variant="secondary" onClick={compile} disabled={compileMutation.isPending}>
            <Eye aria-hidden="true" className="size-4" />
            Compile preset fields
          </AppButton>
          <CompiledPreview preview={preview} />
        </EditorPanel>
      </AppModal>
    </>
  );
}

function PresetFields({
  draft,
  setDraft,
  mainPreset,
}: {
  draft: PresetDraft;
  setDraft: (draft: PresetDraft) => void;
  mainPreset: boolean;
}) {
  return (
    <>
      <TextArea
        label="Before"
        value={draft.before}
        minRows="min-h-24"
        onChange={(before) => setDraft({ ...draft, before })}
      />
      <TextArea
        label="After"
        value={draft.after}
        minRows="min-h-20"
        onChange={(after) => setDraft({ ...draft, after })}
      />
      <TextArea
        label="Replace"
        value={draft.replace}
        minRows="min-h-20"
        onChange={(replace) => setDraft({ ...draft, replace })}
      />
      <div className="grid grid-cols-3 gap-3">
        <TextArea
          label="UC before"
          value={draft.uc_before}
          minRows="min-h-20"
          onChange={(uc_before) => setDraft({ ...draft, uc_before })}
        />
        <TextArea
          label="UC after"
          value={draft.uc_after}
          minRows="min-h-20"
          onChange={(uc_after) => setDraft({ ...draft, uc_after })}
        />
        <TextArea
          label="UC replace"
          value={draft.uc_replace}
          minRows="min-h-20"
          onChange={(uc_replace) => setDraft({ ...draft, uc_replace })}
        />
      </div>
      {mainPreset ? (
        <div className="grid grid-cols-2 gap-3">
          <TextInput
            label="Quality override"
            value={draft.quality_override ?? ""}
            onChange={(quality_override) =>
              setDraft({ ...draft, quality_override: nullableText(quality_override) })
            }
          />
          <TextInput
            label="UC preset override"
            value={draft.uc_preset_override ?? ""}
            onChange={(uc_preset_override) =>
              setDraft({ ...draft, uc_preset_override: nullableText(uc_preset_override) })
            }
          />
        </div>
      ) : null}
    </>
  );
}
