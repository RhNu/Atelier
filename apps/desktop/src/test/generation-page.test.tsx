/* eslint-disable max-lines, max-lines-per-function */
import { currentCompletions } from "@codemirror/autocomplete";
import { QueryClientProvider } from "@tanstack/react-query";
import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { createAtelierQueryClient } from "../app/query-client";
import { AppToastHost } from "../components/ui";
import { GeneratePage } from "../features/generation";
import {
  recordGenerationEvent,
  resetGenerationEventState,
} from "../features/generation/state/generation-event-store";
import type {
  AppEventDto,
  CompileGenerationPromptRequestDto,
  CompiledGenerationPromptDto,
  CountPromptTokensRequestDto,
  DeleteGenerationHistoryBatchesRequestDto,
  DeleteGenerationHistoryBatchesResponseDto,
  DeleteRunHistoryItemsRequestDto,
  DeleteRunHistoryItemsResponseDto,
  GalleryImageReferenceRequestDto,
  GalleryImageReferenceDto,
  GlobalSettingsDto,
  GenerationDraftDto,
  GenerationHistoryBatchDetailDto,
  GenerationHistoryBatchDto,
  GenerationHistoryBatchRequestDto,
  GenerationHistoryPageDto,
  GenerationHistoryQueryDto,
  GenerationAnlasEstimateDto,
  GenerationEstimateRequestDto,
  GenerationStatusDto,
  GenerationStatusQueryDto,
  GetResourceImageRequestDto,
  ImportImageResourceResponseDto,
  ImageResourceKindDto,
  ImageModelDescriptorDto,
  ImageModelDto,
  ImportedVibeDocumentsDto,
  ListVibeDocumentsRequestDto,
  ListPromptPresetsRequestDto,
  ListPromptChunksRequestDto,
  LexiconCompleteRequestDto,
  LexiconSearchItemDto,
  QueueDirectiveDto,
  PromptChunkPageDto,
  PromptPresetPageDto,
  PromptTokenUsageDto,
  ResourceImageDto,
  ReleaseImportedImageResourcesRequestDto,
  ReleaseImportedImageResourcesResponseDto,
  ResourceRefDto,
  RerunGenerationHistoryItemRequestDto,
  RerunGenerationHistoryItemResponseDto,
  RerunGenerationHistoryBatchRequestDto,
  RerunGenerationHistoryBatchResponseDto,
  RunHistoryPageDto,
  RunHistoryQueryDto,
  RunHistoryOutputDto,
  SaveResourceImageRequestDto,
  SaveResourceImagesZipRequestDto,
  SaveGenerationDraftRequestDto,
  SubmitGenerationBatchRequestDto,
  SubscriptionSummaryDto,
  EnsureVibeEncodingRequestDto,
  EnsuredVibeEncodingDto,
  VibeDocumentPageDto,
  WorkspaceSettingsDto,
} from "../types";
import {
  acceptPromptCompletion,
  clearPromptEditor,
  closePromptCompletion,
  promptEditorText,
  promptEditorView,
  startPromptCompletion,
  typeInPromptEditor,
  undoPromptEditor,
} from "./prompt-editor-test-utils";

const mocks = vi.hoisted(() => ({
  generationApi: {
    countPromptTokens:
      vi.fn<(request: CountPromptTokensRequestDto) => Promise<PromptTokenUsageDto>>(),
    listModels: vi.fn<() => Promise<ImageModelDescriptorDto[]>>(),
    getDraft: vi.fn<() => Promise<GenerationDraftDto | null>>(),
    saveDraft: vi.fn<(request: SaveGenerationDraftRequestDto) => Promise<GenerationDraftDto>>(),
    clearDraft: vi.fn<() => Promise<void>>(),
    submitBatch: vi.fn<(request: SubmitGenerationBatchRequestDto) => Promise<QueueDirectiveDto>>(),
    estimate:
      vi.fn<(request: GenerationEstimateRequestDto) => Promise<GenerationAnlasEstimateDto>>(),
    pause: vi.fn<() => Promise<QueueDirectiveDto>>(),
    resume: vi.fn<() => Promise<QueueDirectiveDto>>(),
    stop: vi.fn<() => Promise<QueueDirectiveDto>>(),
    status: vi.fn<(request: GenerationStatusQueryDto) => Promise<GenerationStatusDto>>(),
  },
  historyApi: {
    list: vi.fn<(request: RunHistoryQueryDto) => Promise<RunHistoryPageDto>>(),
    deleteItems:
      vi.fn<
        (request: DeleteRunHistoryItemsRequestDto) => Promise<DeleteRunHistoryItemsResponseDto>
      >(),
    rerunGeneration:
      vi.fn<
        (
          request: RerunGenerationHistoryItemRequestDto,
        ) => Promise<RerunGenerationHistoryItemResponseDto>
      >(),
    listGenerationBatches:
      vi.fn<(request: GenerationHistoryQueryDto) => Promise<GenerationHistoryPageDto>>(),
    getGenerationBatch:
      vi.fn<
        (request: GenerationHistoryBatchRequestDto) => Promise<GenerationHistoryBatchDetailDto>
      >(),
    deleteGenerationBatches:
      vi.fn<
        (
          request: DeleteGenerationHistoryBatchesRequestDto,
        ) => Promise<DeleteGenerationHistoryBatchesResponseDto>
      >(),
    rerunGenerationBatch:
      vi.fn<
        (
          request: RerunGenerationHistoryBatchRequestDto,
        ) => Promise<RerunGenerationHistoryBatchResponseDto>
      >(),
  },
  promptApi: {
    compileGenerationPreview:
      vi.fn<(request: CompileGenerationPromptRequestDto) => Promise<CompiledGenerationPromptDto>>(),
    listChunks: vi.fn<(request: ListPromptChunksRequestDto) => Promise<PromptChunkPageDto>>(),
    listPresets: vi.fn<(request: ListPromptPresetsRequestDto) => Promise<PromptPresetPageDto>>(),
  },
  lexiconApi: {
    complete: vi.fn<(request: LexiconCompleteRequestDto) => Promise<LexiconSearchItemDto[]>>(),
  },
  resourceApi: {
    image: vi.fn<(request: GetResourceImageRequestDto) => Promise<ResourceImageDto>>(),
    releaseImportedImages:
      vi.fn<
        (
          request: ReleaseImportedImageResourcesRequestDto,
        ) => Promise<ReleaseImportedImageResourcesResponseDto>
      >(),
  },
  settingsApi: {
    get: vi.fn<() => Promise<WorkspaceSettingsDto>>(),
  },
  globalSettingsApi: {
    get: vi.fn<() => Promise<GlobalSettingsDto>>(),
  },
  galleryApi: {
    imageReference:
      vi.fn<(request: GalleryImageReferenceRequestDto) => Promise<GalleryImageReferenceDto>>(),
  },
  accountApi: {
    probeActive: vi.fn<() => Promise<SubscriptionSummaryDto>>(),
  },
  desktopApi: {
    pickAndImportImageResources:
      vi.fn<
        (
          role: ImageResourceKindDto,
          options?: { extensions?: string[] },
        ) => Promise<ImportImageResourceResponseDto[]>
      >(),
    saveResourceImage:
      vi.fn<(request: SaveResourceImageRequestDto) => Promise<{ path: string } | null>>(),
    saveResourceImagesZip:
      vi.fn<
        (
          request: SaveResourceImagesZipRequestDto,
        ) => Promise<{ path: string; exported: number } | null>
      >(),
    pickAndImportVibeDocuments:
      vi.fn<(options?: { extensions?: string[] }) => Promise<ImportedVibeDocumentsDto>>(),
  },
  vibeApi: {
    listDocuments: vi.fn<(request: ListVibeDocumentsRequestDto) => Promise<VibeDocumentPageDto>>(),
    ensureEncoding:
      vi.fn<(request: EnsureVibeEncodingRequestDto) => Promise<EnsuredVibeEncodingDto>>(),
    saveDocument: vi.fn<(request: unknown) => Promise<{ path: string } | null>>(),
  },
}));

