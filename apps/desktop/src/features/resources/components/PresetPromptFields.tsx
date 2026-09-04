/* eslint-disable react-perf/jsx-no-new-array-as-prop, react-perf/jsx-no-new-function-as-prop, react-perf/jsx-no-new-object-as-prop */
import { Eye } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";

import { AppButton, AppTabs } from "@/components/ui";
import { useImageModelCatalog } from "@/features/generation/data/useImageModelCatalog";
import {
  generationUcPresetOptions,
  toQualityPreset,
} from "@/features/generation/model/generation-options";
import type { CompiledPromptDto, ImageModelDto } from "@/types";

import type { PresetEditorDraft, PromptBehaviorDraft } from "../preset-editor-model";
import { CompiledPreview, PromptTextArea, SelectField } from "./ResourceEditorPrimitives";
import { ResourcePreviewEditor } from "./ResourcePreviewEditor";

type PresetPromptFieldsProps = {
  draft: PresetEditorDraft;
  previewModel: ImageModelDto;
  preview: CompiledPromptDto | null;
  compilePending: boolean;
  previewPending: boolean;
  previewError: string | null;
  onChange: (draft: PresetEditorDraft) => void;
  onCompile: () => void;
  onImportPreview: (source: "clipboard" | "file") => void;
};

function toQualityOverride(value: string): PresetEditorDraft["qualityOverride"] {
  return value === "" ? "" : toQualityPreset(value);
}

export function PresetPromptFields({
  draft,
  previewModel,
  preview,
  compilePending,
  previewPending,
  previewError,
  onChange,
  onCompile,
  onImportPreview,
}: PresetPromptFieldsProps) {
  const { t } = useTranslation("resources");
  const [tab, setTab] = useState("prompt");
  const mainPreset = draft.kind === "main";
  const modelCatalog = useImageModelCatalog();
  const supportsLight = draft.models.every(
    (model) =>
      modelCatalog.data?.find((descriptor) => descriptor.model === model)?.capabilities
        ?.supports_light_quality_preset !== false,
  );

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
        <PromptBehaviorFields
          behavior={draft.prompt}
          model={previewModel}
          labels={{ before: t("before"), after: t("after"), replace: t("replace") }}
          onChange={(prompt) => onChange({ ...draft, prompt })}
        />
      ) : null}
      {tab === "uc" ? (
        <PromptBehaviorFields
          behavior={draft.uc}
          model={previewModel}
          labels={{ before: t("ucBefore"), after: t("ucAfter"), replace: t("ucReplace") }}
          onChange={(uc) => onChange({ ...draft, uc })}
        />
      ) : null}
      {tab === "overrides" && mainPreset ? (
        <div className="grid grid-cols-2 gap-3">
          <SelectField
            label={t("qualityOverride")}
            value={draft.qualityOverride}
            options={[
              { value: "", label: t("inherit") },
              { value: "standard", label: t("qualityOptions.standard") },
              ...(supportsLight ? [{ value: "light", label: t("qualityOptions.light") }] : []),
              { value: "none", label: t("qualityOptions.none") },
            ]}
            onChange={(qualityOverride) =>
              onChange({
                ...draft,
                qualityOverride: toQualityOverride(qualityOverride),
              })
            }
          />
          <SelectField
            label={t("ucPresetOverride")}
            value={draft.ucPresetOverride}
            options={[
              { value: "", label: t("inherit") },
              ...generationUcPresetOptions.map((value) => ({
                value,
                label: value.replaceAll("_", " "),
              })),
            ]}
            onChange={(ucPresetOverride) => onChange({ ...draft, ucPresetOverride })}
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
            onClear={() => onChange({ ...draft, preview: null })}
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

function PromptBehaviorFields({
  behavior,
  model,
  labels,
  onChange,
}: {
  behavior: PromptBehaviorDraft;
  model: ImageModelDto;
  labels: { before: string; after: string; replace: string };
  onChange: (behavior: PromptBehaviorDraft) => void;
}) {
  const { t } = useTranslation("resources");
  const mode = behavior.mode;

  return (
    <>
      <div className="grid gap-1">
        <span className="text-xs font-semibold text-app-muted">{t("promptBehavior")}</span>
        <AppTabs
          label={t("promptBehavior")}
          value={mode}
          tabs={[
            { value: "surround", label: t("beforeAfterMode") },
            { value: "replace", label: t("replaceMode") },
          ]}
          onChange={(value) =>
            onChange({ ...behavior, mode: value === "replace" ? "replace" : "surround" })
          }
        />
      </div>
      {mode === "replace" ? (
        <PromptTextArea
          label={labels.replace}
          value={behavior.replacement}
          model={model}
          onChange={(replacement) => onChange({ ...behavior, replacement })}
        />
      ) : (
        <div className="grid grid-cols-2 gap-3">
          <PromptTextArea
            label={labels.before}
            value={behavior.before}
            model={model}
            onChange={(before) => onChange({ ...behavior, before })}
          />
          <PromptTextArea
            label={labels.after}
            value={behavior.after}
            model={model}
            onChange={(after) => onChange({ ...behavior, after })}
          />
        </div>
      )}
    </>
  );
}
