/* eslint-disable react-perf/jsx-no-new-function-as-prop, typescript/no-misused-promises */
import { Download, ImagePlus, Library, Trash2, Upload } from "lucide-react";
import { useState } from "react";

import { AppIconButton, AppRangeField } from "@/components/ui";
import type { ResourceRefDto, VibeDocumentEntryDto } from "@/types";

import type { GenerationDraft } from "../model/generation-draft";
import type { GenerationDraftPatchOptions } from "../state/useGenerationDraft";
import { createLocalId } from "./advanced-generation-model";
import { GenerationResourceThumbnail } from "./GenerationResourceThumbnail";
import {
  GuidanceDeveloperMetadata,
  GuidanceSection,
  GuidanceSettingsDisclosure,
} from "./GuidanceSection";
import { findVibeEncodingForModel } from "./vibe-guidance-model";
import { VibeLibraryDialog } from "./VibeLibraryDialog";

export function VibeGuidanceSection({
  draft,
  onPatch,
  onFlush,
  vibeImportPending,
  vibeExportPending,
  vibeEnsurePending,
  pickVibeEncoding,
  onImportVibeDocuments,
  onExportVibeDocument,
  releaseImages,
  developerMode,
}: {
  draft: GenerationDraft;
  onPatch: (patch: Partial<GenerationDraft>, options?: GenerationDraftPatchOptions) => void;
  onFlush: () => void;
  vibeImportPending: boolean;
  vibeExportPending: boolean;
  vibeEnsurePending: boolean;
  pickVibeEncoding: () => Promise<void>;
  onImportVibeDocuments: () => void;
  onExportVibeDocument: (vibeId: string) => void;
  releaseImages: (resources: ReadonlyArray<ResourceRefDto | null>) => Promise<void>;
  developerMode: boolean;
}) {
  const [libraryOpen, setLibraryOpen] = useState(false);
  const slots = draft.vibe.slots;

  function updateVibe(patch: Partial<GenerationDraft["vibe"]>) {
    onPatch({ vibe: { ...draft.vibe, ...patch } });
  }

  function selectLibraryEntry(entry: VibeDocumentEntryDto) {
    const selected = findVibeEncodingForModel(entry, draft.model);
    if (!selected) {
      return;
    }
    onPatch(
      {
        vibe: {
          ...draft.vibe,
          slots: [
            ...slots,
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
      },
      { persist: "immediate" },
    );
    void releaseImages(draft.preciseReferences.map((reference) => reference.image));
  }

  return (
    <>
      <GuidanceSection
        title="Vibe transfer"
        actions={
          <>
            <AppIconButton
              icon={ImagePlus}
              label="Add Vibe from image"
              size="sm"
              onClick={pickVibeEncoding}
              disabled={vibeEnsurePending}
            />
            <AppIconButton
              icon={Library}
              label="Choose from Vibe library"
              size="sm"
              onClick={() => setLibraryOpen(true)}
            />
            <AppIconButton
              icon={Upload}
              label="Import Vibe file"
              size="sm"
              onClick={onImportVibeDocuments}
              disabled={vibeImportPending}
            />
            {slots.length > 0 ? (
              <AppIconButton
                icon={Trash2}
                label="Clear Vibe stack"
                size="sm"
                variant="danger"
                onClick={() => {
                  const resources = slots.map((slot) => slot.sourceImage);
                  onPatch({ vibe: { ...draft.vibe, slots: [] } }, { persist: "immediate" });
                  void releaseImages(resources);
                }}
              />
            ) : null}
          </>
        }
      >
        {slots.length > 0 ? (
          <>
            <AppRangeField
              label="Vibe strength"
              value={draft.vibe.strength}
              valueText={draft.vibe.strength.toFixed(1)}
              min={0}
              max={1}
              step={0.1}
              onChange={(strength) => updateVibe({ strength })}
              onCommit={onFlush}
            />
            <div className="grid gap-2">
              {slots.map((slot) => (
                <VibeSlot
                  key={slot.id}
                  draft={draft}
                  slot={slot}
                  updateVibe={updateVibe}
                  onFlush={onFlush}
                  vibeExportPending={vibeExportPending}
                  onExportVibeDocument={onExportVibeDocument}
                  releaseImages={releaseImages}
                  developerMode={developerMode}
                />
              ))}
            </div>
          </>
        ) : null}
      </GuidanceSection>

      <VibeLibraryDialog
        open={libraryOpen}
        model={draft.model}
        onClose={() => setLibraryOpen(false)}
        onSelect={selectLibraryEntry}
      />
    </>
  );
}

function VibeSlot({
  draft,
  slot,
  updateVibe,
  onFlush,
  vibeExportPending,
  onExportVibeDocument,
  releaseImages,
  developerMode,
}: {
  draft: GenerationDraft;
  slot: GenerationDraft["vibe"]["slots"][number];
  updateVibe: (patch: Partial<GenerationDraft["vibe"]>) => void;
  onFlush: () => void;
  vibeExportPending: boolean;
  onExportVibeDocument: (vibeId: string) => void;
  releaseImages: (resources: ReadonlyArray<ResourceRefDto | null>) => Promise<void>;
  developerMode: boolean;
}) {
  return (
    <article className="grid gap-2 border border-app-border bg-app-bg/70 p-2">
      <div className="flex min-w-0 items-center gap-2">
        <GenerationResourceThumbnail
          resource={slot.sourceImage}
          alt={slot.displayName}
          className="size-12"
        />
        <span className="min-w-0 flex-1 truncate text-xs font-semibold text-app-text">
          {slot.displayName}
        </span>
        {slot.vibeId ? (
          <AppIconButton
            icon={Download}
            label={`Export ${slot.displayName}`}
            size="sm"
            disabled={vibeExportPending}
            onClick={() => slot.vibeId && onExportVibeDocument(slot.vibeId)}
          />
        ) : null}
        <AppIconButton
          icon={Trash2}
          label={`Remove ${slot.displayName}`}
          size="sm"
          variant="danger"
          onClick={() => {
            updateVibe({ slots: draft.vibe.slots.filter((item) => item.id !== slot.id) });
            void releaseImages([slot.sourceImage]);
          }}
        />
      </div>
      <GuidanceSettingsDisclosure>
        <AppRangeField
          label="Info extracted"
          value={slot.informationExtracted}
          valueText={slot.informationExtracted.toFixed(2)}
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
          onCommit={onFlush}
        />
        <AppRangeField
          label="Slot strength"
          value={slot.strength}
          valueText={slot.strength.toFixed(2)}
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
          onCommit={onFlush}
        />
        <GuidanceDeveloperMetadata
          enabled={developerMode}
          resource={slot.encoding}
          vibeId={slot.vibeId}
        />
      </GuidanceSettingsDisclosure>
    </article>
  );
}
