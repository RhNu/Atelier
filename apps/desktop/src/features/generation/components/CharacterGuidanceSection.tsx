/* eslint-disable react-perf/jsx-no-jsx-as-prop, react-perf/jsx-no-new-function-as-prop */
import { Plus, Power, Trash2 } from "lucide-react";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

import { AppIconButton, AppTabs } from "@/components/ui";
import { NaiPromptEditor, promptProfileForModel } from "@/features/prompt-editor";
import type { PromptPresetDto, PromptTokenUsageDto } from "@/types";

import type { GenerationCharacterDraft, GenerationDraft } from "../model/generation-draft";
import { applyPromptPreset } from "../model/prompt-preset-model";
import type { GenerationDraftPatchOptions } from "../state/useGenerationDraft";
import { createLocalId, patchCharacter } from "./advanced-generation-model";
import { CharacterPositionGrid } from "./CharacterPositionGrid";
import { GenerationPresetControl } from "./GenerationPresetControl";
import { GuidanceSection, GuidanceSettingsDisclosure } from "./GuidanceSection";

type PatchDraft = (patch: Partial<GenerationDraft>, options?: GenerationDraftPatchOptions) => void;
type PromptTab = "positive" | "negative";

export function CharacterGuidanceSection({
  draft,
  onPatch,
  characterPresets,
  characterPresetsPending,
  tokenCounts,
}: {
  draft: GenerationDraft;
  onPatch: PatchDraft;
  characterPresets: ReadonlyArray<PromptPresetDto>;
  characterPresetsPending: boolean;
  tokenCounts: PromptTokenUsageDto | null;
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
        <div className="-mx-2 grid gap-2">
          {draft.characters.map((character, index) => (
            <CharacterCard
              key={character.id}
              draft={draft}
              character={character}
              index={index}
              selected={showPositionSettings && activeCharacterIndex === index}
              characterPresets={characterPresets}
              characterPresetsPending={characterPresetsPending}
              tokenCounts={tokenCounts}
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
  characterPresets,
  characterPresetsPending,
  tokenCounts,
  onPatch,
  onSelect,
  onRemove,
}: {
  draft: GenerationDraft;
  character: GenerationCharacterDraft;
  index: number;
  selected: boolean;
  characterPresets: ReadonlyArray<PromptPresetDto>;
  characterPresetsPending: boolean;
  tokenCounts: PromptTokenUsageDto | null;
  onPatch: PatchDraft;
  onSelect: () => void;
  onRemove: () => void;
}) {
  const { t } = useTranslation("generation");
  const [activeTab, setActiveTab] = useState<PromptTab>("positive");
  const promptTabs = useMemo(
    () => [
      { value: "positive" as const, label: t("positive") },
      { value: "negative" as const, label: t("undesiredContent") },
    ],
    [t],
  );

  function handleEditorKeyDown(event: KeyboardEvent) {
    if (event.ctrlKey && event.key === "Tab") {
      event.preventDefault();
      setActiveTab((current) => (current === "positive" ? "negative" : "positive"));
    }
  }

  return (
    <article
      aria-label={t("character", { index: index + 1 })}
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
          <AppIconButton
            icon={Power}
            label={t(character.enabled ? "disableCharacter" : "enableCharacter", {
              index: index + 1,
            })}
            aria-pressed={character.enabled}
            selected={character.enabled}
            size="sm"
            onClick={() =>
              patchCharacter(draft, onPatch, character.id, {
                enabled: !character.enabled,
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
      <AppTabs
        label={t("promptType")}
        value={activeTab}
        tabs={promptTabs}
        onChange={setActiveTab}
      />
      <NaiPromptEditor
        key={activeTab}
        aria-label={
          activeTab === "positive"
            ? t("characterPrompt", { index: index + 1 })
            : t("characterNegativePrompt", { index: index + 1 })
        }
        value={activeTab === "positive" ? character.prompt : character.negativePrompt}
        onChange={(value) =>
          patchCharacter(
            draft,
            onPatch,
            character.id,
            activeTab === "positive" ? { prompt: value } : { negativePrompt: value },
          )
        }
        profile={promptProfileForModel(draft.model)}
        model={draft.model}
        tokenCount={characterTokenCount(tokenCounts, index, activeTab, character.enabled)}
        onKeyDown={handleEditorKeyDown}
        minHeight={88}
      />
      <GenerationPresetControl
        compact
        label={t("characterPreset")}
        noPresetLabel={t("noCharacterPreset")}
        libraryTitle={t("characterPresetLibrary")}
        presets={characterPresets}
        selectedPresetId={character.presetId}
        pending={characterPresetsPending}
        onSelect={(presetId) =>
          patchCharacter(draft, onPatch, character.id, { presetId }, { persist: "immediate" })
        }
        onClear={() =>
          patchCharacter(draft, onPatch, character.id, { presetId: null }, { persist: "immediate" })
        }
        onApply={(preset) =>
          patchCharacter(
            draft,
            onPatch,
            character.id,
            {
              ...applyPromptPreset(preset, character.prompt, character.negativePrompt),
              presetId: null,
            },
            { persist: "immediate" },
          )
        }
      />
    </article>
  );
}

function characterTokenCount(
  usage: PromptTokenUsageDto | null,
  index: number,
  activeTab: PromptTab,
  enabled: boolean,
) {
  if (!usage) return null;
  if (!enabled) return { used: 0, limit: usage.prompt.limit };
  const character = usage.characters.find((item) => item.index === index);
  if (!character) return null;
  return activeTab === "positive" ? character.prompt : character.negative_prompt;
}
