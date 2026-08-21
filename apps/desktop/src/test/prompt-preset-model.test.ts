import { describe, expect, it } from "vitest";

import type { PromptPresetDto } from "@/types";

import { applyPromptPreset } from "../features/generation/model/prompt-preset-model";

const PRESET: PromptPresetDto = {
  preset_id: "preset-1",
  kind: "main",
  name: "Test preset",
  category: null,
  description: null,
  order: 0,
  prompt_behavior: { mode: "surround", before: "masterpiece,", after: "sharp focus" },
  uc_behavior: { mode: "replace", text: "bad anatomy" },
  quality_override: null,
  uc_preset_override: null,
  preview: null,
  created_at_ms: 1,
  updated_at_ms: 1,
  models: ["nai-diffusion-4-5-full"],
};

describe("prompt preset application", () => {
  it("surrounds prompt text with normalized boundaries and replaces UC text", () => {
    expect(applyPromptPreset(PRESET, "1girl", "old UC")).toEqual({
      prompt: "masterpiece, 1girl, sharp focus",
      negativePrompt: "bad anatomy",
    });
  });

  it("preserves prompt syntax boundaries used by the backend compiler", () => {
    expect(
      applyPromptPreset(
        {
          ...PRESET,
          prompt_behavior: { mode: "surround", before: "{", after: "}" },
          uc_behavior: { mode: "surround", before: "", after: "" },
        },
        "1girl",
        "",
      ),
    ).toEqual({
      prompt: "{1girl}",
      negativePrompt: "",
    });
  });
});
