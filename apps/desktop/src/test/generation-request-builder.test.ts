import {
  buildSubmitGenerationRequest,
  createGenerationDraft,
} from "../features/generation/model/generation-draft";
import type { WorkspaceSettingsDto } from "../types";

const settings: WorkspaceSettingsDto = {
  generation: {
    model: "nai-diffusion-4-5-full",
    size: { width: 832, height: 1216 },
    quality: true,
    uc_preset: "light",
    steps: 23,
    scale: 5,
    sampler: "k_euler_ancestral",
    noise_schedule: "karras",
    seed: 0,
    n_samples: 1,
    cfg_rescale: 0,
    variety_boost: false,
    image_format: null,
    strict_mode: false,
  },
  image_variants: {
    thumbnail_long_edge: 320,
    preview_long_edge: 1024,
  },
};

describe("generation request builder", () => {
  it("creates a draft from workspace generation settings without persisting UI-only state", () => {
    const draft = createGenerationDraft(settings);

    expect(draft).toMatchObject({
      prompt: "",
      negativePrompt: "",
      model: "nai-diffusion-4-5-full",
      size: { width: 832, height: 1216 },
      quality: true,
      ucPreset: "light",
      steps: 23,
      scale: 5,
      sampler: "k_euler_ancestral",
      noiseSchedule: "karras",
      seed: 0,
      nSamples: 1,
      cfgRescale: 0,
      varietyBoost: false,
      imageFormat: null,
      strictMode: false,
      streamEnabled: true,
    });
  });

  it("builds stream generation work and leaves advanced image inputs unset", () => {
    const draft = {
      ...createGenerationDraft(settings),
      prompt: "1girl, atelier lighting",
      negativePrompt: "low quality",
      steps: 28,
      seed: 1234,
      nSamples: 2,
      imageFormat: "png" as const,
    };

    const request = buildSubmitGenerationRequest(draft, {
      batchId: "batch-test",
      jobId: "job-test",
    });

    expect(request).toEqual({
      batch_id: "batch-test",
      job_id: "job-test",
      context: {
        request_count: 1,
        pending_vibe_encode_count: 0,
        is_opus: false,
      },
      work: {
        kind: "stream",
        request: {
          stream: "sse",
          base: {
            prompt: "1girl, atelier lighting",
            negative_prompt: "low quality",
            model: "nai-diffusion-4-5-full",
            size: { width: 832, height: 1216 },
            quality: true,
            uc_preset: "light",
            steps: 28,
            scale: 5,
            sampler: "k_euler_ancestral",
            noise_schedule: "karras",
            seed: 1234,
            n_samples: 2,
            cfg_rescale: 0,
            variety_boost: false,
            strict_mode: false,
            image_format: "png",
            i2i: null,
            controlnet: null,
            character_references: null,
            characters: null,
            use_coords: null,
          },
        },
      },
    });
  });
});
