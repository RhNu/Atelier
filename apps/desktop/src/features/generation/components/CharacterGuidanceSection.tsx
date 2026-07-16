/* eslint-disable react-perf/jsx-no-jsx-as-prop, react-perf/jsx-no-new-function-as-prop */
import { Plus, Trash2 } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";

import { AppIconButton } from "@/components/ui";
import { NaiPromptEditor, promptProfileForModel } from "@/features/prompt-editor";

import type { GenerationCharacterDraft, GenerationDraft } from "../model/generation-draft";
import type { GenerationDraftPatchOptions } from "../state/useGenerationDraft";
import { createLocalId, patchCharacter } from "./advanced-generation-model";
import { CharacterPositionGrid } from "./CharacterPositionGrid";
import { SelectField } from "./GenerationFormFields";
import { GuidanceSection, GuidanceSettingsDisclosure } from "./GuidanceSection";

type PatchDraft = (patch: Partial<GenerationDraft>, options?: GenerationDraftPatchOptions) => void;

export function CharacterGuidanceSection({
  draft,
  onPatch,
  characterPresetOptions,
}: {
  draft: GenerationDraft;
  onPatch: PatchDraft;
  characterPresetOptions: ReadonlyArray<{ value: string; label: string }>;
}) {
  const { t } = useTranslation("generation");
  const [activeCharacterIndex, setActiveCharacterIndex] = useState(0);
  const showPositionSettings = draft.characters.length >= 2;

  function addCharacter() {
    onPatch(
      {
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
      },
      { persist: "immediate" },
    );
  }

  function removeCharacter(characterId: string) {
    const characters = draft.characters.filter((item) => item.id !== characterId);
    onPatch(
      {
        characters,
        characterPositionMode: characters.length >= 2 ? draft.characterPositionMode : "global",
      },
      { persist: "immediate" },
    );
    setActiveCharacterIndex((index) => Math.max(0, Math.min(index, characters.length - 1)));
  }

  return (
    <GuidanceSection
      title={t("characters")}
      actions={
        <AppIconButton icon={Plus} label={t("addCharacter")} size="sm" onClick={addCharacter} />
      }
    >
      <CharacterPositionSettings
        draft={draft}
        onPatch={onPatch}
        activeCharacterIndex={activeCharacterIndex}
        onSelectCharacter={setActiveCharacterIndex}
      />
      {draft.characters.length > 0 ? (
        <div className="grid gap-2">
          {draft.characters.map((character, index) => (
            <CharacterCard
              key={character.id}
              draft={draft}
              character={character}
              index={index}
              selected={showPositionSettings && activeCharacterIndex === index}
              characterPresetOptions={characterPresetOptions}
              onPatch={onPatch}
              onSelect={() => setActiveCharacterIndex(index)}
              onRemove={() => removeCharacter(character.id)}
            />
          ))}
        </div>
      ) : null}
    </GuidanceSection>
  );
}

function CharacterPositionSettings({
  draft,
  onPatch,
  activeCharacterIndex,
  onSelectCharacter,
}: {
  draft: GenerationDraft;
  onPatch: PatchDraft;
  activeCharacterIndex: number;
  onSelectCharacter: (index: number) => void;
}) {
  const { t } = useTranslation("generation");
  if (draft.characters.length < 2) {
    return null;
  }
  const useAiPositioning = draft.characterPositionMode === "global";

  return (
    <GuidanceSettingsDisclosure title={t("position")} defaultOpen>
      <label className="flex items-center justify-between gap-3 text-xs text-app-muted">
        {t("aiPositioning")}
        <input
          aria-label={t("aiPositioning")}
          title={t("aiPositioning")}
          type="checkbox"
          checked={useAiPositioning}
          onChange={(event) =>
            onPatch(
              { characterPositionMode: event.target.checked ? "global" : "manual" },
              { persist: "immediate" },
            )
          }
        />
      </label>
      {!useAiPositioning ? (
        <CharacterPositionGrid
          characters={draft.characters}
          activeIndex={Math.min(activeCharacterIndex, draft.characters.length - 1)}
          onSelectCharacter={onSelectCharacter}
          onChangePosition={(index, position) => {
            const character = draft.characters[index];
            if (character) {
              patchCharacter(draft, onPatch, character.id, { position });
            }
          }}
        />
      ) : null}
    </GuidanceSettingsDisclosure>
  );
}

function CharacterCard({
  draft,
  character,
  index,
  selected,
  characterPresetOptions,
  onPatch,
  onSelect,
  onRemove,
}: {
  draft: GenerationDraft;
  character: GenerationCharacterDraft;
  index: number;
  selected: boolean;
  characterPresetOptions: ReadonlyArray<{ value: string; label: string }>;
  onPatch: PatchDraft;
  onSelect: () => void;
  onRemove: () => void;
}) {
  const { t } = useTranslation("generation");
  return (
    <article
      className={[
        "grid gap-2 border bg-app-bg/70 p-2",
        selected ? "border-brand-400/60" : "border-app-border",
      ].join(" ")}
    >
      <header className="flex items-center justify-between gap-2">
        <button
          type="button"
          aria-label={t("selectCharacter", { index: index + 1 })}
          aria-pressed={selected}
          className="text-xs font-semibold text-app-muted hover:text-app-text"
          onClick={onSelect}
        >
          {t("character", { index: index + 1 })}
        </button>
        <div className="flex items-center gap-1">
          <input
            aria-label={t("enableCharacter", { index: index + 1 })}
            title={t("enableCharacter", { index: index + 1 })}
            type="checkbox"
            checked={character.enabled}
            onChange={(event) =>
              patchCharacter(draft, onPatch, character.id, {
                enabled: event.target.checked,
              })
            }
          />
          <AppIconButton
            icon={Trash2}
            label={t("removeCharacter", { index: index + 1 })}
            size="sm"
            variant="danger"
            onClick={onRemove}
          />
        </div>
      </header>
      <SelectField
        label={t("characterPreset")}
        value={character.presetId ?? ""}
        options={characterPresetOptions}
        onChange={(presetId) =>
          patchCharacter(draft, onPatch, character.id, { presetId: presetId || null })
        }
      />
      <NaiPromptEditor
        aria-label={t("characterPrompt", { index: index + 1 })}
        value={character.prompt}
        onChange={(prompt) => patchCharacter(draft, onPatch, character.id, { prompt })}
        profile={promptProfileForModel(draft.model)}
        minHeight={72}
        showStatus={false}
      />
      <NaiPromptEditor
        aria-label={t("characterNegativePrompt", { index: index + 1 })}
        value={character.negativePrompt}
        onChange={(negativePrompt) =>
          patchCharacter(draft, onPatch, character.id, { negativePrompt })
        }
        profile={promptProfileForModel(draft.model)}
        minHeight={64}
        showStatus={false}
      />
    </article>
  );
}
