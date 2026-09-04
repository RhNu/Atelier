/* eslint-disable max-lines-per-function */
import {
  buildSubmitGenerationBatchRequest,
  createGenerationDraft,
  generationDraftFromDto,
  generationDraftToDto,
  switchGenerationModel,
} from "../features/generation/model/generation-draft";
import type { ModelCapabilitiesDto } from "../types";
import type { WorkspaceSettingsDto } from "../types";

function modelCapabilities(defaultScale: number, supportsStreaming = true): ModelCapabilitiesDto {
  return {
    prompt_structure: "v4",
    params_version: 4,
    default_steps: 23,
    default_scale: defaultScale,
    max_characters: 6,
    character_position_mode: "freeform",
    can_position_one_character: true,
    supports_vibe_transfer: false,
    supports_encoded_vibe: false,
    supports_character_reference: false,
    supports_character_reference_inpainting: false,
    supports_variety_boost: false,
    supports_inpainting: true,
    supports_streaming: supportsStreaming,
    supports_smea: false,
    supports_dynamic_thresholding: false,
    uses_v5_extensions: true,
    has_opus_usage_limit: true,
    supports_light_quality_preset: true,
    supports_transparent_background: true,
    variety_sigma_coefficient: null,
    prompt_token_limit: 1471,
  };
}

