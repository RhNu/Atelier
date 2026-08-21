/* eslint-disable max-lines, react-perf/jsx-no-new-function-as-prop, typescript/no-misused-promises */
import { Download, ImagePlus, Library, Trash2, Upload } from "lucide-react";
import { useCallback, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

import { reportBackgroundPromise } from "@/app/logger";
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

type VibeGuidanceSectionProps = {
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
};

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
}: VibeGuidanceSectionProps) {
  const { t } = useTranslation("generation");
  const [libraryOpen, setLibraryOpen] = useState(false);
  const slots = useMemo(
    () => draft.vibe.slots.filter((slot) => slot.model === draft.model),
    [draft.model, draft.vibe.slots],
  );
  const openLibrary = useCallback(() => setLibraryOpen(true), []);

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
              model: draft.model,
            },
          ],
        },
        preciseReferences: [],
      },
      { persist: "immediate" },
    );
    reportBackgroundPromise(
      releaseImages(draft.preciseReferences.map((reference) => reference.image)),
      "Release precise reference images",
    );
  }
  const actions = useMemo(
    () => (
      <VibeActions
        draft={draft}
        slots={slots}
        onPatch={onPatch}
        pickVibeEncoding={pickVibeEncoding}
        onImportVibeDocuments={onImportVibeDocuments}
        releaseImages={releaseImages}
        vibeImportPending={vibeImportPending}
        vibeEnsurePending={vibeEnsurePending}
        openLibrary={openLibrary}
      />
    ),
    [
      draft,
      onImportVibeDocuments,
      onPatch,
      openLibrary,
      pickVibeEncoding,
      releaseImages,
      slots,
      vibeEnsurePending,
      vibeImportPending,
    ],
  );

  return (
    <>
      <GuidanceSection title={t("vibeTransfer")} actions={actions}>
        {slots.length > 0 ? (
          <>
            <AppRangeField
              label={t("vibeStrength")}
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

function VibeActions({
  draft,
  slots,
  onPatch,
  pickVibeEncoding,
  onImportVibeDocuments,
  releaseImages,
  vibeImportPending,
  vibeEnsurePending,
  openLibrary,
}: Pick<
  VibeGuidanceSectionProps,
  | "draft"
  | "onPatch"
  | "pickVibeEncoding"
  | "onImportVibeDocuments"
  | "releaseImages"
  | "vibeImportPending"
  | "vibeEnsurePending"
> & {
  slots: GenerationDraft["vibe"]["slots"];
  openLibrary: () => void;
}) {
  const { t } = useTranslation("generation");
  return (
    <>
      <AppIconButton
        icon={ImagePlus}
        label={t("addVibeFromImage")}
        size="sm"
        onClick={pickVibeEncoding}
        disabled={vibeEnsurePending}
      />
      <AppIconButton
        icon={Library}
        label={t("chooseVibeLibrary")}
        size="sm"
        onClick={openLibrary}
      />
      <AppIconButton
        icon={Upload}
        label={t("importVibeFile")}
        size="sm"
        onClick={onImportVibeDocuments}
        disabled={vibeImportPending}
      />
      {slots.length > 0 ? (
        <AppIconButton
          icon={Trash2}
          label={t("clearVibeStack")}
          size="sm"
          variant="danger"
          onClick={() => {
            const resources = slots.map((slot) => slot.sourceImage);
            onPatch(
              {
                vibe: {
                  ...draft.vibe,
                  slots: draft.vibe.slots.filter((slot) => slot.model !== draft.model),
                },
              },
              { persist: "immediate" },
            );
            reportBackgroundPromise(releaseImages(resources), "Release Vibe source images");
          }}
        />
      ) : null}
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
  const { t } = useTranslation("generation");
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
            reportBackgroundPromise(releaseImages([slot.sourceImage]), "Release Vibe source image");
          }}
        />
      </div>
      <GuidanceSettingsDisclosure>
        <AppRangeField
          label={t("infoExtracted")}
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
          label={t("slotStrength")}
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
