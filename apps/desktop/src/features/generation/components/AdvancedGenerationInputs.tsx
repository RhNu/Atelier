/* eslint-disable max-lines-per-function, react-perf/jsx-no-new-function-as-prop */
import { useCallback, useState } from "react";
import { useTranslation } from "react-i18next";

import { describeError, frontendLogger } from "@/app/logger";
import type {
  CharacterReferenceTypeDto,
  ModelCapabilitiesDto,
  PromptTokenUsageDto,
  PromptPresetDto,
  ResourceRefDto,
} from "@/types";

import type { EnsuredVibeEncodingFromResource } from "../data/useGenerationActions";
import type { GenerationDraft } from "../model/generation-draft";
import type { GenerationDraftPatchOptions } from "../state/useGenerationDraft";
import { createLocalId } from "./advanced-generation-model";
import { CharacterGuidanceSection } from "./CharacterGuidanceSection";
import { ImageToImageSection, PreciseReferenceSection } from "./ImageGuidanceSections";
import { VibeGuidanceSection } from "./VibeGuidanceSection";

type AdvancedGenerationInputsProps = {
  draft: GenerationDraft;
  onPatch: (patch: Partial<GenerationDraft>, options?: GenerationDraftPatchOptions) => void;
  onFlush: () => void;
  characterPresets: ReadonlyArray<PromptPresetDto>;
  characterPresetsPending: boolean;
  vibeImportPending: boolean;
  vibeExportPending: boolean;
  imageImportPending: boolean;
  vibeEnsurePending: boolean;
  onPickImageResources: (kind: "source_image" | "reference_image") => Promise<ResourceRefDto[]>;
  onPickVibeEncoding: () => Promise<EnsuredVibeEncodingFromResource | null>;
  onReleaseImageResources: (resources: ReadonlyArray<ResourceRefDto | null>) => Promise<void>;
  onImportVibeDocuments: () => void;
  onExportVibeDocument: (vibeId: string) => void;
  developerMode: boolean;
  capabilities?: ModelCapabilitiesDto;
  tokenCounts: PromptTokenUsageDto | null;
  onOpenPositionEditor: () => void;
};

const DEFAULT_REFERENCE_TYPE: CharacterReferenceTypeDto = "character";

