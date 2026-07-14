/* eslint-disable react-perf/jsx-no-new-array-as-prop, react-perf/jsx-no-new-function-as-prop */
import { Plus, Trash2 } from "lucide-react";

import { AppButton } from "../../../components/ui";
import type { GenerationDraft } from "../model/generation-draft";
import { createLocalId, patchCharacter } from "./advanced-generation-model";
import { BooleanField, NumberField, SelectField } from "./GenerationFormFields";
import { PromptCompletionTextarea } from "./prompt-completion";

export function CharacterGuidanceSection({
  draft,
  onPatch,
  characterPresetOptions,
}: {
  draft: GenerationDraft;
  onPatch: (patch: Partial<GenerationDraft>) => void;
  characterPresetOptions: ReadonlyArray<{ value: string; label: string }>;
}) {
  return (
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
  );
}