const settings: WorkspaceSettingsDto = {
  generation: {
    model: "nai-diffusion-4-5-full",
    size: { width: 832, height: 1216 },
    quality: "standard",
    transparent_background: false,
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
  it("restores per-model prompt state and only changes the official scale", () => {
    const v5Capabilities = modelCapabilities(7);
    const original = { ...createGenerationDraft(settings), prompt: "v4 prompt", steps: 31 };
    const v5 = switchGenerationModel(original, "nai-diffusion-5-full", v5Capabilities);
    expect(v5).toMatchObject({ prompt: "", scale: 7, steps: 31 });
    const editedV5 = { ...v5, prompt: "自然语言 😀" };
    const restored = switchGenerationModel(
      editedV5,
      "nai-diffusion-4-5-full",
      modelCapabilities(5),
    );
    expect(restored).toMatchObject({ prompt: "v4 prompt", scale: 5, steps: 31 });
  });

  it("creates a draft from workspace generation settings without persisting UI-only state", () => {
    const draft = createGenerationDraft(settings);

    expect(draft).toMatchObject({
      prompt: "",
      mainPresetId: null,
      negativePrompt: "",
      model: "nai-diffusion-4-5-full",
      size: { width: 832, height: 1216 },
      quality: "standard",
      ucPreset: "light",
      steps: 23,
      scale: 5,
      sampler: "k_euler_ancestral",
      noiseSchedule: "karras",
      seedMode: "random",
      seed: 0,
      nSamples: 1,
      requestCount: 1,
      cfgRescale: 0,
      varietyBoost: false,
      imageFormat: null,
      strictMode: false,
      streamEnabled: true,
      i2i: null,
      vibe: { strength: 1, slots: [] },
      preciseReferences: [],
      characters: [],
      characterPositionMode: "global",
    });
  });

  it("derives Vibe activation from slots and Precise Reference precedence", () => {
    const draft = createGenerationDraft(settings);
    draft.vibe.slots.push({
      id: "slot-a",
      encoding: { id: "vibe-encoding", variant_id: null },
      vibeId: null,
      informationExtracted: 0.7,
      strength: 0.5,
      displayName: "Style",
      sourceImage: null,
      sourceSha256: null,
      model: "nai-diffusion-4-5-full",
    });

    const dto = generationDraftToDto(draft);
    expect(dto.vibe.enabled).toBe(true);
    dto.vibe.enabled = false;
    const restored = generationDraftFromDto(dto);
    const restoredRequest = buildSubmitGenerationBatchRequest(restored, {
      batchId: "batch-vibe",
      jobIds: ["job-vibe"],
    });
    expect(restoredRequest.jobs[0]?.work).toMatchObject({
      request: {
        base: { vibe_transfer: { references: [{ encoding: { id: "vibe-encoding" } }] } },
      },
    });

    draft.preciseReferences.push({
      id: "ref-a",
      image: { id: "reference-image", variant_id: null },
      referenceType: "character",
      fidelity: 0.5,
      strength: 0.6,
      displayName: "Reference 1",
    });
    expect(generationDraftToDto(draft).vibe.enabled).toBe(false);
  });

  it("builds a stream generation batch and keeps fixed seed stable across jobs", () => {
    const draft = {
      ...createGenerationDraft(settings),
      prompt: "1girl, atelier lighting",
      negativePrompt: "low quality",
      steps: 28,
      seedMode: "fixed" as const,
      seed: 1234,
      nSamples: 2,
      requestCount: 2,
      imageFormat: "png" as const,
    };

    const request = buildSubmitGenerationBatchRequest(draft, {
      batchId: "batch-test",
      jobIds: ["job-a", "job-b"],
    });

    expect(request.batch_id).toBe("batch-test");
    expect(request.jobs[0]).toEqual({
      job_id: "job-a",
      work: {
        kind: "stream",
        request: {
          stream: "sse",
          base: {
            prompt: "1girl, atelier lighting",
            main_preset_id: null,
            negative_prompt: "low quality",
            model: "nai-diffusion-4-5-full" as const,
            size: { width: 832, height: 1216 },
            quality: "standard",
            transparent_background: false,
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
            img2img: null,
            vibe_transfer: null,
            character_references: null,
            characters: null,
            use_coords: null,
          },
        },
      },
    });
    expect(request.jobs[1]?.job_id).toBe("job-b");
    expect(request.context).toEqual({
      request_count: 2,
      pending_vibe_encode_count: 0,
      tier: 0,
      subscription_active: false,
      v5_usage_is_negative: false,
    });
    const secondBase =
      request.jobs[1]?.work.kind === "stream" ? request.jobs[1].work.request.base : null;
    expect(secondBase?.prompt).toBe("1girl, atelier lighting");
    expect(secondBase?.seed).toBe(1234);
  });

  it("uses normal generation when the selected model cannot stream", () => {
    const draft = {
      ...createGenerationDraft(settings),
      model: "nai-diffusion-3" as const,
      prompt: "1girl",
      streamEnabled: true,
    };

    const request = buildSubmitGenerationBatchRequest(
      draft,
      { batchId: "batch-v3", jobIds: ["job-v3"] },
      { capabilities: modelCapabilities(5, false) },
    );

    expect(request.jobs[0]?.work).toMatchObject({
      kind: "image",
      request: { model: "nai-diffusion-3" },
    });
  });

  it("builds resource-backed i2i, vibe, and character payloads", () => {
    const draft = {
      ...createGenerationDraft(settings),
      prompt: "1girl",
      negativePrompt: "low quality",
      i2i: {
        image: { id: "source-image", variant_id: null },
        inpaint: {
          regionToReplace: { id: "mask-image", variant_id: null },
          display: {
            color: "#2563eb",
            opacity: 0.45,
            pattern: "solid" as const,
            showBorder: true,
            brushSize: 48,
          },
          focus: null,
        },
        strength: 0.64,
        noise: 0.12,
      },
      vibe: {
        strength: 0.9,
        slots: [
          {
            id: "slot-a",
            encoding: { id: "vibe-encoding", variant_id: null },
            informationExtracted: 0.7,
            strength: 0.4,
            displayName: "vibe-a",
            sourceImage: null,
            sourceSha256: null,
            model: "nai-diffusion-4-5-full" as const,
          },
        ],
      },
      characters: [
        {
          id: "char-a",
          presetId: null,
          prompt: "hero",
          negativePrompt: "flat",
          enabled: true,
          position: { x: 0.5, y: 0.5 },
        },
      ],
      characterPositionMode: "manual" as const,
    };

    const request = buildSubmitGenerationBatchRequest(draft, {
      batchId: "batch-test",
      jobIds: ["job-test"],
    });

    const base = request.jobs[0]?.work.kind === "stream" ? request.jobs[0].work.request.base : null;
    expect(base).toMatchObject({
      img2img: {
        image: { kind: "resource_ref", resource: { id: "source-image", variant_id: null } },
        inpaint: {
          region_to_replace: {
            kind: "resource_ref",
            resource: { id: "mask-image", variant_id: null },
          },
        },
        strength: 0.64,
        noise: 0.12,
      },
      vibe_transfer: {
        strength: 0.9,
        references: [
          {
            encoding: { id: "vibe-encoding", variant_id: null },
            strength: 0.4,
          },
        ],
      },
      character_references: null,
      characters: [
        {
          preset_id: null,
          prompt: "hero",
          negative_prompt: "flat",
          enabled: true,
          position: { x: 0.5, y: 0.5 },
        },
      ],
      use_coords: false,
    });
  });

  it("omits disabled or blank character rows from submit payloads", () => {
    const draft = {
      ...createGenerationDraft(settings),
      prompt: "1girl",
      characters: [
        {
          id: "char-empty",
          presetId: null,
          prompt: "",
          negativePrompt: "",
          enabled: true,
          position: { x: 0.1, y: 0.2 },
        },
        {
          id: "char-disabled",
          presetId: null,
          prompt: "disabled character",
          negativePrompt: "",
          enabled: false,
          position: { x: 0.3, y: 0.4 },
        },
      ],
      characterPositionMode: "manual" as const,
    };

    const request = buildSubmitGenerationBatchRequest(draft, {
      batchId: "batch-test",
      jobIds: ["job-test"],
    });

    const base = request.jobs[0]?.work.kind === "stream" ? request.jobs[0].work.request.base : null;
    expect(base?.characters).toBeNull();
    expect(base?.use_coords).toBeNull();
  });

  it("carries prompt preset bindings without rewriting draft text", () => {
    const draft = {
      ...createGenerationDraft(settings),
      mainPresetId: "main-preset",
      prompt: "$chunk(hero), 1girl",
      characters: [
        {
          id: "char-preset",
          presetId: "character-preset",
          prompt: "hero",
          negativePrompt: "",
          enabled: true,
          position: { x: 0.5, y: 0.5 },
        },
      ],
    };

    const request = buildSubmitGenerationBatchRequest(draft, {
      batchId: "batch-test",
      jobIds: ["job-test"],
    });

    const base = request.jobs[0]?.work.kind === "stream" ? request.jobs[0].work.request.base : null;
    expect(base).toMatchObject({
      main_preset_id: "main-preset",
      prompt: "$chunk(hero), 1girl",
      characters: [
        {
          preset_id: "character-preset",
          prompt: "hero",
        },
      ],
    });
    expect(draft.prompt).toBe("$chunk(hero), 1girl");
    expect(draft.characters[0]?.prompt).toBe("hero");
  });

  it("prefers precise references over vibe controlnet because NovelAI forbids both together", () => {
    const draft = {
      ...createGenerationDraft(settings),
      prompt: "1girl",
      vibe: {
        strength: 1,
        slots: [
          {
            id: "slot-a",
            encoding: { id: "vibe-encoding", variant_id: null },
            informationExtracted: 1,
            strength: 1,
            displayName: "vibe-a",
            sourceImage: null,
            sourceSha256: null,
            model: "nai-diffusion-4-5-full" as const,
          },
        ],
      },
      preciseReferences: [
        {
          id: "ref-a",
          image: { id: "ref-image", variant_id: null },
          referenceType: "character_and_style" as const,
          fidelity: 0.5,
          strength: 0.6,
          displayName: "ref-a",
        },
      ],
    };

    const request = buildSubmitGenerationBatchRequest(draft, {
      batchId: "batch-test",
      jobIds: ["job-test"],
    });

    const base = request.jobs[0]?.work.kind === "stream" ? request.jobs[0].work.request.base : null;
    expect(base?.vibe_transfer).toBeNull();
    expect(base?.character_references).toEqual([
      {
        image: { kind: "resource_ref", resource: { id: "ref-image", variant_id: null } },
        reference_type: "character_and_style",
        fidelity: 0.5,
        strength: 0.6,
      },
    ]);
  });

  it("forces focused inpainting onto the final-image path", () => {
    const draft = createGenerationDraft(settings);
    draft.i2i = {
      image: { id: "source", variant_id: null },
      strength: 1,
      noise: 0,
      inpaint: {
        regionToReplace: { id: "mask", variant_id: null },
        display: {
          color: "#2563eb",
          opacity: 0.45,
          pattern: "solid",
          showBorder: true,
          brushSize: 48,
        },
        focus: { x: 0.25, y: 0.25, width: 0.5, height: 0.5, minimumContextArea: 0.25 },
      },
    };
    const request = buildSubmitGenerationBatchRequest(
      draft,
      { batchId: "batch-focus", jobIds: ["job-focus"] },
      { capabilities: modelCapabilities(5) },
    );
    expect(request.jobs[0]?.work.kind).toBe("image");
  });
});
