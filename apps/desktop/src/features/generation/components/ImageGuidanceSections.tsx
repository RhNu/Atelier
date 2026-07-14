/* eslint-disable react-perf/jsx-no-new-function-as-prop, typescript/no-misused-promises */
import { ImagePlus, Trash2 } from "lucide-react";

import { AppButton } from "../../../components/ui";
import type { ResourceRefDto } from "../../../types";
import type { GenerationDraft } from "../model/generation-draft";
import {
  REFERENCE_TYPE_OPTIONS,
  isCharacterReferenceType,
  patchPreciseReference,
} from "./advanced-generation-model";
import { NumberField, SelectField } from "./GenerationFormFields";
import { GuidancePanelTitle } from "./GuidancePanelTitle";

export function ImageToImageSection({
  draft,
  onPatch,
  updateI2i,
  pickSourceImage,
  pickMaskImage,
  imageImportPending,
  releaseImages,
}: {
  draft: GenerationDraft;
  onPatch: (patch: Partial<GenerationDraft>) => void;
  updateI2i: (patch: Partial<NonNullable<GenerationDraft["i2i"]>>) => void;
  pickSourceImage: () => Promise<void>;
  pickMaskImage: () => Promise<void>;
  imageImportPending: boolean;
  releaseImages: (resources: ReadonlyArray<ResourceRefDto | null>) => Promise<void>;
}) {
  return (
    <section className="grid gap-2">
      <GuidancePanelTitle title="Image to image" resource={draft.i2i?.image ?? null} />
      <div className="flex flex-wrap gap-2">
        <AppButton variant="secondary" onClick={pickSourceImage} disabled={imageImportPending}>
          <ImagePlus aria-hidden="true" className="size-4" />
          {draft.i2i ? "Replace source" : "Add source"}
        </AppButton>
        <AppButton
          variant="ghost"
          onClick={pickMaskImage}
          disabled={!draft.i2i || imageImportPending}
        >
          <ImagePlus aria-hidden="true" className="size-4" />
          Mask
        </AppButton>
        <AppButton
          variant="ghost"
          onClick={() => {
            const resources = draft.i2i ? [draft.i2i.image, draft.i2i.mask] : [];
            onPatch({ i2i: null });
            void releaseImages(resources);
          }}
          disabled={!draft.i2i}
        >
          <Trash2 aria-hidden="true" className="size-4" />
          Clear
        </AppButton>
      </div>
      {draft.i2i ? (
        <div className="grid grid-cols-2 gap-2">
          <NumberField
            label="I2I strength"
            value={draft.i2i.strength}
            min={0.01}
            max={0.99}
            step={0.01}
            onChange={(strength) => updateI2i({ strength })}
          />
          <NumberField
            label="I2I noise"
            value={draft.i2i.noise}
            min={0}
            max={0.99}
            step={0.01}
            onChange={(noise) => updateI2i({ noise })}
          />
        </div>
      ) : null}
    </section>
  );
}

export function PreciseReferenceSection({
  draft,
  onPatch,
  pickPreciseReference,
  imageImportPending,
  releaseImages,
}: {
  draft: GenerationDraft;
  onPatch: (patch: Partial<GenerationDraft>) => void;
  pickPreciseReference: () => Promise<void>;
  imageImportPending: boolean;
  releaseImages: (resources: ReadonlyArray<ResourceRefDto | null>) => Promise<void>;
}) {
  return (
    <section className="grid gap-2">
      <GuidancePanelTitle
        title="Precise reference"
        resource={draft.preciseReferences[0]?.image ?? null}
      />
      <AppButton variant="secondary" onClick={pickPreciseReference} disabled={imageImportPending}>
        <ImagePlus aria-hidden="true" className="size-4" />
        Add reference
      </AppButton>
      {draft.preciseReferences.map((reference) => (
        <div key={reference.id} className="grid gap-2 border border-app-border bg-black/20 p-2">
          <div className="flex items-center justify-between gap-2">
            <span className="truncate text-xs text-app-muted">{reference.displayName}</span>
            <button
              type="button"
              aria-label={`Remove ${reference.displayName}`}
              className="text-app-muted hover:text-rose-100"
              onClick={() => {
                onPatch({
                  preciseReferences: draft.preciseReferences.filter(
                    (item) => item.id !== reference.id,
                  ),
                });
                void releaseImages([reference.image]);
              }}
            >
              <Trash2 aria-hidden="true" className="size-4" />
            </button>
          </div>
          <SelectField
            label="Reference type"
            value={reference.referenceType}
            options={REFERENCE_TYPE_OPTIONS}
            onChange={(value) => {
              if (isCharacterReferenceType(value)) {
                patchPreciseReference(draft, onPatch, reference.id, { referenceType: value });
              }
            }}
          />
          <div className="grid grid-cols-2 gap-2">
            <NumberField
              label="Fidelity"
              value={reference.fidelity}
              min={0}
              max={1}
              step={0.01}
              onChange={(fidelity) =>
                patchPreciseReference(draft, onPatch, reference.id, { fidelity })
              }
            />
            <NumberField
              label="Strength"
              value={reference.strength}
              min={0}
              max={1}
              step={0.01}
              onChange={(strength) =>
                patchPreciseReference(draft, onPatch, reference.id, { strength })
              }
            />
          </div>
        </div>
      ))}
    </section>
  );
}