vi.mock("../platform/atelier", () => ({
  generationApi: mocks.generationApi,
  historyApi: mocks.historyApi,
  promptApi: mocks.promptApi,
  lexiconApi: mocks.lexiconApi,
  resourceApi: mocks.resourceApi,
  settingsApi: mocks.settingsApi,
  globalSettingsApi: mocks.globalSettingsApi,
  galleryApi: mocks.galleryApi,
  accountApi: mocks.accountApi,
  desktopApi: mocks.desktopApi,
  vibeApi: mocks.vibeApi,
  queryKeys: {
    generation: {
      root: () => ["generation"],
      draft: () => ["generation", "draft"],
      status: (jobId?: string | null) => ["generation", "status", jobId ?? null],
      estimate: (request: unknown) => ["generation", "estimate", request],
    },
    history: {
      root: () => ["history"],
      list: (query: RunHistoryQueryDto) => ["history", "list", query],
      generationBatches: (query: GenerationHistoryQueryDto) => ["history", "batches", query],
      generationBatch: (batchId: string | null) => ["history", "batch", batchId],
    },
    settings: {
      workspace: () => ["settings", "workspace"],
    },
    app: {
      globalSettings: () => ["app", "global-settings"],
    },
    prompt: {
      root: () => ["prompt"],
      chunks: (query?: unknown) =>
        query === undefined ? ["prompt", "chunks"] : ["prompt", "chunks", query],
      presets: (query: ListPromptPresetsRequestDto) => ["prompt", "presets", query],
    },
    lexicon: {
      completion: (query: string, limit: number) => ["lexicon", "completion", query, limit],
    },
    resource: {
      root: () => ["resource"],
      image: (resource: { id: string; variant_id: string | null }) => [
        "resource",
        "image",
        resource,
      ],
    },
    gallery: {
      root: () => ["gallery"],
    },
    account: {
      activeSummary: () => ["account", "active-summary"],
    },
    vibe: {
      root: () => ["vibe"],
      list: (query: ListVibeDocumentsRequestDto) => ["vibe", "list", query],
    },
  },
  uniqueImportedImageResources: (resources: ReadonlyArray<ResourceRefDto | null>) =>
    resources.filter(
      (resource): resource is ResourceRefDto =>
        resource?.variant_id === null && resource.id.startsWith("resource:import:"),
    ),
  resourceImageToDataUrl: (image: ResourceImageDto) =>
    `data:${image.mime_type ?? "image/png"};base64,${image.image_base64}`,
}));

