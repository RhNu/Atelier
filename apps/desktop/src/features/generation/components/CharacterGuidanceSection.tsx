/* eslint-disable react-perf/jsx-no-jsx-as-prop, react-perf/jsx-no-new-function-as-prop */
import { Plus, Trash2 } from "lucide-react";
import { useState } from "react";

import { AppIconButton } from "../../../components/ui";
import type { GenerationCharacterDraft, GenerationDraft } from "../model/generation-draft";
import type { GenerationDraftPatchOptions } from "../state/useGenerationDraft";
import { createLocalId, patchCharacter } from "./advanced-generation-model";
import { CharacterPositionGrid } from "./CharacterPositionGrid";
import { SelectField } from "./GenerationFormFields";
import { GuidanceSection, GuidanceSettingsDisclosure } from "./GuidanceSection";
import { PromptCompletionTextarea } from "./prompt-completion";

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
      title="Characters"
      actions={
        <AppIconButton icon={Plus} label="Add character prompt" size="sm" onClick={addCharacter} />
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
  if (draft.characters.length < 2) {
    return null;
  }
  const useAiPositioning = draft.characterPositionMode === "global";

  return (
    <GuidanceSettingsDisclosure title="Position" defaultOpen>
      <label className="flex items-center justify-between gap-3 text-xs text-app-muted">
        AI positioning
        <input
          aria-label="Use AI character positioning"
          title="Use AI character positioning"
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
          aria-label={`Select character ${index + 1}`}
          aria-pressed={selected}
          className="text-xs font-semibold text-app-muted hover:text-app-text"
          onClick={onSelect}
        >
          Character {index + 1}
        </button>
        <div className="flex items-center gap-1">
          <input
            aria-label={`Enable character ${index + 1}`}
            title={`Enable character ${index + 1}`}
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
            label={`Remove character ${index + 1}`}
            size="sm"
            variant="danger"
            onClick={onRemove}
          />
        </div>
      </header>
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
        className="min-h-16 resize-none border border-app-border bg-black/20 p-2 text-sm text-app-text outline-none focus:border-brand-400"
      />
      <PromptCompletionTextarea
        aria-label={`Character ${index + 1} negative prompt`}
        value={character.negativePrompt}
        onChange={(negativePrompt) =>
          patchCharacter(draft, onPatch, character.id, { negativePrompt })
        }
        className="min-h-14 resize-none border border-app-border bg-black/20 p-2 text-sm text-app-text outline-none focus:border-brand-400"
      />
    </article>
  );
}
