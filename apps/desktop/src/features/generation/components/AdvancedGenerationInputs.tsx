/* eslint-disable max-lines, max-lines-per-function, react-perf/jsx-no-new-array-as-prop, react-perf/jsx-no-new-function-as-prop, typescript/no-misused-promises, typescript/no-unsafe-type-assertion */
import { ImagePlus, Plus, Trash2 } from "lucide-react";
import { useCallback, useMemo, useState } from "react";

import { AppButton, AppPanel } from "../../../components/ui";
import type {
  CharacterReferenceTypeDto,
  PromptPresetDto,
  ResourceRefDto,
  VibeDocumentEntryDto,
} from "../../../types";
import type { EnsuredVibeEncodingFromResource } from "../data/useGenerationActions";
import type { GenerationDraft } from "../model/generation-draft";
import { BooleanField, NumberField, SelectField } from "./GenerationFormFields";
import { PromptCompletionTextarea } from "./prompt-completion";

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

const REFERENCE_TYPE_OPTIONS = [
  { value: "character", label: "Character" },
  { value: "style", label: "Style" },
  { value: "character_and_style", label: "Character + style" },
] as const;

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
              referenceType: "character" as CharacterReferenceTypeDto,
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
        <section className="grid gap-2">
          <PanelTitle title="Image to image" resource={draft.i2i?.image ?? null} />
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
            <AppButton variant="ghost" onClick={() => onPatch({ i2i: null })} disabled={!draft.i2i}>
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

        <section className="grid gap-2">
          <PanelTitle title="Vibe transfer" resource={draft.vibe.slots[0]?.encoding ?? null} />
          <div className="flex flex-wrap gap-2">
            <AppButton variant="secondary" onClick={pickVibeEncoding} disabled={vibeEnsurePending}>
              <Plus aria-hidden="true" className="size-4" />
              Add Vibe slot
            </AppButton>
            <AppButton variant="ghost" onClick={onImportVibeDocuments} disabled={vibeImportPending}>
              <Plus aria-hidden="true" className="size-4" />
              Import Vibe file
            </AppButton>
            <AppButton
              variant="ghost"
              onClick={() => updateVibe({ slots: [], enabled: false })}
              disabled={draft.vibe.slots.length === 0}
            >
              <Trash2 aria-hidden="true" className="size-4" />
              Clear stack
            </AppButton>
          </div>
          <BooleanField
            label="Enable Vibe transfer"
            checked={draft.vibe.enabled}
            onChange={(enabled) => updateVibe({ enabled })}
          />
          <NumberField
            label="Vibe strength"
            value={draft.vibe.strength}
            min={0}
            max={1}
            step={0.01}
            onChange={(strength) => updateVibe({ strength })}
          />
          {vibeError ? <p className="text-xs text-rose-100">{vibeError}</p> : null}
          <SelectField
            label="Vibe library"
            value=""
            options={[
              {
                value: "",
                label: vibePending ? "Loading Vibes" : "Choose imported Vibe",
              },
              ...vibeDocuments
                .filter((entry) => entry.encodings.length > 0)
                .map((entry) => ({
                  value: entry.vibe_id,
                  label: entry.display_name,
                })),
            ]}
            onChange={(vibeId) => {
              const entry = vibeDocuments.find((item) => item.vibe_id === vibeId);
              const encoding = entry?.encodings[0];
              if (!entry || !encoding) {
                return;
              }
              onPatch({
                vibe: {
                  ...draft.vibe,
                  enabled: true,
                  slots: [
                    ...draft.vibe.slots,
                    {
                      id: createLocalId("vibe"),
                      encoding,
                      vibeId: entry.vibe_id,
                      informationExtracted:
                        entry.available_encoding_configs[0]?.information_extracted ?? 1,
                      strength: 1,
                      displayName: entry.display_name,
                      sourceImage: entry.source_image,
                      sourceSha256: null,
                    },
                  ],
                },
                preciseReferences: [],
              });
            }}
          />
          {draft.vibe.slots.map((slot) => (
            <div key={slot.id} className="grid gap-2 border border-app-border bg-black/20 p-2">
              <div className="flex items-center justify-between gap-2">
                <span className="truncate text-xs text-app-muted">{slot.displayName}</span>
                <button
                  type="button"
                  aria-label={`Remove ${slot.displayName}`}
                  className="text-app-muted hover:text-rose-100"
                  onClick={() =>
                    updateVibe({ slots: draft.vibe.slots.filter((item) => item.id !== slot.id) })
                  }
                >
                  <Trash2 aria-hidden="true" className="size-4" />
                </button>
              </div>
              {slot.vibeId ? (
                <AppButton
                  variant="ghost"
                  onClick={() => onExportVibeDocument(slot.vibeId as string)}
                  disabled={vibeExportPending}
                >
                  Export Vibe
                </AppButton>
              ) : null}
              <div className="grid grid-cols-2 gap-2">
                <NumberField
                  label="Info extracted"
                  value={slot.informationExtracted}
                  min={0.01}
                  max={1}
                  step={0.01}
                  onChange={(informationExtracted) =>
                    updateVibe({
                      slots: draft.vibe.slots.map((item) =>
                        item.id === slot.id ? { ...item, informationExtracted } : item,
                      ),
                    })
                  }
                />
                <NumberField
                  label="Slot strength"
                  value={slot.strength}
                  min={0}
                  max={1}
                  step={0.01}
                  onChange={(strength) =>
                    updateVibe({
                      slots: draft.vibe.slots.map((item) =>
                        item.id === slot.id ? { ...item, strength } : item,
                      ),
                    })
                  }
                />
              </div>
            </div>
          ))}
        </section>

        <section className="grid gap-2">
          <PanelTitle
            title="Precise reference"
            resource={draft.preciseReferences[0]?.image ?? null}
          />
          <AppButton
            variant="secondary"
            onClick={pickPreciseReference}
            disabled={imageImportPending}
          >
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
                  onClick={() =>
                    onPatch({
                      preciseReferences: draft.preciseReferences.filter(
                        (item) => item.id !== reference.id,
                      ),
                    })
                  }
                >
                  <Trash2 aria-hidden="true" className="size-4" />
                </button>
              </div>
              <SelectField
                label="Reference type"
                value={reference.referenceType}
                options={REFERENCE_TYPE_OPTIONS}
                onChange={(value) =>
                  onPatch({
                    preciseReferences: draft.preciseReferences.map((item) =>
                      item.id === reference.id
                        ? { ...item, referenceType: value as CharacterReferenceTypeDto }
                        : item,
                    ),
                  })
                }
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

        <section className="grid gap-2">
          <div className="flex items-center justify-between gap-2">
            <h3 className="text-xs font-semibold text-app-muted uppercase">Characters</h3>
            <AppButton
              variant="ghost"
              aria-label="Add character prompt"
              onClick={() =>
                onPatch({
                  characters: [
                    ...draft.characters,
                    {
                      id: createLocalId("char"),
                      presetId: null,
                      prompt: "",
                      negativePrompt: "",
                      enabled: true,
                      position: { x: 0.5, y: 0.5 },
                    },
                  ],
                })
              }
            >
              <Plus aria-hidden="true" className="size-4" />
              Character
            </AppButton>
          </div>
          <SelectField
            label="Position mode"
            value={draft.characterPositionMode}
            options={[
              { value: "global", label: "AI choice" },
              { value: "manual", label: "Manual coordinates" },
            ]}
            onChange={(value) =>
              onPatch({ characterPositionMode: value === "manual" ? "manual" : "global" })
            }
          />
          {draft.characters.map((character, index) => (
            <div key={character.id} className="grid gap-2 border border-app-border bg-black/20 p-2">
              <div className="flex items-center justify-between gap-2">
                <span className="text-xs font-semibold text-app-muted uppercase">
                  Character {index + 1}
                </span>
                <button
                  type="button"
                  aria-label={`Remove character ${index + 1}`}
                  className="text-app-muted hover:text-rose-100"
                  onClick={() =>
                    onPatch({
                      characters: draft.characters.filter((item) => item.id !== character.id),
                    })
                  }
                >
                  <Trash2 aria-hidden="true" className="size-4" />
                </button>
              </div>
              <BooleanField
                label="Enabled"
                checked={character.enabled}
                onChange={(enabled) => patchCharacter(draft, onPatch, character.id, { enabled })}
              />
              <SelectField
                label="Character preset"
                value={character.presetId ?? ""}
                options={characterPresetOptions}
                onChange={(presetId) =>
                  patchCharacter(draft, onPatch, character.id, { presetId: presetId || null })
                }
              />
              <PromptCompletionTextarea
                aria-label={`Character ${index + 1} prompt`}
                value={character.prompt}
                onChange={(prompt) => patchCharacter(draft, onPatch, character.id, { prompt })}
                className="min-h-20 resize-none border border-app-border bg-black/20 p-2 text-sm text-app-text outline-none focus:border-brand-400"
              />
              <PromptCompletionTextarea
                aria-label={`Character ${index + 1} negative prompt`}
                value={character.negativePrompt}
                onChange={(negativePrompt) =>
                  patchCharacter(draft, onPatch, character.id, { negativePrompt })
                }
                className="min-h-16 resize-none border border-app-border bg-black/20 p-2 text-sm text-app-text outline-none focus:border-brand-400"
              />
              <div className="grid grid-cols-2 gap-2">
                <NumberField
                  label="X"
                  value={character.position.x}
                  min={0}
                  max={1}
                  step={0.1}
                  onChange={(x) =>
                    patchCharacter(draft, onPatch, character.id, {
                      position: { ...character.position, x },
                    })
                  }
                />
                <NumberField
                  label="Y"
                  value={character.position.y}
                  min={0}
                  max={1}
                  step={0.1}
                  onChange={(y) =>
                    patchCharacter(draft, onPatch, character.id, {
                      position: { ...character.position, y },
                    })
                  }
                />
              </div>
            </div>
          ))}
        </section>
      </div>
    </AppPanel>
  );
}