const defaultSettings: WorkspaceSettingsDto = {
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

const defaultGlobalSettings: GlobalSettingsDto = {
  last_workspace: "D:/atelier",
  frontend: {
    language: "system",
    developer_mode: false,
    convert_full_width_punctuation: false,
    gallery: { blur_sensitive_images: false },
  },
  safety: { wd_auto_review_enabled: false },
};

const imageModelRows: ReadonlyArray<readonly [ImageModelDto, number, boolean]> = [
  ["nai-diffusion-5-full", 7, true],
  ["nai-diffusion-5-curated", 7, true],
  ["nai-diffusion-4-5-full", 5, false],
  ["nai-diffusion-4-5-curated", 5, false],
  ["nai-diffusion-4-full", 5.5, false],
  ["nai-diffusion-4-curated", 5.5, false],
  ["nai-diffusion-3", 5, false],
  ["nai-diffusion-furry-3", 6.2, false],
];

const imageModelCatalog: ImageModelDescriptorDto[] = imageModelRows.map(
  ([model, defaultScale, isV5]) => ({
    model,
    capabilities: {
      prompt_structure:
        model === "nai-diffusion-3" || model === "nai-diffusion-furry-3" ? "legacy" : "v4",
      params_version: isV5 ? 4 : 3,
      default_steps: 23,
      default_scale: defaultScale,
      max_characters: model === "nai-diffusion-3" || model === "nai-diffusion-furry-3" ? 0 : 6,
      character_position_mode:
        model === "nai-diffusion-3" || model === "nai-diffusion-furry-3"
          ? null
          : isV5
            ? "freeform"
            : "grid_5x5",
      can_position_one_character: isV5,
      supports_vibe_transfer: !isV5,
      supports_encoded_vibe:
        !isV5 && model !== "nai-diffusion-3" && model !== "nai-diffusion-furry-3",
      supports_character_reference: !isV5 && String(model).includes("4-5"),
      supports_character_reference_inpainting: !isV5 && String(model).includes("4-5"),
      supports_variety_boost: !isV5 && String(model).includes("diffusion-4"),
      supports_inpainting: true,
      supports_furry_mode: model !== "nai-diffusion-3" && model !== "nai-diffusion-furry-3",
      supports_streaming: model !== "nai-diffusion-3" && model !== "nai-diffusion-furry-3",
      supports_smea: model === "nai-diffusion-3" || model === "nai-diffusion-furry-3",
      supports_dynamic_thresholding:
        model === "nai-diffusion-3" || model === "nai-diffusion-furry-3",
      uses_v5_extensions: Boolean(isV5),
      has_opus_usage_limit: Boolean(isV5),
      supports_light_quality_preset: Boolean(isV5),
      supports_transparent_background: Boolean(isV5),
      variety_sigma_coefficient: null,
      prompt_token_limit: isV5 ? 1471 : 512,
    },
  }),
);

function opusSubscription(
  v5Usage: Omit<NonNullable<SubscriptionSummaryDto["v5_usage"]>, "is_negative">,
) {
  return {
    anlas_balance: 10_000,
    is_opus: true,
    subscription_active: true,
    tier: 3,
    tier_name: "Opus",
    expires_at_ms: null,
    v5_usage: { is_negative: false, ...v5Usage },
  } satisfies SubscriptionSummaryDto;
}

function appEvent(kind: AppEventDto["kind"], sequence = 1): AppEventDto {
  return { sequence, kind };
}

function setup(options?: {
  status?: GenerationStatusDto;
  statusError?: Error;
  history?: GenerationHistoryPageDto;
  historyDetail?: GenerationHistoryBatchDetailDto;
  vibeDocuments?: VibeDocumentPageDto;
  settingsError?: Error;
  storedDraft?: GenerationDraftDto;
  draftError?: Error;
  mainPresets?: PromptPresetPageDto;
  characterPresets?: PromptPresetPageDto;
  developerMode?: boolean;
  convertFullWidthPunctuation?: boolean;
  model?: ImageModelDto;
  subscription?: SubscriptionSummaryDto;
}) {
  mocks.generationApi.listModels.mockResolvedValue(imageModelCatalog);
  mocks.generationApi.countPromptTokens.mockResolvedValue({
    prompt: { used: 3, limit: 512 },
    negative_prompt: { used: 4, limit: 512 },
    characters: [],
  });
  if (options?.settingsError) {
    mocks.settingsApi.get.mockRejectedValue(options.settingsError);
  } else {
    const settings = structuredClone(defaultSettings) as WorkspaceSettingsDto;
    if (options?.model) settings.generation.model = options.model;
    mocks.settingsApi.get.mockResolvedValue(settings);
  }
  mocks.globalSettingsApi.get.mockResolvedValue({
    ...defaultGlobalSettings,
    frontend: {
      ...defaultGlobalSettings.frontend,
      developer_mode: options?.developerMode ?? false,
      convert_full_width_punctuation: options?.convertFullWidthPunctuation ?? false,
    },
  });
  if (options?.statusError) {
    mocks.generationApi.status.mockRejectedValue(options.statusError);
  } else {
    mocks.generationApi.status.mockResolvedValue(
      options?.status ?? {
        batch_id: null,
        batch_status: null,
        current_job_id: null,
        job_status: null,
        requests: [],
      },
    );
  }
  mocks.generationApi.submitBatch.mockResolvedValue({ kind: "start_job", job_id: "job-submitted" });
  if (options?.draftError) {
    mocks.generationApi.getDraft.mockRejectedValue(options.draftError);
  } else {
    mocks.generationApi.getDraft.mockResolvedValue(options?.storedDraft ?? null);
  }
  mocks.generationApi.saveDraft.mockImplementation(async (request) => request.draft);
  mocks.generationApi.clearDraft.mockResolvedValue();
  mocks.generationApi.estimate.mockResolvedValue({
    status: "available",
    per_image_cost: 3,
    per_request_cost: 3,
    request_count: 1,
    generation_cost: 3,
    character_reference_cost: 0,
    vibe_reference_overage_cost: 0,
    pending_encode_cost: 0,
    total_cost: 3,
    requested_samples: 1,
    sample_limit: 4,
    priced_samples: 1,
    billable_samples: 1,
    free_first_image_applied: false,
  });
  mocks.generationApi.pause.mockResolvedValue({ kind: "paused" });
  mocks.generationApi.resume.mockResolvedValue({ kind: "start_job", job_id: "job-submitted" });
  mocks.generationApi.stop.mockResolvedValue({ kind: "idle" });
  mocks.historyApi.list.mockResolvedValue({
    items: [],
    offset: 0,
    limit: 8,
    total: 0,
  });
  mocks.historyApi.listGenerationBatches.mockResolvedValue(
    options?.history ?? { items: [], offset: 0, limit: 8, total: 0 },
  );
  mocks.historyApi.getGenerationBatch.mockImplementation(async ({ batch_id }) => {
    if (options?.historyDetail) return options.historyDetail;
    return emptyBatchDetail(batch_id);
  });
  mocks.historyApi.deleteItems.mockResolvedValue({ deleted: 1 });
  mocks.historyApi.deleteGenerationBatches.mockResolvedValue({ deleted_requests: 1 });
  mocks.historyApi.rerunGeneration.mockResolvedValue({
    directive: { kind: "start_job", job_id: "job-rerun" },
    item: {
      run_id: "job-rerun",
      kind: "generation",
      status: "queued",
      batch_id: "batch-rerun",
      job_id: "job-rerun",
      origin_run_id: "job-1",
      title: "1girl",
      last_error: null,
      created_at_ms: 3,
      updated_at_ms: 3,
      completed_at_ms: null,
      recoverable: false,
      outputs: [],
    },
  });
  mocks.historyApi.rerunGenerationBatch.mockImplementation(async (request) => ({
    directive: { kind: "start_job", job_id: request.job_ids[0] ?? "job-rerun" },
    batch: {
      ...emptyBatchDetail(request.batch_id).batch,
      request_count: request.job_ids.length,
      expected_sample_count: request.job_ids.length,
    },
  }));
  mocks.promptApi.compileGenerationPreview.mockResolvedValue({
    prompt: {
      expanded_prompt: "expanded prompt",
      trace: {
        raw_prompt: "raw prompt",
        expanded_prompt: "expanded prompt",
        function_calls: [],
      },
    },
    negative_prompt: {
      expanded_prompt: "expanded negative",
      trace: {
        raw_prompt: "raw negative",
        expanded_prompt: "expanded negative",
        function_calls: [],
      },
    },
    characters: [],
    quality_override: null,
    uc_preset_override: null,
  });
  mocks.promptApi.listChunks.mockResolvedValue({
    items: [
      {
        chunk_id: "chunk-lighting",
        key: "lighting",
        content: "cinematic lighting, rim light",
        category: "Lighting",
        description: "Reusable lighting stack",
        preview: null,
        created_at_ms: 1,
        updated_at_ms: 1,
        models: ["nai-diffusion-4-5-full"],
      },
      {
        chunk_id: "chunk-hero",
        key: "hero",
        content: "solo, looking at viewer",
        category: "Subject",
        description: "Main character setup",
        preview: null,
        created_at_ms: 2,
        updated_at_ms: 2,
        models: ["nai-diffusion-4-5-full"],
      },
    ],
    total: 2,
    offset: 0,
    limit: 200,
  });
  mocks.promptApi.listPresets.mockImplementation(async (request) => {
    if (request.kind === "main") {
      return options?.mainPresets ?? emptyPresetPage();
    }
    if (request.kind === "character") {
      return options?.characterPresets ?? emptyPresetPage();
    }
    return emptyPresetPage();
  });
  mocks.lexiconApi.complete.mockImplementation(async (request) =>
    request.query.trim().length > 0
      ? [
          {
            entity_id: 1,
            canonical_name: "cinematic_lighting",
            primary_translation: "cinematic lighting",
            kind: "tag",
            category: "general",
            post_count: 1200,
            rating: "safe",
            matched_text: request.query,
            match_reason: "canonical_prefix",
            score: 97,
          },
        ]
      : [],
  );
  mocks.resourceApi.image.mockResolvedValue({
    image_base64: "final-image",
    mime_type: "image/png",
  });
  mocks.resourceApi.releaseImportedImages.mockResolvedValue({
    released: 1,
    resources_deleted: 1,
    blobs_deleted: 1,
  });
  mocks.galleryApi.imageReference.mockResolvedValue({
    item_id: "gallery-1",
    artifact_id: "artifact-1",
    target: "director",
    resource: { id: "resource:generated:job-1:0", variant_id: null },
    asset_role: "primary",
    variant_kind: null,
  });
  mocks.desktopApi.pickAndImportImageResources.mockResolvedValue([]);
  mocks.desktopApi.saveResourceImage.mockResolvedValue({ path: "C:\\exports\\job-1.png" });
  mocks.desktopApi.saveResourceImagesZip.mockResolvedValue({
    path: "C:\\exports\\batch.zip",
    exported: 1,
  });
  mocks.desktopApi.pickAndImportVibeDocuments.mockResolvedValue({ entries: [] });
  mocks.vibeApi.listDocuments.mockResolvedValue(
    options?.vibeDocuments ?? {
      items: [],
      total: 0,
      offset: 0,
      limit: 32,
    },
  );
  mocks.vibeApi.ensureEncoding.mockResolvedValue({
    resource: { id: "vibe-encoding:source-image", variant_id: null },
    created: true,
  });
  mocks.vibeApi.saveDocument.mockResolvedValue({ path: "C:\\exports\\style.naiv4vibe" });
  mocks.accountApi.probeActive.mockResolvedValue(
    options?.subscription ?? {
      anlas_balance: 100,
      is_opus: false,
      subscription_active: true,
      tier: 1,
      tier_name: "Tablet",
      expires_at_ms: null,
      v5_usage: null,
    },
  );

  return {
    user: userEvent.setup(),
    ...render(
      <QueryClientProvider client={createAtelierQueryClient()}>
        <GeneratePage />
        <AppToastHost />
      </QueryClientProvider>,
    ),
  };
}

function emptyBatchDetail(batchId: string): GenerationHistoryBatchDetailDto {
  return {
    batch: {
      batch_id: batchId,
      status: "queued",
      title: null,
      last_error: null,
      created_at_ms: 1,
      updated_at_ms: 1,
      completed_at_ms: null,
      request_count: 0,
      completed_request_count: 0,
      expected_sample_count: 0,
      completed_sample_count: 0,
      available_sample_count: 0,
      outputs: [],
    },
    requests: [],
  };
}

function generationBatchFixture(): {
  page: GenerationHistoryPageDto;
  detail: GenerationHistoryBatchDetailDto;
} {
  const output: RunHistoryOutputDto = {
    sample_index: 0,
    artifact_id: "artifact-1",
    item_id: "gallery-1",
    resource: { id: "resource:generated:job-1:0", variant_id: null },
    asset_role: "primary",
    variant_kind: null,
    state: "available",
  };
  const batch: GenerationHistoryBatchDto = {
    batch_id: "batch-1",
    status: "succeeded",
    title: "1girl",
    last_error: null,
    created_at_ms: 1,
    updated_at_ms: 2,
    completed_at_ms: 2,
    request_count: 1,
    completed_request_count: 1,
    expected_sample_count: 1,
    completed_sample_count: 1,
    available_sample_count: 1,
    outputs: [output],
  };
  return {
    page: { items: [batch], offset: 0, limit: 8, total: 1 },
    detail: {
      batch,
      requests: [
        {
          run_id: "job-1",
          job_id: "job-1",
          origin_run_id: null,
          request_index: 0,
          expected_samples: 1,
          status: "succeeded",
          title: "1girl",
          last_error: null,
          created_at_ms: 1,
          updated_at_ms: 2,
          completed_at_ms: 2,
          outputs: [output],
        },
      ],
    },
  };
}

function emptyPresetPage(): PromptPresetPageDto {
  return {
    items: [],
    total: 0,
    offset: 0,
    limit: 200,
  };
}

function storedDraft(overrides: Partial<GenerationDraftDto> = {}): GenerationDraftDto {
  return {
    model: defaultSettings.generation.model,
    prompt_states: [
      {
        model: defaultSettings.generation.model,
        main_preset_id: null,
        prompt: "restored prompt",
        negative_prompt: "restored negative",
        furry_mode: false,
        characters: [],
        character_position_mode: "global",
      },
    ],
    size: { ...defaultSettings.generation.size },
    quality: defaultSettings.generation.quality,
    transparent_background: false,
    uc_preset: defaultSettings.generation.uc_preset,
    steps: defaultSettings.generation.steps,
    scale: defaultSettings.generation.scale,
    sampler: defaultSettings.generation.sampler,
    noise_schedule: defaultSettings.generation.noise_schedule,
    seed_mode: "random",
    seed: defaultSettings.generation.seed,
    n_samples: defaultSettings.generation.n_samples,
    request_count: 1,
    cfg_rescale: defaultSettings.generation.cfg_rescale,
    variety_boost: defaultSettings.generation.variety_boost,
    image_format: defaultSettings.generation.image_format,
    strict_mode: defaultSettings.generation.strict_mode,
    stream_enabled: true,
    i2i: null,
    vibe: { enabled: false, strength: 1, slots: [] },
    precise_references: [],
    ...overrides,
  };
}

beforeEach(() => {
  vi.restoreAllMocks();
  vi.clearAllMocks();
  window.localStorage.clear();
  vi.spyOn(crypto, "randomUUID")
    .mockReturnValueOnce("00000000-0000-4000-8000-0000000000aa")
    .mockReturnValueOnce("00000000-0000-4000-8000-0000000000bb");
  resetGenerationEventState();
});

describe("GeneratePage", () => {
  it("shows settings errors instead of false loading states", async () => {
    setup({ settingsError: new Error("settings db missing") });

    expect(
      await screen.findByText("Generation settings unavailable", undefined, {
        timeout: 4_000,
      }),
    ).toBeInTheDocument();
    expect(screen.getByText("settings db missing")).toBeInTheDocument();
  });

  it("shows status errors instead of false idle states", async () => {
    setup({ statusError: new Error("Queue offline") });

    expect(
      await screen.findByText("Queue offline", undefined, {
        timeout: 4_000,
      }),
    ).toBeInTheDocument();
  });

  it("hydrates the generation draft from workspace settings", async () => {
    const { user } = setup();

    expect(await screen.findByDisplayValue("832")).toBeInTheDocument();
    expect(screen.getByDisplayValue("1216")).toBeInTheDocument();
    expect(screen.getByLabelText("Model")).toHaveValue("nai-diffusion-4-5-full");
    await user.click(screen.getByLabelText("Size preset"));
    expect(screen.getByRole("group", { name: "Normal" })).toBeInTheDocument();
    expect(screen.getByRole("option", { name: "Portrait (832×1216)" })).toBeInTheDocument();
    expect(screen.getByRole("combobox", { name: "Size preset" }).parentElement).toHaveClass(
      "!w-40",
      "shrink-0",
    );
    expect(screen.getByTestId("generation-settings-sidebar")).toHaveStyle({ width: "360px" });
    await user.click(screen.getByRole("button", { name: /Steps 23/u }));
    expect(screen.getByLabelText("Steps")).toHaveValue("23");
    expect(screen.getByLabelText("Scale")).toHaveValue("5");
    expect(screen.getByLabelText("Sampler")).toHaveValue("k_euler_ancestral");
  });

  it("shows the Opus generation allowance for an active Opus account on a metered V5 model", async () => {
    setup({
      model: "nai-diffusion-5-full",
      subscription: opusSubscription({ percent: 100, seconds_until_next_percent: 7888 }),
    });

    expect(await screen.findByText(/Opus generations/u)).toBeInTheDocument();
    expect(screen.getByText("100%")).toBeInTheDocument();
  });

  it("hides the Opus generation allowance for a V4.5 model", async () => {
    setup({
      model: "nai-diffusion-4-5-full",
      subscription: opusSubscription({ percent: 100, seconds_until_next_percent: 7888 }),
    });

    expect(await screen.findByLabelText("Model")).toHaveValue("nai-diffusion-4-5-full");
    expect(screen.queryByText(/Opus generations/u)).not.toBeInTheDocument();
  });

  it("hides the Opus generation allowance for a non-Opus account on V5", async () => {
    setup({ model: "nai-diffusion-5-full" });

    expect(await screen.findByLabelText("Model")).toHaveValue("nai-diffusion-5-full");
    expect(screen.queryByText(/Opus generations/u)).not.toBeInTheDocument();
  });

  it("warns about per-image Anlas cost when the V5 allowance is nearly exhausted", async () => {
    setup({
      model: "nai-diffusion-5-full",
      subscription: opusSubscription({ percent: 2, seconds_until_next_percent: 7888 }),
    });

    expect(
      await screen.findByText("3 Anlas per image once the allowance runs out"),
    ).toBeInTheDocument();
    expect(screen.getByText("2% · ~215h")).toBeInTheDocument();
  });

  it("keeps empty guidance sections compact and reveals character positions only when useful", async () => {
    const { user } = setup();

    expect(await screen.findByRole("button", { name: "Add I2I source" })).toBeEnabled();
    for (const title of ["Image to image", "Vibe transfer", "Precise reference", "Characters"]) {
      const heading = screen.getByRole("heading", { name: title });
      expect(heading.closest("section")).not.toHaveClass("border", "bg-app-surface/30");
      expect(heading.closest("header")).not.toHaveClass("border-b");
    }
    expect(screen.queryByRole("button", { name: "Remove I2I source" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /I2I mask/u })).not.toBeInTheDocument();
    expect(screen.queryByLabelText("Vibe strength")).not.toBeInTheDocument();
    expect(screen.queryByLabelText("Use AI character positioning")).not.toBeInTheDocument();
    expect(screen.queryByText(/64–1600px/u)).not.toBeInTheDocument();
    expect(screen.queryByText(/Add source images, Vibe encodings/u)).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Add character prompt" }));
    expect(screen.queryByLabelText("Use AI character positioning")).not.toBeInTheDocument();
    typeInPromptEditor(screen.getByLabelText("Character 1 prompt"), "alice");
    await user.click(screen.getByRole("button", { name: "Add character prompt" }));
    typeInPromptEditor(screen.getByLabelText("Character 2 prompt"), "bob");
    const aiPositioning = await screen.findByLabelText("Use AI character positioning");
    expect(aiPositioning).toBeChecked();
    await user.click(aiPositioning);
    await user.click(await screen.findByRole("button", { name: "Open position editor" }));
    expect(
      await screen.findByRole("application", { name: "Character position canvas" }),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Disable character 1" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
  });

  it("uses bounded sliders only after an I2I source exists", async () => {
    const { user } = setup();
    mocks.desktopApi.pickAndImportImageResources.mockResolvedValueOnce([
      { resource: { id: "resource:import:source:1", variant_id: null } },
    ]);

    await user.click(await screen.findByRole("button", { name: "Add I2I source" }));
    const strength = await screen.findByRole("slider", { name: "Strength" });
    const noise = screen.getByRole("slider", { name: "Noise" });
    expect(strength).toHaveAttribute("min", "0.01");
    expect(strength).toHaveAttribute("max", "0.99");
    expect(strength).toHaveAttribute("step", "0.01");
    expect(noise).toHaveAttribute("min", "0");
    expect(noise).toHaveAttribute("max", "0.99");
    expect(screen.getByRole("button", { name: "Import mask" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Open Inpaint editor" })).toBeInTheDocument();
  });

  it("shows resource references only in developer mode", async () => {
    const normal = setup();
    mocks.desktopApi.pickAndImportImageResources.mockResolvedValueOnce([
      { resource: { id: "resource:import:normal:1", variant_id: null } },
    ]);
    await normal.user.click(await screen.findByRole("button", { name: "Add I2I source" }));
    expect(screen.queryByText(/resource:import:normal:1/u)).not.toBeInTheDocument();
    normal.unmount();

    const developer = setup({ developerMode: true });
    mocks.desktopApi.pickAndImportImageResources.mockResolvedValueOnce([
      { resource: { id: "resource:import:developer:1", variant_id: null } },
    ]);
    await developer.user.click(await screen.findByRole("button", { name: "Add I2I source" }));
    expect(await screen.findByText(/resource:import:developer:1/u)).toBeInTheDocument();
  });

  it("hydrates a persisted workspace draft and auto-saves the latest prompt", async () => {
    setup({ storedDraft: storedDraft() });

    const prompt = await screen.findByLabelText("Positive prompt");
    expect(promptEditorText(prompt)).toBe("restored prompt");
    typeInPromptEditor(prompt, ", detailed eyes");

    await waitFor(() => expect(mocks.generationApi.saveDraft).toHaveBeenCalled(), {
      timeout: 2_000,
    });
    const savedRequest = mocks.generationApi.saveDraft.mock.lastCall?.[0];
    expect(savedRequest?.draft.prompt_states[0]?.prompt).toBe("restored prompt, detailed eyes");
  });

  it("counts the complete generation prompt assembly once and displays the returned field", async () => {
    const draft = storedDraft({
      prompt_states: [
        {
          model: "nai-diffusion-4-5-full",
          main_preset_id: "preset-main",
          prompt: '1girl, $comment("draft note")',
          negative_prompt: "bad hands",
          furry_mode: false,
          characters: [
            {
              id: "character-1",
              preset_id: "preset-character",
              prompt: "$chunk(hero)",
              negative_prompt: "extra arms",
              enabled: true,
              position: { x: 0.5, y: 0.5 },
            },
          ],
          character_position_mode: "global",
        },
      ],
    });
    setup({ storedDraft: draft });
    mocks.generationApi.countPromptTokens.mockResolvedValue({
      prompt: { used: 13, limit: 512 },
      negative_prompt: { used: 9, limit: 512 },
      characters: [
        {
          index: 0,
          prompt: { used: 5, limit: 512 },
          negative_prompt: { used: 4, limit: 512 },
        },
      ],
    });

    expect(
      await screen.findByRole("progressbar", { name: "13 of 512 tokens" }),
    ).toBeInTheDocument();
    expect(mocks.generationApi.countPromptTokens).toHaveBeenCalledWith({
      compile: {
        model: "nai-diffusion-4-5-full",
        main_preset_id: "preset-main",
        prompt: '1girl, $comment("draft note")',
        negative_prompt: "bad hands",
        characters: [
          {
            preset_id: "preset-character",
            prompt: "$chunk(hero)",
            negative_prompt: "extra arms",
            enabled: true,
          },
        ],
        max_depth: 16,
      },
      quality: draft.quality,
      transparent_background: false,
      uc_preset: draft.uc_preset,
      furry_mode: false,
    });
  });

  it("converts full-width punctuation in generation prompts when enabled", async () => {
    setup({ convertFullWidthPunctuation: true });
    const prompt = await screen.findByLabelText("Positive prompt");

    typeInPromptEditor(prompt, "1girl，blue；sky。night");

    expect(promptEditorText(prompt)).toBe("1girl, blue, sky, night");
  });

  it("isolates undo history between positive and undesired prompts", async () => {
    const { user } = setup();
    typeInPromptEditor(await screen.findByLabelText("Positive prompt"), "positive edit");

    await user.click(screen.getByRole("tab", { name: "Undesired Content" }));
    const negative = screen.getByLabelText("Undesired Content");
    typeInPromptEditor(negative, "negative edit");
    expect(undoPromptEditor(negative)).toBe(true);
    expect(promptEditorText(negative)).toBe("");

    await user.click(screen.getByRole("tab", { name: "Positive" }));
    expect(promptEditorText(screen.getByLabelText("Positive prompt"))).toBe("positive edit");
  });

  it("surfaces non-blocking draft save failures and retries the latest draft", async () => {
    const { user } = setup({ storedDraft: storedDraft() });
    mocks.generationApi.saveDraft
      .mockRejectedValueOnce(new Error("draft database busy"))
      .mockImplementation(async (request) => request.draft);

    typeInPromptEditor(await screen.findByLabelText("Positive prompt"), ", retry me");
    expect(await screen.findByText("draft database busy")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Retry save" }));

    await waitFor(() => expect(mocks.generationApi.saveDraft.mock.calls.length).toBeGreaterThan(1));
    const retriedRequest = mocks.generationApi.saveDraft.mock.lastCall?.[0];
    expect(retriedRequest?.draft.prompt_states[0]?.prompt).toBe("restored prompt, retry me");
  });

  it("submits batch stream generation work from the current draft", async () => {
    const { user } = setup();

    typeInPromptEditor(await screen.findByLabelText("Positive prompt"), "1girl, atelier lighting");
    await user.click(screen.getByRole("tab", { name: "Undesired Content" }));
    typeInPromptEditor(screen.getByLabelText("Undesired Content"), "low quality");
    await user.click(screen.getByRole("button", { name: /Steps 23/u }));
    fireEvent.change(screen.getByLabelText("Steps"), { target: { value: "28" } });
    await user.click(screen.getByRole("button", { name: /^Generate 1 images/u }));

    await waitFor(() => expect(mocks.generationApi.submitBatch).toHaveBeenCalledTimes(1));
    const request = mocks.generationApi.submitBatch.mock.calls[0]?.[0];
    expect(request).toMatchObject({
      batch_id: "generation-00000000-0000-4000-8000-0000000000aa",
      context: {
        request_count: 1,
        pending_vibe_encode_count: 0,
        tier: 1,
        subscription_active: true,
        v5_usage_is_negative: false,
      },
      jobs: [
        {
          job_id: "job-00000000-0000-4000-8000-0000000000bb",
          work: {
            kind: "stream",
            request: {
              stream: "sse",
              base: {
                prompt: "1girl, atelier lighting",
                negative_prompt: "low quality",
                steps: 28,
                img2img: null,
                vibe_transfer: null,
                character_references: null,
                characters: null,
                use_coords: null,
              },
            },
          },
        },
      ],
    });
    expect(screen.getByRole("button", { name: "Add I2I source" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "Add Vibe from image" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "Add precise reference" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "Add character prompt" })).toBeEnabled();
  });

  it("prevents duplicate submit requests while queueing", async () => {
    const { user } = setup();
    mocks.generationApi.submitBatch.mockReturnValue(new Promise(() => {}));

    typeInPromptEditor(await screen.findByLabelText("Positive prompt"), "1girl");
    await user.click(screen.getByRole("button", { name: /^Generate 1 images/u }));

    const pendingButton = await screen.findByRole("button", { name: /^Queueing generation/u });
    expect(pendingButton).toBeDisabled();

    await user.click(pendingButton);
    expect(mocks.generationApi.submitBatch).toHaveBeenCalledTimes(1);
  });

  it("blocks empty prompt submission and keeps the draft after backend failures", async () => {
    mocks.generationApi.submitBatch.mockRejectedValueOnce(new Error("NovelAI key missing"));
    const { user } = setup();

    await user.click(await screen.findByRole("button", { name: /^Generate 1 images/u }));
    expect(mocks.generationApi.submitBatch).not.toHaveBeenCalled();
    expect(screen.getByText("Positive prompt is required.")).toBeInTheDocument();

    typeInPromptEditor(screen.getByLabelText("Positive prompt"), "1girl");
    await user.click(screen.getByRole("button", { name: /^Generate 1 images/u }));

    expect(await screen.findByText("NovelAI key missing")).toBeInTheDocument();
    expect(promptEditorText(screen.getByLabelText("Positive prompt"))).toBe("1girl");
  });

  it("allows main preset only generation without rewriting draft prompt", async () => {
    const { user } = setup({
      mainPresets: {
        items: [
          {
            preset_id: "preset-main",
            kind: "main",
            name: "Main stack",
            category: null,
            description: null,
            order: 0,
            prompt_behavior: {
              mode: "surround",
              before: "1girl",
              after: "sharp focus",
            },
            uc_behavior: { mode: "surround", before: "", after: "" },
            quality_override: null,
            uc_preset_override: null,
            preview: null,
            created_at_ms: 1,
            updated_at_ms: 1,
            models: ["nai-diffusion-4-5-full"],
          },
        ],
        total: 1,
        offset: 0,
        limit: 200,
      },
    });

    await user.click(await screen.findByRole("button", { name: "Choose Main preset" }));
    await user.click(screen.getByRole("button", { name: /Main stack/u }));
    await user.click(screen.getByRole("button", { name: /^Generate 1 images/u }));

    await waitFor(() => expect(mocks.generationApi.submitBatch).toHaveBeenCalledTimes(1));
    expect(mocks.generationApi.submitBatch.mock.calls[0]?.[0].jobs[0]?.work).toMatchObject({
      request: {
        base: {
          main_preset_id: "preset-main",
          prompt: "",
        },
      },
    });
    expect(promptEditorText(screen.getByLabelText("Positive prompt"))).toBe("");
  });

  it("searches, filters, clears, and directly applies a main preset", async () => {
    const { user } = setup({
      mainPresets: {
        items: [
          {
            preset_id: "preset-main",
            kind: "main",
            name: "Cinematic stack",
            category: "Style",
            description: "Cinematic lighting preset",
            order: 0,
            prompt_behavior: {
              mode: "surround",
              before: "masterpiece",
              after: "cinematic lighting",
            },
            uc_behavior: { mode: "replace", text: "bad anatomy" },
            quality_override: null,
            uc_preset_override: null,
            preview: null,
            created_at_ms: 1,
            updated_at_ms: 1,
            models: ["nai-diffusion-4-5-full"],
          },
          {
            preset_id: "preset-other",
            kind: "main",
            name: "Portrait stack",
            category: "Subject",
            description: null,
            order: 1,
            prompt_behavior: { mode: "replace", text: "portrait" },
            uc_behavior: { mode: "replace", text: "" },
            quality_override: null,
            uc_preset_override: null,
            preview: null,
            created_at_ms: 2,
            updated_at_ms: 2,
            models: ["nai-diffusion-4-5-full"],
          },
        ],
        total: 2,
        offset: 0,
        limit: 10_000,
      },
    });

    typeInPromptEditor(await screen.findByLabelText("Positive prompt"), "1girl");
    await user.click(screen.getByRole("button", { name: "Choose Main preset" }));
    await user.type(screen.getByLabelText("Search presets"), "cinematic");
    await user.click(screen.getByLabelText("Filter preset category"));
    await user.click(screen.getByRole("option", { name: "Style" }));
    expect(screen.queryByRole("button", { name: /Portrait stack/u })).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: /Cinematic stack/u }));

    expect(screen.getByLabelText("Main preset")).toHaveValue("Cinematic stack");
    await user.click(screen.getByRole("button", { name: "Remove Main preset" }));
    expect(screen.getByLabelText("Main preset")).toHaveValue("No main preset");

    await user.click(screen.getByRole("button", { name: "Choose Main preset" }));
    await user.click(screen.getByRole("button", { name: /Cinematic stack/u }));
    await user.click(screen.getByRole("button", { name: "Apply Main preset directly" }));

    expect(promptEditorText(screen.getByLabelText("Positive prompt"))).toBe(
      "masterpiece, 1girl, cinematic lighting",
    );
    await user.click(screen.getByRole("tab", { name: "Undesired Content" }));
    expect(promptEditorText(screen.getByLabelText("Undesired Content"))).toBe("bad anatomy");
    expect(screen.getByLabelText("Main preset")).toHaveValue("No main preset");
  });

  it("uses tabs and directly applies a character preset into both prompt fields", async () => {
    const { user } = setup({
      characterPresets: {
        items: [
          {
            preset_id: "preset-character",
            kind: "character",
            name: "Heroine",
            category: "Cast",
            description: null,
            order: 0,
            prompt_behavior: { mode: "surround", before: "solo", after: "blue eyes" },
            uc_behavior: { mode: "replace", text: "extra arms" },
            quality_override: null,
            uc_preset_override: null,
            preview: null,
            created_at_ms: 1,
            updated_at_ms: 1,
            models: ["nai-diffusion-4-5-full"],
          },
        ],
        total: 1,
        offset: 0,
        limit: 10_000,
      },
    });

    await user.click(await screen.findByRole("button", { name: "Add character prompt" }));
    const card = screen.getByRole("article", { name: "Character 1" });
    typeInPromptEditor(screen.getByLabelText("Character 1 prompt"), "1girl");
    await user.click(within(card).getByRole("button", { name: "Choose Character preset" }));
    await user.click(screen.getByRole("button", { name: /Heroine/u }));
    await user.click(within(card).getByRole("button", { name: "Apply Character preset directly" }));

    expect(promptEditorText(screen.getByLabelText("Character 1 prompt"))).toBe(
      "solo, 1girl, blue eyes",
    );
    await user.click(within(card).getByRole("tab", { name: "Undesired Content" }));
    expect(promptEditorText(screen.getByLabelText("Character 1 negative prompt"))).toBe(
      "extra arms",
    );
    expect(within(card).getByLabelText("Character preset")).toHaveValue("No character preset");
  });

  it("imports an image resource into i2i before submit", async () => {
    const { user } = setup();
    mocks.desktopApi.pickAndImportImageResources.mockResolvedValueOnce([
      { resource: { id: "source-image", variant_id: null } },
    ]);

    await user.click(await screen.findByRole("button", { name: "Add I2I source" }));
    typeInPromptEditor(screen.getByLabelText("Positive prompt"), "1girl");
    await user.click(screen.getByRole("button", { name: /^Generate 1 images/u }));

    await waitFor(() => expect(mocks.generationApi.submitBatch).toHaveBeenCalledTimes(1));
    expect(mocks.desktopApi.pickAndImportImageResources).toHaveBeenCalledWith("source_image", {
      extensions: [],
    });
    expect(mocks.generationApi.submitBatch.mock.calls[0]?.[0].jobs[0]?.work).toMatchObject({
      kind: "stream",
      request: {
        base: {
          img2img: {
            image: {
              kind: "resource_ref",
              resource: { id: "source-image", variant_id: null },
            },
          },
        },
      },
    });
  });

  it("releases unused and cleared imported input images", async () => {
    const { user } = setup();
    const source = { id: "resource:import:source:1", variant_id: null };
    const unused = { id: "resource:import:source:2", variant_id: null };
    mocks.desktopApi.pickAndImportImageResources.mockResolvedValueOnce([
      { resource: source },
      { resource: unused },
    ]);

    await user.click(await screen.findByRole("button", { name: "Add I2I source" }));
    await waitFor(() =>
      expect(mocks.resourceApi.releaseImportedImages).toHaveBeenCalledWith({
        resources: [unused],
      }),
    );

    await user.click(screen.getByRole("button", { name: "Remove I2I source" }));
    await waitFor(() =>
      expect(mocks.resourceApi.releaseImportedImages).toHaveBeenCalledWith({
        resources: [source],
      }),
    );
  });

  it("wires Precise Reference into generation and hides Vibe Transfer while it is active", async () => {
    const { user } = setup();
    const reference = { id: "resource:import:reference:1", variant_id: null };
    mocks.desktopApi.pickAndImportImageResources.mockResolvedValueOnce([{ resource: reference }]);

    await user.click(await screen.findByRole("button", { name: "Add precise reference" }));

    await waitFor(() =>
      expect(mocks.desktopApi.pickAndImportImageResources).toHaveBeenCalledWith("reference_image", {
        extensions: [],
      }),
    );
    expect(screen.queryByRole("button", { name: "Add Vibe from image" })).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Precise reference help" })).toBeInTheDocument();

    typeInPromptEditor(screen.getByLabelText("Positive prompt"), "1girl");
    await user.click(screen.getByRole("button", { name: /^Generate 1 images/u }));

    await waitFor(() => expect(mocks.generationApi.submitBatch).toHaveBeenCalledTimes(1));
    expect(mocks.generationApi.submitBatch.mock.calls[0]?.[0].jobs[0]?.work).toMatchObject({
      kind: "stream",
      request: {
        base: {
          vibe_transfer: null,
          character_references: [
            {
              image: { kind: "resource_ref", resource: reference },
              reference_type: "character",
              fidelity: 0.5,
              strength: 0.6,
            },
          ],
        },
      },
    });
  });

  it("adds an imported Vibe encoding from the Vibe library", async () => {
    const { user } = setup({
      vibeDocuments: {
        items: [
          {
            vibe_id: "vibe-1",
            display_name: "Style A",
            has_image: true,
            hidden: false,
            available_model_keys: ["v4curated", "v4-5full"],
            available_encoding_configs: [
              { model: "nai-diffusion-4-curated", information_extracted: 0.4 },
              { model: "nai-diffusion-4-5-full", information_extracted: 0.7 },
            ],
            document: { id: "vibe-document:vibe-1", variant_id: null },
            source_image: { id: "vibe-source:vibe-1", variant_id: null },
            preview: { id: "vibe-preview:vibe-1", variant_id: null },
            encodings: [
              { id: "vibe-encoding:vibe-1:v4curated:0", variant_id: null },
              { id: "vibe-encoding:vibe-1:v4-5full:1", variant_id: null },
            ],
            created_at_ms: 1,
            updated_at_ms: 1,
          },
        ],
        total: 1,
        offset: 0,
        limit: 32,
      },
    });
    await user.click(await screen.findByRole("button", { name: "Choose from Vibe library" }));
    const dialog = await screen.findByRole("dialog", { name: "Vibe library" });
    expect(await within(dialog).findByAltText("Style A")).toHaveAttribute(
      "src",
      "data:image/png;base64,final-image",
    );
    await user.click(within(dialog).getByRole("button", { name: /Style A/u }));
    typeInPromptEditor(screen.getByLabelText("Positive prompt"), "1girl");
    await user.click(screen.getByRole("button", { name: /^Generate 1 images/u }));

    await waitFor(() => expect(mocks.generationApi.submitBatch).toHaveBeenCalledTimes(1));
    expect(mocks.generationApi.submitBatch.mock.calls[0]?.[0].jobs[0]?.work).toMatchObject({
      kind: "stream",
      request: {
        base: {
          vibe_transfer: {
            references: [
              {
                encoding: { id: "vibe-encoding:vibe-1:v4-5full:1", variant_id: null },
              },
            ],
          },
        },
      },
    });
  });

  it("encodes a picked Vibe source image before submitting Vibe transfer", async () => {
    const { user } = setup();
    mocks.desktopApi.pickAndImportImageResources.mockResolvedValueOnce([
      { resource: { id: "control-source", variant_id: null } },
    ]);
    mocks.resourceApi.image.mockResolvedValueOnce({
      image_base64: "AQID",
      mime_type: "image/png",
    });
    mocks.vibeApi.ensureEncoding.mockResolvedValueOnce({
      resource: { id: "vibe-encoding:control-source", variant_id: null },
      created: true,
    });

    await user.click(await screen.findByRole("button", { name: "Add Vibe from image" }));
    await waitFor(() => expect(mocks.vibeApi.ensureEncoding).toHaveBeenCalledTimes(1));
    typeInPromptEditor(screen.getByLabelText("Positive prompt"), "1girl");
    await user.click(screen.getByRole("button", { name: /^Generate 1 images/u }));

    await waitFor(() => expect(mocks.generationApi.submitBatch).toHaveBeenCalledTimes(1));
    expect(mocks.desktopApi.pickAndImportImageResources).toHaveBeenCalledWith("control_net_image", {
      extensions: [],
    });
    expect(mocks.vibeApi.ensureEncoding).toHaveBeenCalledWith(
      expect.objectContaining({
        vibe_id: "control-source",
        image: "AQID",
        model: "nai-diffusion-4-5-full",
        information_extracted: 1,
      }),
    );
    expect(mocks.generationApi.submitBatch.mock.calls[0]?.[0].jobs[0]?.work).toMatchObject({
      kind: "stream",
      request: {
        base: {
          vibe_transfer: {
            references: [
              {
                encoding: { id: "vibe-encoding:control-source", variant_id: null },
              },
            ],
          },
        },
      },
    });
  });
});

describe("GeneratePage queue and preview behavior", () => {
  it("updates queue controls, one stable sample slot, final preview, and batch history", async () => {
    const fixture = generationBatchFixture();
    const { user } = setup({
      status: {
        batch_id: "batch-1",
        batch_status: "running",
        current_job_id: "job-1",
        job_status: "running",
        requests: [{ job_id: "job-1", request_index: 0, expected_samples: 1, status: "running" }],
      },
      history: fixture.page,
      historyDetail: {
        ...fixture.detail,
        requests: fixture.detail.requests.map((request) => ({
          ...request,
          status: "running",
          outputs: [],
        })),
      },
    });

    expect(await screen.findByRole("button", { name: "Pause queue" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "Resume queue" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Stop queue" })).toBeEnabled();
    expect(screen.queryByText("Live preview")).not.toBeInTheDocument();
    expect(screen.queryByText("History · batches")).not.toBeInTheDocument();
    expect(screen.getByLabelText("Live preview")).toBeInTheDocument();
    expect(screen.getByLabelText("1 request")).toBeInTheDocument();

    const requestCursorUnit = screen.getByText("R1").closest("button");
    expect(requestCursorUnit?.firstElementChild).toHaveClass("grid-cols-1", "grid-rows-1");

    await user.click(screen.getByRole("button", { name: "Pause queue" }));
    await user.click(screen.getByRole("button", { name: "Stop queue" }));
    expect(mocks.generationApi.pause).toHaveBeenCalledTimes(1);
    expect(mocks.generationApi.stop).toHaveBeenCalledTimes(1);

    act(() => {
      recordGenerationEvent(
        appEvent({
          kind: "generation_stream_chunk",
          batch_id: "batch-1",
          job_id: "job-1",
          event_type: "intermediate",
          sample_index: 0,
          step_index: 3,
          generation_id: 7,
          sigma: null,
          image: "stream-frame",
        }),
      );
    });

    await waitFor(() =>
      expect(
        screen
          .getAllByAltText(/Request 1 sample 1|Generation sample 1/u)
          .some((image) => image.getAttribute("src") === "data:image/png;base64,stream-frame"),
      ).toBe(true),
    );
    expect(screen.getByAltText("Request 1 sample 1")).toHaveStyle({ objectFit: "cover" });

    act(() => {
      recordGenerationEvent(
        appEvent({
          kind: "sample_persisted",
          batch_id: "batch-1",
          job_id: "job-1",
          sample_index: 0,
          resource: { id: "resource:generated:job-1:0", variant_id: null },
          artifact_id: "artifact-1",
        }),
      );
    });

    await waitFor(() =>
      expect(
        screen
          .getAllByAltText(/Request 1 sample 1|Generation sample 1/u)
          .some((image) => image.getAttribute("src") === "data:image/png;base64,final-image"),
      ).toBe(true),
    );
    const historyRail = screen.getByRole("complementary", { name: "Generation history" });
    expect(within(historyRail).getByText("1girl")).toBeInTheDocument();
    expect(within(historyRail).getByText("Succeeded", { selector: "span" })).toBeInTheDocument();
  });

  it("runs batch, request, and sample-level history actions", async () => {
    const fixture = generationBatchFixture();
    const { user } = setup({
      history: fixture.page,
      historyDetail: fixture.detail,
    });

    const historyRail = await screen.findByRole("complementary", {
      name: "Generation history",
    });
    await user.click(within(historyRail).getByText("1girl"));
    await user.click(within(historyRail).getByRole("button", { name: "Rerun selected batch" }));
    await user.click(
      within(historyRail).getByRole("button", { name: "Export selected batch as ZIP" }),
    );

    await user.click(await screen.findByRole("button", { name: "Focus sample 1" }));
    await user.click(screen.getByRole("button", { name: "Save selected sample" }));
    await user.click(screen.getByRole("button", { name: "Send selected sample to Director" }));
    await user.click(screen.getByRole("button", { name: "Export request as ZIP" }));
    await user.click(screen.getByRole("button", { name: "Rerun request" }));
    await user.click(screen.getByRole("button", { name: "Delete request history" }));

    expect(mocks.historyApi.rerunGenerationBatch).toHaveBeenCalledWith({
      source_batch_id: "batch-1",
      batch_id: "generation-00000000-0000-4000-8000-0000000000aa",
      job_ids: ["job-00000000-0000-4000-8000-0000000000bb"],
    });
    expect(mocks.desktopApi.saveResourceImagesZip).toHaveBeenCalledWith({
      entries: [
        {
          resource: { id: "resource:generated:job-1:0", variant_id: null },
          file_name: "request-01_sample-01",
        },
      ],
      suggested_file_name: "batch-1",
    });
    const requestRerun = mocks.historyApi.rerunGeneration.mock.calls[0]?.[0];
    expect(requestRerun?.run_id).toBe("job-1");
    expect(requestRerun?.batch_id).toMatch(/^generation-/u);
    expect(requestRerun?.job_id).toMatch(/^job-/u);
    expect(mocks.desktopApi.saveResourceImage).toHaveBeenCalledWith({
      resource: { id: "resource:generated:job-1:0", variant_id: null },
      format: null,
      suggested_file_name: "request-01_sample-01",
    });
    expect(mocks.galleryApi.imageReference).toHaveBeenCalledWith({
      item_id: "gallery-1",
      target: "director",
    });
    expect(mocks.historyApi.deleteItems).toHaveBeenCalledWith({ run_ids: ["job-1"] });
  });

  it("selects the visible task grid and deletes batch history in bulk", async () => {
    const fixture = generationBatchFixture();
    const secondBatch: GenerationHistoryBatchDto = {
      ...fixture.page.items[0],
      batch_id: "batch-2",
      title: "2girls",
      outputs: [],
      available_sample_count: 0,
    };
    const { user } = setup({
      history: {
        ...fixture.page,
        items: [...fixture.page.items, secondBatch],
        total: 2,
      },
      historyDetail: fixture.detail,
    });

    const historyRail = await screen.findByRole("complementary", {
      name: "Generation history",
    });
    await user.click(
      within(historyRail).getByRole("button", { name: "Select all visible batches" }),
    );
    expect(within(historyRail).getByText("2 selected")).toBeInTheDocument();
    await user.click(
      within(historyRail).getByRole("button", { name: "Delete selected batch histories" }),
    );
    expect(mocks.historyApi.deleteGenerationBatches).not.toHaveBeenCalled();
    await user.click(screen.getByRole("button", { name: "Delete histories" }));

    await waitFor(() =>
      expect(mocks.historyApi.deleteGenerationBatches).toHaveBeenCalledWith({
        batch_ids: ["batch-1", "batch-2"],
      }),
    );
  });

  it("surfaces queue command and final image failures", async () => {
    const { user } = setup({
      status: {
        batch_id: "batch-1",
        batch_status: "running",
        current_job_id: "job-1",
        job_status: "running",
        requests: [{ job_id: "job-1", request_index: 0, expected_samples: 1, status: "running" }],
      },
    });
    mocks.generationApi.pause.mockRejectedValueOnce(new Error("Pause command failed"));
    mocks.resourceApi.image.mockRejectedValue(new Error("resource missing"));

    await user.click(await screen.findByRole("button", { name: "Pause queue" }));
    expect(await screen.findByText("Open Generation to view details.")).toBeInTheDocument();

    act(() => {
      recordGenerationEvent(
        appEvent({
          kind: "sample_persisted",
          batch_id: "batch-1",
          job_id: "job-1",
          sample_index: 0,
          resource: { id: "resource:generated:job-1:0", variant_id: null },
          artifact_id: "artifact-1",
        }),
      );
    });

    expect(
      (
        await screen.findAllByText("Image unavailable: resource missing", undefined, {
          timeout: 4_000,
        })
      ).length,
    ).toBeGreaterThan(0);
  });

  it("compiles positive and negative prompt previews", async () => {
    const { user } = setup();

    typeInPromptEditor(await screen.findByLabelText("Positive prompt"), "$chunk(hero)");
    await user.click(screen.getByRole("tab", { name: "Undesired Content" }));
    typeInPromptEditor(screen.getByLabelText("Undesired Content"), "bad anatomy");
    await user.click(screen.getByRole("button", { name: "Compile" }));

    await waitFor(() => expect(mocks.promptApi.compileGenerationPreview).toHaveBeenCalledTimes(1));
    expect(mocks.promptApi.compileGenerationPreview).toHaveBeenCalledWith({
      prompt: "$chunk(hero)",
      model: "nai-diffusion-4-5-full",
      main_preset_id: null,
      negative_prompt: "bad anatomy",
      characters: [],
      max_depth: 8,
    });
    expect(await screen.findByText("expanded prompt")).toBeInTheDocument();
    expect(await screen.findByText("expanded negative")).toBeInTheDocument();
  });

  it("inserts a tag completion into the positive prompt before submit", async () => {
    const { user } = setup();

    typeInPromptEditor(await screen.findByLabelText("Positive prompt"), "cine");
    await user.click(await screen.findByRole("option", { name: /cinematic_lighting/u }));
    await user.click(screen.getByRole("button", { name: /^Generate 1 images/u }));

    await waitFor(() => expect(mocks.generationApi.submitBatch).toHaveBeenCalledTimes(1));
    expect(mocks.generationApi.submitBatch.mock.calls[0]?.[0].jobs[0]?.work).toMatchObject({
      request: {
        base: {
          prompt: "cinematic_lighting, ",
        },
      },
    });
  });

  it("uses Ctrl+Space to insert a prompt chunk into undesired content before compile", async () => {
    const { user } = setup();

    await user.click(await screen.findByRole("tab", { name: "Undesired Content" }));
    await user.click(screen.getByLabelText("Undesired Content"));
    await user.keyboard("{Control>} {/Control}");
    await user.click(await screen.findByRole("option", { name: /lighting/u }));
    await user.click(screen.getByRole("tab", { name: "Positive" }));
    typeInPromptEditor(screen.getByLabelText("Positive prompt"), "1girl");
    await user.click(screen.getByRole("button", { name: "Compile" }));

    await waitFor(() => expect(mocks.promptApi.compileGenerationPreview).toHaveBeenCalledTimes(1));
    expect(mocks.promptApi.compileGenerationPreview).toHaveBeenCalledWith(
      expect.objectContaining({
        prompt: "1girl",
        negative_prompt: "$chunk(lighting), ",
      }),
    );
  });

  it("supports tag and chunk completion in character prompts", async () => {
    const { user } = setup();

    fireEvent.click(await screen.findByRole("button", { name: "Add character prompt" }));
    const characterPrompt = await screen.findByLabelText("Character 1 prompt");
    typeInPromptEditor(characterPrompt, "cine");
    await screen.findByRole("option", { name: /cinematic_lighting/u });
    expect(acceptPromptCompletion(characterPrompt)).toBe(true);
    await waitFor(() =>
      expect(screen.queryByRole("option", { name: /cinematic_lighting/u })).not.toBeInTheDocument(),
    );
    const characterCard = screen.getByRole("article", { name: "Character 1" });
    await user.click(within(characterCard).getByRole("tab", { name: "Undesired Content" }));
    const characterNegativePrompt = screen.getByLabelText("Character 1 negative prompt");
    typeInPromptEditor(characterNegativePrompt, "$chunk(li");
    expect(startPromptCompletion(characterNegativePrompt)).toBe(true);
    await vi.waitFor(() =>
      expect(
        currentCompletions(promptEditorView(characterNegativePrompt).state).map(
          (item) => item.label,
        ),
      ).toContain("lighting"),
    );
    expect(acceptPromptCompletion(characterNegativePrompt)).toBe(true);
    typeInPromptEditor(screen.getByLabelText("Positive prompt"), "1girl");
    await user.click(screen.getByRole("button", { name: "Compile" }));

    await waitFor(() => expect(mocks.promptApi.compileGenerationPreview).toHaveBeenCalledTimes(1));
    expect(mocks.promptApi.compileGenerationPreview).toHaveBeenCalledWith(
      expect.objectContaining({
        characters: [
          {
            preset_id: null,
            prompt: "cinematic_lighting,",
            negative_prompt: "$chunk(lighting),",
            enabled: true,
          },
        ],
      }),
    );
  });

  it("closes and reopens completion before accepting an option", async () => {
    const { user } = setup();
    const prompt = await screen.findByLabelText("Positive prompt");

    typeInPromptEditor(prompt, "cine");
    expect(await screen.findByRole("listbox", { name: "Prompt completions" })).toBeInTheDocument();

    expect(closePromptCompletion(prompt)).toBe(true);
    await waitFor(() =>
      expect(screen.queryByRole("listbox", { name: "Prompt completions" })).not.toBeInTheDocument(),
    );

    clearPromptEditor(prompt);
    expect(startPromptCompletion(prompt)).toBe(true);
    expect(await screen.findByRole("option", { name: /lighting/u })).toBeInTheDocument();
    expect(screen.queryByRole("option", { name: /cinematic_lighting/u })).not.toBeInTheDocument();
    await user.click(screen.getByRole("option", { name: /hero/u }));

    expect(promptEditorText(prompt)).toBe("$chunk(hero), ");
  });
});
