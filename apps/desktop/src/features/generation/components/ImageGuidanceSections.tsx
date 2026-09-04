/* eslint-disable max-lines-per-function, react-perf/jsx-no-jsx-as-prop, react-perf/jsx-no-new-function-as-prop, typescript/no-misused-promises */
import { ImagePlus, Pencil, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";

import { reportBackgroundPromise } from "@/app/logger";
import { AppIconButton, AppRangeField } from "@/components/ui";
import type { ResourceRefDto } from "@/types";

import type { GenerationDraft } from "../model/generation-draft";
import type { GenerationDraftPatchOptions } from "../state/useGenerationDraft";
import {
  REFERENCE_TYPE_OPTIONS,
  isCharacterReferenceType,
  patchPreciseReference,
} from "./advanced-generation-model";
import { SelectField } from "./GenerationFormFields";
import { GenerationResourceThumbnail } from "./GenerationResourceThumbnail";
import {
  GuidanceDeveloperMetadata,
  GuidanceSection,
  GuidanceSettingsDisclosure,
} from "./GuidanceSection";

export function ImageToImageSection({
  draft,
  onPatch,
  updateI2i,
  pickSourceImage,
  pickMaskImage,
  imageImportPending,
  releaseImages,
  onFlush,
  developerMode,
  onOpenInpaintEditor,
}: {
  draft: GenerationDraft;
  onPatch: (patch: Partial<GenerationDraft>, options?: GenerationDraftPatchOptions) => void;
  updateI2i: (patch: Partial<NonNullable<GenerationDraft["i2i"]>>) => void;
  pickSourceImage: () => Promise<void>;
  pickMaskImage: () => Promise<void>;
  imageImportPending: boolean;
  releaseImages: (resources: ReadonlyArray<ResourceRefDto | null>) => Promise<void>;
  onFlush: () => void;
  developerMode: boolean;
  onOpenInpaintEditor: () => void;
}) {
  const { t } = useTranslation("generation");
  const i2i = draft.i2i;

  return (
    <GuidanceSection
      title={t("imageToImage")}
      actions={
        i2i ? (
          <>
            <AppIconButton
              icon={Pencil}
              label={t("replaceI2iSource")}
              size="sm"
              onClick={pickSourceImage}
              disabled={imageImportPending}
            />
            {draft.model !== "nai-diffusion-5-curated" ? (
              <AppIconButton
                icon={Pencil}
                label={t("editInpaintMask")}
                size="sm"
                onClick={onOpenInpaintEditor}
                disabled={imageImportPending}
              />
            ) : null}
            <AppIconButton
              icon={ImagePlus}
              label={i2i.inpaint ? t("replaceI2iMask") : t("importI2iMask")}
              size="sm"
              onClick={pickMaskImage}
              disabled={imageImportPending || draft.model === "nai-diffusion-5-curated"}
            />
            {i2i.inpaint ? (
              <AppIconButton
                icon={Trash2}
                label={t("removeI2iMask")}
                size="sm"
                variant="danger"
                onClick={() => {
                  const mask = i2i.inpaint?.regionToReplace ?? null;
                  const insets = i2i.inpaint?.referenceInsets.map((inset) => inset.image) ?? [];
                  updateI2i({ inpaint: null });
                  reportBackgroundPromise(
                    releaseImages([mask, ...insets]),
                    "Release generation mask",
                  );
                }}
              />
            ) : null}
            <AppIconButton
              icon={Trash2}
              label={t("removeI2iSource")}
              size="sm"
              variant="danger"
              onClick={() => {
                onPatch({ i2i: null }, { persist: "immediate" });
                reportBackgroundPromise(
                  releaseImages([
                    i2i.image,
                    i2i.inpaint?.regionToReplace ?? null,
                    ...(i2i.inpaint?.referenceInsets.map((inset) => inset.image) ?? []),
                  ]),
                  "Release generation image guidance",
                );
              }}
            />
          </>
        ) : (
          <AppIconButton
            icon={ImagePlus}
            label={t("addI2iSource")}
            size="sm"
            onClick={pickSourceImage}
            disabled={imageImportPending}
          />
        )
      }
    >
      {i2i ? (
        <div className="grid gap-3">
          <div className="flex items-start gap-2">
            <GenerationResourceThumbnail
              resource={i2i.image}
              alt="I2I source"
              className="size-14"
            />
            {i2i.inpaint ? (
              <GenerationResourceThumbnail
                resource={i2i.inpaint.regionToReplace}
                alt="I2I mask"
                className="size-14"
              />
            ) : null}
          </div>
          <div className="grid grid-cols-2 gap-3">
            <AppRangeField
              label={t("strength")}
              value={i2i.strength}
              valueText={i2i.strength.toFixed(2)}
              min={0.01}
              max={i2i.inpaint ? 1 : 0.99}
              step={0.01}
              onChange={(strength) => updateI2i({ strength })}
              onCommit={onFlush}
            />
            <AppRangeField
              label={t("noise")}
              value={i2i.noise}
              valueText={i2i.noise.toFixed(2)}
              min={0}
              max={0.99}
              step={0.01}
              onChange={(noise) => updateI2i({ noise })}
              onCommit={onFlush}
            />
          </div>
          <GuidanceDeveloperMetadata enabled={developerMode} label="source" resource={i2i.image} />
          <GuidanceDeveloperMetadata
            enabled={developerMode}
            label="mask"
            resource={i2i.inpaint?.regionToReplace ?? null}
          />
          {draft.model === "nai-diffusion-5-curated" ? (
            <p className="text-xs text-app-muted">{t("v5CuratedInpaintUnavailable")}</p>
          ) : null}
        </div>
      ) : null}
    </GuidanceSection>
  );
}

export function PreciseReferenceSection({
  draft,
  onPatch,
  pickPreciseReference,
  imageImportPending,
  releaseImages,
  onFlush,
  developerMode,
}: {
  draft: GenerationDraft;
  onPatch: (patch: Partial<GenerationDraft>, options?: GenerationDraftPatchOptions) => void;
  pickPreciseReference: () => Promise<void>;
  imageImportPending: boolean;
  releaseImages: (resources: ReadonlyArray<ResourceRefDto | null>) => Promise<void>;
  onFlush: () => void;
  developerMode: boolean;
}) {
  const { t } = useTranslation("generation");
  return (
    <GuidanceSection
      title={t("preciseReference")}
      help="Precise Reference takes priority over Vibe Transfer. Existing Vibe slots stay saved and become active again when all precise references are removed."
      actions={
        <AppIconButton
          icon={ImagePlus}
          label={t("addPreciseReference")}
          size="sm"
          onClick={pickPreciseReference}
          disabled={imageImportPending}
        />
      }
    >
      {draft.preciseReferences.length > 0 ? (
        <div className="grid gap-2">
          {draft.preciseReferences.map((reference) => (
            <article
              key={reference.id}
              className="grid gap-2 border border-app-border bg-app-bg/70 p-2"
            >
              <div className="flex min-w-0 items-center gap-2">
                <GenerationResourceThumbnail
                  resource={reference.image}
                  alt={reference.displayName}
                  className="size-12"
                />
                <span className="min-w-0 flex-1 truncate text-xs font-semibold text-app-text">
                  {reference.displayName}
                </span>
                <AppIconButton
                  icon={Trash2}
                  label={`Remove ${reference.displayName}`}
                  size="sm"
                  variant="danger"
                  onClick={() => {
                    onPatch(
                      {
                        preciseReferences: draft.preciseReferences.filter(
                          (item) => item.id !== reference.id,
                        ),
                      },
                      { persist: "immediate" },
                    );
                    reportBackgroundPromise(
                      releaseImages([reference.image]),
                      "Release precise reference image",
                    );
                  }}
                />
              </div>
              <GuidanceSettingsDisclosure>
                <SelectField
                  label={t("referenceType")}
                  value={reference.referenceType}
                  options={REFERENCE_TYPE_OPTIONS}
                  onChange={(value) => {
                    if (isCharacterReferenceType(value)) {
                      patchPreciseReference(draft, onPatch, reference.id, {
                        referenceType: value,
                      });
                    }
                  }}
                />
                <div className="grid grid-cols-2 gap-3">
                  <AppRangeField
                    label={t("fidelity")}
                    value={reference.fidelity}
                    valueText={reference.fidelity.toFixed(1)}
                    min={0}
                    max={1}
                    step={0.1}
                    onChange={(fidelity) =>
                      patchPreciseReference(draft, onPatch, reference.id, { fidelity })
                    }
                    onCommit={onFlush}
                  />
                  <AppRangeField
                    label={t("strength")}
                    value={reference.strength}
                    valueText={reference.strength.toFixed(1)}
                    min={0}
                    max={1}
                    step={0.1}
                    onChange={(strength) =>
                      patchPreciseReference(draft, onPatch, reference.id, { strength })
                    }
                    onCommit={onFlush}
                  />
                </div>
                <GuidanceDeveloperMetadata enabled={developerMode} resource={reference.image} />
              </GuidanceSettingsDisclosure>
            </article>
          ))}
        </div>
      ) : null}
    </GuidanceSection>
  );
}