function patchPreciseReference(
  draft: GenerationDraft,
  onPatch: (patch: Partial<GenerationDraft>) => void,
  id: string,
  patch: Partial<GenerationDraft["preciseReferences"][number]>,
) {
  onPatch({
    preciseReferences: draft.preciseReferences.map((reference) =>
      reference.id === id ? { ...reference, ...patch } : reference,
    ),
  });
}

function PanelTitle({ title, resource }: { title: string; resource: ResourceRefDto | null }) {
  return (
    <div className="flex items-center justify-between gap-2">
      <h3 className="text-xs font-semibold text-app-muted uppercase">{title}</h3>
      {resource ? (
        <span className="max-w-32 truncate text-xs text-brand-200">{resource.id}</span>
      ) : null}
    </div>
  );
}

function patchCharacter(
  draft: GenerationDraft,
  onPatch: (patch: Partial<GenerationDraft>) => void,
  id: string,
  patch: Partial<GenerationDraft["characters"][number]>,
) {
  onPatch({
    characters: draft.characters.map((character) =>
      character.id === id ? { ...character, ...patch } : character,
    ),
  });
}

function createLocalId(prefix: string): string {
  return `${prefix}-${globalThis.crypto?.randomUUID?.() ?? Math.random().toString(16).slice(2)}`;
}

function formatError(error: unknown): string {
  return error instanceof Error ? error.message : "Command failed";
}
