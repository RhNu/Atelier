/* eslint-disable max-lines, max-lines-per-function */
import { QueryClientProvider } from "@tanstack/react-query";
import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { createAtelierQueryClient } from "../app/query-client";
import { GeneratePage } from "../features/generation";
import {
  recordGenerationEvent,
  resetGenerationEventState,
} from "../features/generation/state/generation-event-store";
import type {
  AppEventDto,
  CompileGenerationPromptRequestDto,
  CompiledGenerationPromptDto,
  DeleteRunHistoryItemsRequestDto,
  DeleteRunHistoryItemsResponseDto,
  GalleryImageReferenceRequestDto,
  GalleryImageReferenceDto,
  GenerationAnlasEstimateDto,
  GenerationEstimateRequestDto,
  GenerationStatusDto,
  GenerationStatusQueryDto,
  GetResourceImageRequestDto,
  ImportImageResourceResponseDto,
  ImageResourceKindDto,
  ImportedVibeDocumentsDto,
  ListVibeDocumentsRequestDto,
  ListPromptPresetsRequestDto,
  ListPromptChunksRequestDto,
  QueueDirectiveDto,
  PromptChunkPageDto,
  PromptLexiconPageDto,
  PromptLexiconSearchQueryDto,
  PromptPresetPageDto,
  ResourceImageDto,
  ReleaseImportedImageResourcesRequestDto,
  ReleaseImportedImageResourcesResponseDto,
  ResourceRefDto,
  RerunGenerationHistoryItemRequestDto,
  RerunGenerationHistoryItemResponseDto,
  RunHistoryPageDto,
  RunHistoryQueryDto,
  SaveResourceImageRequestDto,
  SubmitGenerationBatchRequestDto,
  SubscriptionSummaryDto,
  EnsureVibeEncodingRequestDto,
  EnsuredVibeEncodingDto,
  VibeDocumentPageDto,
  WorkspaceSettingsDto,
} from "../types";

