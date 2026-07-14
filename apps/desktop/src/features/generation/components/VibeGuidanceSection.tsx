/* eslint-disable react-perf/jsx-no-new-array-as-prop, react-perf/jsx-no-new-function-as-prop, typescript/no-misused-promises */
import { Plus, Trash2 } from "lucide-react";

import { AppButton } from "../../../components/ui";
import type { VibeDocumentEntryDto } from "../../../types";
import type { GenerationDraft } from "../model/generation-draft";
import { createLocalId } from "./advanced-generation-model";
import { BooleanField, NumberField, SelectField } from "./GenerationFormFields";
import { GuidancePanelTitle } from "./GuidancePanelTitle";
import { findVibeEncodingForModel } from "./vibe-guidance-model";

export function VibeGuidanceSection({
  draft,
  onPatch,
  updateVibe,
  vibeDocuments,
  vibePending,
  vibeError,
  vibeImportPending,
  vibeExportPending,
  vibeEnsurePending,
  pickVibeEncoding,
  onImportVibeDocuments,
  onExportVibeDocument,
}: {
  draft: GenerationDraft;
  onPatch: (patch: Partial<GenerationDraft>) => void;
  updateVibe: (patch: Partial<GenerationDraft["vibe"]>) => void;
  vibeDocuments: ReadonlyArray<VibeDocumentEntryDto>;
  vibePending: boolean;
  vibeError: string | null;
  vibeImportPending: boolean;
  vibeExportPending: boolean;
  vibeEnsurePending: boolean;
  pickVibeEncoding: () => Promise<void>;
  onImportVibeDocuments: () => void;
  onExportVibeDocument: (vibeId: string) => void;
}) {
  return (
    <section className="grid gap-2">
      <GuidancePanelTitle title="Vibe transfer" resource={draft.vibe.slots[0]?.encoding ?? null} />
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
            .filter((entry) => findVibeEncodingForModel(entry, draft.model) !== null)
            .map((entry) => ({
              value: entry.vibe_id,
              label: entry.display_name,
            })),
        ]}
        onChange={(vibeId) => {
          const entry = vibeDocuments.find((item) => item.vibe_id === vibeId);
          const selected = entry ? findVibeEncodingForModel(entry, draft.model) : null;
          if (!entry || !selected) {
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
                  encoding: selected.encoding,
                  vibeId: entry.vibe_id,
                  informationExtracted: selected.config.information_extracted,
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
        <VibeSlot
          key={slot.id}
          draft={draft}
          slot={slot}
          updateVibe={updateVibe}
          vibeExportPending={vibeExportPending}
          onExportVibeDocument={onExportVibeDocument}
        />
      ))}
    </section>
  );
}

function VibeSlot({
  draft,
  slot,
  updateVibe,
  vibeExportPending,
  onExportVibeDocument,
}: {
  draft: GenerationDraft;
  slot: GenerationDraft["vibe"]["slots"][number];
  updateVibe: (patch: Partial<GenerationDraft["vibe"]>) => void;
  vibeExportPending: boolean;
  onExportVibeDocument: (vibeId: string) => void;
}) {
  return (
    <div className="grid gap-2 border border-app-border bg-black/20 p-2">
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
          onClick={() => slot.vibeId && onExportVibeDocument(slot.vibeId)}
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
  );
}