export function AdvancedGenerationInputs({
  draft,
  onPatch,
  onFlush,
  characterPresets,
  characterPresetsPending,
  vibeImportPending,
  vibeExportPending,
  imageImportPending,
  vibeEnsurePending,
  onPickImageResources,
  onPickVibeEncoding,
  onReleaseImageResources,
  onImportVibeDocuments,
  onExportVibeDocument,
  developerMode,
  capabilities,
  tokenCounts,
  onOpenPositionEditor,
}: AdvancedGenerationInputsProps) {
  const { t } = useTranslation("generation");
  const [error, setError] = useState<string | null>(null);
  const updateI2i = useCallback(
    (patch: Partial<NonNullable<GenerationDraft["i2i"]>>) => {
      if (!draft.i2i) {
        return;
      }
      onPatch({ i2i: { ...draft.i2i, ...patch } });
    },
    [draft.i2i, onPatch],
  );
  async function releaseImages(resources: ReadonlyArray<ResourceRefDto | null>) {
    try {
      await onReleaseImageResources(resources);
    } catch (err) {
      frontendLogger.error("Release generation image resources from input failed", {
        error: describeError(err),
      });
      setError(formatError(err));
    }
  }

  async function pickSourceImage() {
    setError(null);
    try {
      const [resource, ...unused] = await onPickImageResources("source_image");
      await releaseImages(unused);
      if (resource) {
        const replaced = draft.i2i?.image ?? null;
        onPatch(
          {
            i2i: {
              image: resource,
              mask: draft.i2i?.mask ?? null,
              strength: draft.i2i?.strength ?? 0.7,
              noise: draft.i2i?.noise ?? 0,
            },
          },
          { persist: "immediate" },
        );
        await releaseImages([replaced]);
      }
    } catch (err) {
      setError(formatError(err));
    }
  }

  async function pickMaskImage() {
    setError(null);
    try {
      const [resource, ...unused] = await onPickImageResources("source_image");
      await releaseImages(unused);
      if (resource && draft.i2i) {
        const replaced = draft.i2i.mask;
        updateI2i({ mask: resource });
        await releaseImages([replaced]);
      }
    } catch (err) {
      setError(formatError(err));
    }
  }

  async function pickPreciseReference() {
    setError(null);
    try {
      const resources = await onPickImageResources("reference_image");
      if (resources.length) {
        onPatch(
          {
            preciseReferences: [
              ...draft.preciseReferences,
              ...resources.map((resource, index) => ({
                id: createLocalId("ref"),
                image: resource,
                referenceType: DEFAULT_REFERENCE_TYPE,
                fidelity: 0.5,
                strength: 0.6,
                displayName: `Reference ${draft.preciseReferences.length + index + 1}`,
              })),
            ],
          },
          { persist: "immediate" },
        );
      }
    } catch (err) {
      setError(formatError(err));
    }
  }

  async function pickVibeEncoding() {
    setError(null);
    try {
      const ensured = await onPickVibeEncoding();
      if (ensured) {
        onPatch(
          {
            vibe: {
              ...draft.vibe,
              slots: [
                ...draft.vibe.slots,
                {
                  id: createLocalId("vibe"),
                  encoding: ensured.encoding,
                  vibeId: null,
                  informationExtracted: 1,
                  strength: 1,
                  displayName: "Uploaded Vibe",
                  sourceImage: null,
                  sourceSha256: ensured.sourceSha256,
                  model: draft.model,
                },
              ],
            },
            preciseReferences: [],
          },
          { persist: "immediate" },
        );
        await releaseImages(draft.preciseReferences.map((reference) => reference.image));
      }
    } catch (err) {
      setError(formatError(err));
    }
  }

  return (
    <section className="space-y-3 border-b border-app-border p-4">
      <div className="grid gap-1 text-sm text-app-text">
        {error ? <p className="text-rose-100">{error}</p> : null}
        <ImageToImageSection
          draft={draft}
          onPatch={onPatch}
          updateI2i={updateI2i}
          pickSourceImage={pickSourceImage}
          pickMaskImage={pickMaskImage}
          imageImportPending={imageImportPending}
          releaseImages={releaseImages}
          onFlush={onFlush}
          developerMode={developerMode}
        />
        {capabilities?.supports_vibe_transfer !== false && draft.preciseReferences.length === 0 ? (
          <VibeGuidanceSection
            draft={draft}
            onPatch={onPatch}
            onFlush={onFlush}
            vibeImportPending={vibeImportPending}
            vibeExportPending={vibeExportPending}
            vibeEnsurePending={vibeEnsurePending}
            pickVibeEncoding={pickVibeEncoding}
            onImportVibeDocuments={onImportVibeDocuments}
            onExportVibeDocument={onExportVibeDocument}
            releaseImages={releaseImages}
            developerMode={developerMode}
          />
        ) : null}
        {capabilities?.supports_character_reference !== false ? (
          <PreciseReferenceSection
            draft={draft}
            onPatch={onPatch}
            pickPreciseReference={pickPreciseReference}
            imageImportPending={imageImportPending}
            releaseImages={releaseImages}
            onFlush={onFlush}
            developerMode={developerMode}
          />
        ) : null}
        {(capabilities?.max_characters ?? 6) > 0 ? (
          <CharacterGuidanceSection
            draft={draft}
            onPatch={onPatch}
            characterPresets={characterPresets}
            characterPresetsPending={characterPresetsPending}
            tokenCounts={tokenCounts}
            capabilities={capabilities}
            onOpenPositionEditor={onOpenPositionEditor}
          />
        ) : null}
        {(!capabilities?.supports_vibe_transfer && draft.vibe.slots.length > 0) ||
        (!capabilities?.supports_character_reference && draft.preciseReferences.length > 0) ? (
          <p className="border border-app-border bg-black/20 p-2 text-xs text-app-muted">
            {t("dormantGuidanceSummary")}
          </p>
        ) : null}
      </div>
    </section>
  );
}

function formatError(error: unknown): string {
  return error instanceof Error ? error.message : "Command failed";
}