const mocks = vi.hoisted(() => ({
  generationApi: {
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
  },
  promptApi: {
    compileGenerationPreview:
      vi.fn<(request: CompileGenerationPromptRequestDto) => Promise<CompiledGenerationPromptDto>>(),
    listChunks: vi.fn<(request: ListPromptChunksRequestDto) => Promise<PromptChunkPageDto>>(),
    listPresets: vi.fn<(request: ListPromptPresetsRequestDto) => Promise<PromptPresetPageDto>>(),
    lexiconSearch: vi.fn<(request: PromptLexiconSearchQueryDto) => Promise<PromptLexiconPageDto>>(),
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
  galleryApi: {
    imageReference:
      vi.fn<(request: GalleryImageReferenceRequestDto) => Promise<GalleryImageReferenceDto>>(),
  },
  accountApi: {
    cachedActiveSubscription: vi.fn<() => Promise<SubscriptionSummaryDto | null>>(),
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
  resourceApi: mocks.resourceApi,
  settingsApi: mocks.settingsApi,
  galleryApi: mocks.galleryApi,
  accountApi: mocks.accountApi,
  desktopApi: mocks.desktopApi,
  vibeApi: mocks.vibeApi,
  queryKeys: {
    generation: {
      root: () => ["generation"],
      status: (jobId?: string | null) => ["generation", "status", jobId ?? null],
      estimate: (request: unknown) => ["generation", "estimate", request],
    },
    history: {
      root: () => ["history"],
      list: (query: RunHistoryQueryDto) => ["history", "list", query],
    },
    settings: {
      workspace: () => ["settings", "workspace"],
    },
    prompt: {
      root: () => ["prompt"],
      chunks: (query?: unknown) =>
        query === undefined ? ["prompt", "chunks"] : ["prompt", "chunks", query],
      presets: (query: ListPromptPresetsRequestDto) => ["prompt", "presets", query],
      lexiconSearch: (query: unknown) => ["prompt", "lexicon", "search", query],
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
      activeProbe: () => ["account", "active-probe"],
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
}));

const defaultSettings: WorkspaceSettingsDto = {
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
  frontend: {
    gallery: {
      blur_sensitive_images: false,
    },
  },
};

function appEvent(kind: AppEventDto["kind"], sequence = 1): AppEventDto {
  return { sequence, kind };
}

function setup(options?: {
  status?: GenerationStatusDto;
  statusError?: Error;
  history?: RunHistoryPageDto;
  vibeDocuments?: VibeDocumentPageDto;
  settingsError?: Error;
  mainPresets?: PromptPresetPageDto;
  characterPresets?: PromptPresetPageDto;
}) {
  if (options?.settingsError) {
    mocks.settingsApi.get.mockRejectedValue(options.settingsError);
  } else {
    mocks.settingsApi.get.mockResolvedValue(
      structuredClone(defaultSettings) as WorkspaceSettingsDto,
    );
  }
  if (options?.statusError) {
    mocks.generationApi.status.mockRejectedValue(options.statusError);
  } else {
    mocks.generationApi.status.mockResolvedValue(
      options?.status ?? { batch_status: null, job_status: null },
    );
  }
  mocks.generationApi.submitBatch.mockResolvedValue({ kind: "start_job", job_id: "job-submitted" });
  mocks.generationApi.estimate.mockResolvedValue({
    per_sample_cost: 3,
    per_request_cost: 3,
    total_cost: 3,
    adjusted_resolution: 1_011_712,
    opus_discount_applied: false,
    pending_encode_cost: 0,
  });
  mocks.generationApi.pause.mockResolvedValue({ kind: "paused" });
  mocks.generationApi.resume.mockResolvedValue({ kind: "start_job", job_id: "job-submitted" });
  mocks.generationApi.stop.mockResolvedValue({ kind: "idle" });
  mocks.historyApi.list.mockResolvedValue(
    options?.history ?? {
      items: [],
      offset: 0,
      limit: 8,
      total: 0,
    },
  );
  mocks.historyApi.deleteItems.mockResolvedValue({ deleted: 1 });
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
  mocks.promptApi.lexiconSearch.mockImplementation(async (request) => ({
    items:
      request.query.trim().length > 0
        ? [
            {
              tag: "cinematic_lighting",
              weight: 1200,
              category: "copyright",
              subcategory: "lighting",
              primary_translation: "cinematic lighting",
              matched_translation: "cinematic lighting",
              match_field: "tag",
              match_rank: "prefix",
            },
          ]
        : [],
    total: request.query.trim().length > 0 ? 1 : 0,
    offset: 0,
    limit: request.limit,
  }));
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
  mocks.accountApi.cachedActiveSubscription.mockResolvedValue({
    anlas_balance: 100,
    is_opus: false,
    tier: 1,
    tier_name: "Tablet",
    expires_at_ms: null,
  });

  return {
    user: userEvent.setup(),
    ...render(
      <QueryClientProvider client={createAtelierQueryClient()}>
        <GeneratePage />
      </QueryClientProvider>,
    ),
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

beforeEach(() => {
  vi.restoreAllMocks();
  vi.clearAllMocks();
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
      await screen.findByText("Generation status unavailable: Queue offline", undefined, {
        timeout: 4_000,
      }),
    ).toBeInTheDocument();
  });

  it("hydrates the generation draft from workspace settings", async () => {
    setup();

    expect(await screen.findByDisplayValue("832")).toBeInTheDocument();
    expect(screen.getByDisplayValue("1216")).toBeInTheDocument();
    expect(screen.getByDisplayValue("23")).toBeInTheDocument();
    expect(screen.getByDisplayValue("5")).toBeInTheDocument();
    expect(screen.getByLabelText("Model")).toHaveValue("nai-diffusion-4-5-full");
    expect(screen.getByLabelText("Sampler")).toHaveValue("k_euler_ancestral");
  });

  it("submits batch stream generation work from the current draft", async () => {
    const { user } = setup();

    await user.type(await screen.findByLabelText("Positive prompt"), "1girl, atelier lighting");
    await user.type(screen.getByLabelText("Undesired content"), "low quality");
    await user.clear(screen.getByLabelText("Steps"));
    await user.type(screen.getByLabelText("Steps"), "28");
    await user.click(screen.getByRole("button", { name: "Queue generation" }));

    await waitFor(() => expect(mocks.generationApi.submitBatch).toHaveBeenCalledTimes(1));
    const request = mocks.generationApi.submitBatch.mock.calls[0]?.[0];
    expect(request).toMatchObject({
      batch_id: "generation-00000000-0000-4000-8000-0000000000aa",
      context: { request_count: 1, pending_vibe_encode_count: 0, is_opus: false },
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
                i2i: null,
                controlnet: null,
                character_references: null,
                characters: null,
                use_coords: null,
              },
            },
          },
        },
      ],
    });
    expect(screen.getByRole("button", { name: "Add source" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "Add Vibe slot" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "Add reference" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "Add character prompt" })).toBeEnabled();
  });

  it("prevents duplicate submit requests while queueing", async () => {
    const { user } = setup();
    mocks.generationApi.submitBatch.mockReturnValue(new Promise(() => {}));

    await user.type(await screen.findByLabelText("Positive prompt"), "1girl");
    await user.click(screen.getByRole("button", { name: "Queue generation" }));

    const pendingButton = await screen.findByRole("button", { name: "Queueing generation" });
    expect(pendingButton).toBeDisabled();

    await user.click(pendingButton);
    expect(mocks.generationApi.submitBatch).toHaveBeenCalledTimes(1);
  });

  it("blocks empty prompt submission and keeps the draft after backend failures", async () => {
    mocks.generationApi.submitBatch.mockRejectedValueOnce(new Error("NovelAI key missing"));
    const { user } = setup();

    await user.click(await screen.findByRole("button", { name: "Queue generation" }));
    expect(mocks.generationApi.submitBatch).not.toHaveBeenCalled();
    expect(screen.getByText("Positive prompt is required.")).toBeInTheDocument();

    await user.type(screen.getByLabelText("Positive prompt"), "1girl");
    await user.click(screen.getByRole("button", { name: "Queue generation" }));

    expect(await screen.findByText("NovelAI key missing")).toBeInTheDocument();
    expect(screen.getByLabelText("Positive prompt")).toHaveValue("1girl");
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
            enabled: true,
            before: "1girl",
            after: "sharp focus",
            replace: "",
            uc_before: "",
            uc_after: "",
            uc_replace: "",
            quality_override: null,
            uc_preset_override: null,
            preview: null,
            created_at_ms: 1,
            updated_at_ms: 1,
          },
        ],
        total: 1,
        offset: 0,
        limit: 200,
      },
    });

    await user.selectOptions(await screen.findByLabelText("Main preset"), "preset-main");
    await user.click(screen.getByRole("button", { name: "Queue generation" }));

    await waitFor(() => expect(mocks.generationApi.submitBatch).toHaveBeenCalledTimes(1));
    expect(mocks.generationApi.submitBatch.mock.calls[0]?.[0].jobs[0]?.work).toMatchObject({
      request: {
        base: {
          main_preset_id: "preset-main",
          prompt: "",
        },
      },
    });
    expect(screen.getByLabelText("Positive prompt")).toHaveValue("");
  });

  it("imports an image resource into i2i before submit", async () => {
    const { user } = setup();
    mocks.desktopApi.pickAndImportImageResources.mockResolvedValueOnce([
      { resource: { id: "source-image", variant_id: null } },
    ]);

    await user.click(await screen.findByRole("button", { name: "Add source" }));
    await user.type(screen.getByLabelText("Positive prompt"), "1girl");
    await user.click(screen.getByRole("button", { name: "Queue generation" }));

    await waitFor(() => expect(mocks.generationApi.submitBatch).toHaveBeenCalledTimes(1));
    expect(mocks.desktopApi.pickAndImportImageResources).toHaveBeenCalledWith("source_image", {
      extensions: [],
    });
    expect(mocks.generationApi.submitBatch.mock.calls[0]?.[0].jobs[0]?.work).toMatchObject({
      kind: "stream",
      request: {
        base: {
          i2i: {
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

    await user.click(await screen.findByRole("button", { name: "Add source" }));
    await waitFor(() =>
      expect(mocks.resourceApi.releaseImportedImages).toHaveBeenCalledWith({
        resources: [unused],
      }),
    );

    await user.click(screen.getByRole("button", { name: "Clear" }));
    await waitFor(() =>
      expect(mocks.resourceApi.releaseImportedImages).toHaveBeenCalledWith({
        resources: [source],
      }),
    );
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
    await user.selectOptions(await screen.findByLabelText("Vibe library"), "vibe-1");
    await user.type(screen.getByLabelText("Positive prompt"), "1girl");
    await user.click(screen.getByRole("button", { name: "Queue generation" }));

    await waitFor(() => expect(mocks.generationApi.submitBatch).toHaveBeenCalledTimes(1));
    expect(mocks.generationApi.submitBatch.mock.calls[0]?.[0].jobs[0]?.work).toMatchObject({
      kind: "stream",
      request: {
        base: {
          controlnet: {
            images: [
              {
                encoding: { id: "vibe-encoding:vibe-1:v4-5full:1", variant_id: null },
                info_extracted: 0.7,
              },
            ],
          },
        },
      },
    });
  });

  it("encodes a picked Vibe source image before submitting controlnet", async () => {
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

    await user.click(await screen.findByRole("button", { name: "Add Vibe slot" }));
    await waitFor(() => expect(mocks.vibeApi.ensureEncoding).toHaveBeenCalledTimes(1));
    await user.type(screen.getByLabelText("Positive prompt"), "1girl");
    await user.click(screen.getByRole("button", { name: "Queue generation" }));

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
          controlnet: {
            images: [
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
  it("updates queue controls, stream preview, final preview, and history rail", async () => {
    const { user } = setup({
      status: {
        batch_status: "running",
        job_status: "running",
      },
      history: {
        items: [
          {
            run_id: "job-1",
            kind: "generation",
            status: "succeeded",
            batch_id: "batch-1",
            job_id: "job-1",
            origin_run_id: null,
            title: "1girl",
            last_error: null,
            created_at_ms: 1,
            updated_at_ms: 2,
            completed_at_ms: 2,
            recoverable: false,
            outputs: [
              {
                artifact_id: "artifact-1",
                item_id: "gallery-1",
                resource: { id: "resource:generated:job-1:0", variant_id: null },
                asset_role: "primary",
                variant_kind: null,
              },
            ],
          },
        ],
        offset: 0,
        limit: 8,
        total: 1,
      },
    });

    expect(await screen.findByRole("button", { name: "Pause queue" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "Resume queue" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Stop queue" })).toBeEnabled();

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

    expect(await screen.findByAltText("Active generation preview")).toHaveAttribute(
      "src",
      "data:image/png;base64,stream-frame",
    );

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

    expect(await screen.findByAltText("Final generation preview")).toHaveAttribute(
      "src",
      "data:image/png;base64,final-image",
    );
    const historyRail = screen.getByRole("complementary", { name: "Generation history" });
    expect(within(historyRail).getByText("1girl")).toBeInTheDocument();
    expect(within(historyRail).getByText(/succeeded/)).toBeInTheDocument();
  });

  it("runs selected history rerun, export, director handoff, and delete actions", async () => {
    const { user } = setup({
      history: {
        items: [
          {
            run_id: "job-1",
            kind: "generation",
            status: "succeeded",
            batch_id: "batch-1",
            job_id: "job-1",
            origin_run_id: null,
            title: "1girl",
            last_error: null,
            created_at_ms: 1,
            updated_at_ms: 2,
            completed_at_ms: 2,
            recoverable: false,
            outputs: [
              {
                artifact_id: "artifact-1",
                item_id: "gallery-1",
                resource: { id: "resource:generated:job-1:0", variant_id: null },
                asset_role: "primary",
                variant_kind: null,
              },
            ],
          },
        ],
        offset: 0,
        limit: 8,
        total: 1,
      },
    });

    const historyRail = await screen.findByRole("complementary", {
      name: "Generation history",
    });
    await user.click(within(historyRail).getByText("1girl"));
    await user.click(
      within(historyRail).getByRole("button", { name: "Rerun selected history item" }),
    );
    await user.click(
      within(historyRail).getByRole("button", { name: "Export selected history output" }),
    );
    await user.click(
      within(historyRail).getByRole("button", {
        name: "Send selected history output to Director",
      }),
    );
    await user.click(
      within(historyRail).getByRole("button", { name: "Delete selected history item" }),
    );

    expect(mocks.historyApi.rerunGeneration).toHaveBeenCalledWith({
      run_id: "job-1",
      batch_id: "generation-00000000-0000-4000-8000-0000000000aa",
      job_id: "job-00000000-0000-4000-8000-0000000000bb",
    });
    expect(mocks.desktopApi.saveResourceImage).toHaveBeenCalledWith({
      resource: { id: "resource:generated:job-1:0", variant_id: null },
      suggested_file_name: "job-1-sample",
    });
    expect(mocks.galleryApi.imageReference).toHaveBeenCalledWith({
      item_id: "gallery-1",
      target: "director",
    });
    expect(mocks.historyApi.deleteItems).toHaveBeenCalledWith({ run_ids: ["job-1"] });
  });

  it("surfaces queue command and final image failures", async () => {
    const { user } = setup({
      status: {
        batch_status: "running",
        job_status: "running",
      },
    });
    mocks.generationApi.pause.mockRejectedValueOnce(new Error("Pause command failed"));
    mocks.resourceApi.image.mockRejectedValue(new Error("resource missing"));

    await user.click(await screen.findByRole("button", { name: "Pause queue" }));
    expect(await screen.findByText("Pause command failed")).toBeInTheDocument();

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
      await screen.findByText("Final image unavailable: resource missing", undefined, {
        timeout: 4_000,
      }),
    ).toBeInTheDocument();
  });

  it("compiles positive and negative prompt previews", async () => {
    const { user } = setup();

    await user.type(await screen.findByLabelText("Positive prompt"), "@chunk(hero)");
    await user.type(screen.getByLabelText("Undesired content"), "bad anatomy");
    await user.click(screen.getByRole("button", { name: "Compile prompt preview" }));

    await waitFor(() => expect(mocks.promptApi.compileGenerationPreview).toHaveBeenCalledTimes(1));
    expect(mocks.promptApi.compileGenerationPreview).toHaveBeenCalledWith({
      prompt: "@chunk(hero)",
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

    await user.type(await screen.findByLabelText("Positive prompt"), "cine");
    await user.click(await screen.findByRole("option", { name: /cinematic_lighting/u }));
    await user.click(screen.getByRole("button", { name: "Queue generation" }));

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

    await user.click(await screen.findByLabelText("Undesired content"));
    await user.keyboard("{Control>} {/Control}");
    await user.click(await screen.findByRole("option", { name: /lighting/u }));
    await user.type(screen.getByLabelText("Positive prompt"), "1girl");
    await user.click(screen.getByRole("button", { name: "Compile prompt preview" }));

    await waitFor(() => expect(mocks.promptApi.compileGenerationPreview).toHaveBeenCalledTimes(1));
    expect(mocks.promptApi.compileGenerationPreview).toHaveBeenCalledWith(
      expect.objectContaining({
        prompt: "1girl",
        negative_prompt: "@chunk(lighting), ",
      }),
    );
  });

  it("supports tag and chunk completion in character prompts", async () => {
    const { user } = setup();

    fireEvent.click(await screen.findByRole("button", { name: "Add character prompt" }));
    await user.type(await screen.findByLabelText("Character 1 prompt"), "cine");
    await screen.findByRole("option", { name: /cinematic_lighting/u });
    await user.keyboard("{Enter}");
    await user.type(screen.getByLabelText("Character 1 negative prompt"), "@chunk(li");
    await screen.findByRole("option", { name: /lighting/u });
    await user.keyboard("{Tab}");
    await user.type(screen.getByLabelText("Positive prompt"), "1girl");
    await user.click(screen.getByRole("button", { name: "Compile prompt preview" }));

    await waitFor(() => expect(mocks.promptApi.compileGenerationPreview).toHaveBeenCalledTimes(1));
    expect(mocks.promptApi.compileGenerationPreview).toHaveBeenCalledWith(
      expect.objectContaining({
        characters: [
          {
            preset_id: null,
            prompt: "cinematic_lighting,",
            negative_prompt: "@chunk(lighting), ",
            enabled: true,
          },
        ],
      }),
    );
  });

  it("closes completion with Escape and accepts the active option with Enter", async () => {
    const { user } = setup();
    const prompt = await screen.findByLabelText("Positive prompt");

    await user.type(prompt, "cine");
    expect(await screen.findByRole("listbox", { name: "Prompt completions" })).toBeInTheDocument();

    await user.keyboard("{Escape}");
    await waitFor(() =>
      expect(screen.queryByRole("listbox", { name: "Prompt completions" })).not.toBeInTheDocument(),
    );

    await user.clear(prompt);
    await user.keyboard("{Control>} {/Control}");
    expect(await screen.findByRole("option", { name: /lighting/u })).toBeInTheDocument();
    expect(screen.queryByRole("option", { name: /cinematic_lighting/u })).not.toBeInTheDocument();
    await user.keyboard("{ArrowDown}");
    await user.keyboard("{Enter}");

    expect(prompt).toHaveValue("@chunk(hero), ");
  });
});
