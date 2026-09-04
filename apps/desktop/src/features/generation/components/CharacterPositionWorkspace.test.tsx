/* eslint-disable react-perf/jsx-no-new-object-as-prop */
import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type { ModelCapabilitiesDto } from "@/types";

import { initializePositionDraft } from "../model/character-position";
import type { GenerationCharacterDraft } from "../model/generation-draft";
import { CharacterPositionWorkspace } from "./CharacterPositionWorkspace";

const characters: GenerationCharacterDraft[] = [
  {
    id: "a",
    presetId: null,
    prompt: "alice",
    negativePrompt: "",
    enabled: true,
    position: { x: 0.5, y: 0.5 },
  },
  {
    id: "b",
    presetId: null,
    prompt: "bob",
    negativePrompt: "",
    enabled: true,
    position: { x: 0.5, y: 0.5 },
  },
];

describe("CharacterPositionWorkspace", () => {
  it("allocates overlapping defaults deterministically from the center", () => {
    expect(initializePositionDraft(characters).map((item) => item.position)).toEqual([
      { x: 0.5, y: 0.5 },
      { x: 0.3, y: 0.5 },
    ]);
  });

  it("uses exact NovelAI 5x5 coordinates and applies staged changes", () => {
    const onApply = vi.fn<(characters: GenerationCharacterDraft[]) => void>();
    render(
      <CharacterPositionWorkspace
        characters={characters}
        capabilities={capabilities("grid_5x5", false)}
        size={{ width: 832, height: 1216 }}
        underlayResource={null}
        underlayStreamSrc={null}
        onApply={onApply}
        onCancel={vi.fn<() => void>()}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: "Position 10%, 90%" }));
    fireEvent.click(screen.getByRole("button", { name: "Apply" }));
    expect(onApply.mock.calls[0]?.[0][0].position).toEqual({ x: 0.1, y: 0.9 });
  });

  it("adds newly eligible characters without losing staged positions", () => {
    const { rerender } = render(
      <CharacterPositionWorkspace
        characters={characters.slice(0, 1)}
        capabilities={capabilities("grid_5x5", false)}
        size={{ width: 832, height: 1216 }}
        underlayResource={null}
        underlayStreamSrc={null}
        onApply={vi.fn<(characters: GenerationCharacterDraft[]) => void>()}
        onCancel={vi.fn<() => void>()}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: "Position 10%, 90%" }));

    const onApply = vi.fn<(characters: GenerationCharacterDraft[]) => void>();
    rerender(
      <CharacterPositionWorkspace
        characters={characters}
        capabilities={capabilities("grid_5x5", false)}
        size={{ width: 832, height: 1216 }}
        underlayResource={null}
        underlayStreamSrc={null}
        onApply={onApply}
        onCancel={vi.fn<() => void>()}
      />,
    );

    expect(screen.getByRole("button", { name: "Select character 2" })).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Apply" }));
    expect(onApply.mock.calls[0]?.[0].map((character) => character.position)).toEqual([
      { x: 0.1, y: 0.9 },
      { x: 0.3, y: 0.5 },
    ]);
  });
});

function capabilities(
  mode: ModelCapabilitiesDto["character_position_mode"],
  oneCharacter: boolean,
): ModelCapabilitiesDto {
  return {
    prompt_structure: "v4",
    params_version: 3,
    default_steps: 23,
    default_scale: 5,
    max_characters: 6,
    character_position_mode: mode,
    can_position_one_character: oneCharacter,
    supports_vibe_transfer: true,
    supports_encoded_vibe: true,
    supports_character_reference: true,
    supports_character_reference_inpainting: true,
    supports_variety_boost: true,
    supports_inpainting: true,
    supports_furry_mode: true,
    supports_streaming: true,
    supports_smea: false,
    supports_dynamic_thresholding: false,
    uses_v5_extensions: false,
    has_opus_usage_limit: false,
    supports_light_quality_preset: false,
    supports_transparent_background: false,
    variety_sigma_coefficient: null,
    prompt_token_limit: 225,
  };
}
