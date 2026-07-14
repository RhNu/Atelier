/* eslint-disable max-lines-per-function, react-perf/jsx-no-new-function-as-prop */
import { useCallback, useMemo, useState } from "react";

import { AppPanel } from "../../../components/ui";
import type {
  CharacterReferenceTypeDto,
  PromptPresetDto,
  ResourceRefDto,
  VibeDocumentEntryDto,
} from "../../../types";
import type { EnsuredVibeEncodingFromResource } from "../data/useGenerationActions";
import type { GenerationDraft } from "../model/generation-draft";
import { createLocalId } from "./advanced-generation-model";
import { CharacterGuidanceSection } from "./CharacterGuidanceSection";
import { ImageToImageSection, PreciseReferenceSection } from "./ImageGuidanceSections";
import { VibeGuidanceSection } from "./VibeGuidanceSection";

type AdvancedGenerationInputsProps = {
  draft: GenerationDraft;
  onPatch: (patch: Partial<GenerationDraft>) => void;
  characterPresets: ReadonlyArray<PromptPresetDto>;
  vibeDocuments: ReadonlyArray<VibeDocumentEntryDto>;
  vibePending: boolean;
  vibeError: string | null;
  vibeImportPending: boolean;
  vibeExportPending: boolean;
  imageImportPending: boolean;
  vibeEnsurePending: boolean;
  onPickImageResources: (kind: "source_image" | "reference_image") => Promise<ResourceRefDto[]>;
  onPickVibeEncoding: () => Promise<EnsuredVibeEncodingFromResource | null>;
  onImportVibeDocuments: () => void;
  onExportVibeDocument: (vibeId: string) => void;
};

const DEFAULT_REFERENCE_TYPE: CharacterReferenceTypeDto = "character";

export function AdvancedGenerationInputs({
  draft,
  onPatch,
  characterPresets,
  vibeDocuments,
  vibePending,
  vibeError,
  vibeImportPending,
  vibeExportPending,
  imageImportPending,
  vibeEnsurePending,
  onPickImageResources,
  onPickVibeEncoding,
  onImportVibeDocuments,
  onExportVibeDocument,
}: AdvancedGenerationInputsProps) {
  const [error, setError] = useState<string | null>(null);
  const characterPresetOptions = useMemo(
    () => [
      { value: "", label: "No character preset" },
      ...characterPresets
        .filter((preset) => preset.enabled)
        .map((preset) => ({ value: preset.preset_id, label: preset.name })),
    ],
    [characterPresets],
  );
  const updateI2i = useCallback(
    (patch: Partial<NonNullable<GenerationDraft["i2i"]>>) => {
      if (!draft.i2i) {
        return;
      }
      onPatch({ i2i: { ...draft.i2i, ...patch } });
    },
    [draft.i2i, onPatch],
  );
  const updateVibe = useCallback(
    (patch: Partial<GenerationDraft["vibe"]>) => {
      onPatch({ vibe: { ...draft.vibe, ...patch } });
    },
    [draft.vibe, onPatch],
  );

  async function pickSourceImage() {
    setError(null);
    try {
      const [resource] = await onPickImageResources("source_image");
      if (resource) {
        onPatch({
          i2i: {
            image: resource,
            mask: draft.i2i?.mask ?? null,
            strength: draft.i2i?.strength ?? 0.7,
            noise: draft.i2i?.noise ?? 0,
          },
        });
      }
    } catch (err) {
      setError(formatError(err));
    }
  }

  async function pickMaskImage() {
    setError(null);
    try {
      const [resource] = await onPickImageResources("source_image");
      if (resource && draft.i2i) {
        updateI2i({ mask: resource });
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
        onPatch({
          preciseReferences: [
            ...draft.preciseReferences,
            ...resources.map((resource) => ({
              id: createLocalId("ref"),
              image: resource,
              referenceType: DEFAULT_REFERENCE_TYPE,
              fidelity: 0.5,
              strength: 0.6,
              displayName: resource.id,
            })),
          ],
          vibe: { ...draft.vibe, enabled: false },
        });
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
        onPatch({
          vibe: {
            ...draft.vibe,
            enabled: true,
            slots: [
              ...draft.vibe.slots,
              {
                id: createLocalId("vibe"),
                encoding: ensured.encoding,
                vibeId: null,
                informationExtracted: 1,
                strength: 1,
                displayName: ensured.sourceImage.id,
                sourceImage: ensured.sourceImage,
                sourceSha256: ensured.sourceSha256,
              },
            ],
          },
          preciseReferences: [],
        });
      }
    } catch (err) {
      setError(formatError(err));
    }
  }

  return (
    <AppPanel className="min-h-0 overflow-auto">
      <header className="border-b border-app-border px-4 py-3">
        <h2 className="text-sm font-semibold text-white">Image Guidance</h2>
      </header>
      <div className="grid gap-4 p-3 text-sm text-app-text">
        {error ? <p className="text-rose-100">{error}</p> : null}
        <ImageToImageSection
          draft={draft}
          onPatch={onPatch}
          updateI2i={updateI2i}
          pickSourceImage={pickSourceImage}
          pickMaskImage={pickMaskImage}
          imageImportPending={imageImportPending}
        />
        <VibeGuidanceSection
          draft={draft}
          onPatch={onPatch}
          updateVibe={updateVibe}
          vibeDocuments={vibeDocuments}
          vibePending={vibePending}
          vibeError={vibeError}
          vibeImportPending={vibeImportPending}
          vibeExportPending={vibeExportPending}
          vibeEnsurePending={vibeEnsurePending}
          pickVibeEncoding={pickVibeEncoding}
          onImportVibeDocuments={onImportVibeDocuments}
          onExportVibeDocument={onExportVibeDocument}
        />
        <PreciseReferenceSection
          draft={draft}
          onPatch={onPatch}
          pickPreciseReference={pickPreciseReference}
          imageImportPending={imageImportPending}
        />
        <CharacterGuidanceSection
          draft={draft}
          onPatch={onPatch}
          characterPresetOptions={characterPresetOptions}
        />
      </div>
    </AppPanel>
  );
}

function formatError(error: unknown): string {
  return error instanceof Error ? error.message : "Command failed";
}
