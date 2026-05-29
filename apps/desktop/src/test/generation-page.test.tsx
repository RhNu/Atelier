import { QueryClientProvider } from "@tanstack/react-query";
import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { createAtelierQueryClient } from "../app/query-client";
import { GeneratePage } from "../features/generation";
import {
  recordGenerationEvent,
  resetGenerationEventState,
} from "../features/generation/state/generation-event-store";
import type {
  AppEventDto,
  CompilePromptRequestDto,
  CompiledPromptDto,
  GalleryImageReferenceRequestDto,
  GenerationStatusDto,
  GenerationStatusQueryDto,
  GetResourceImageRequestDto,
  QueueDirectiveDto,
  ResourceImageDto,
  RunHistoryPageDto,
  RunHistoryQueryDto,
  SubmitGenerationRequestDto,
  WorkspaceSettingsDto,
} from "../types";

const mocks = vi.hoisted(() => ({
  generationApi: {
    submit: vi.fn<(request: SubmitGenerationRequestDto) => Promise<QueueDirectiveDto>>(),
    pause: vi.fn<() => Promise<QueueDirectiveDto>>(),
    resume: vi.fn<() => Promise<QueueDirectiveDto>>(),
    stop: vi.fn<() => Promise<QueueDirectiveDto>>(),
    status: vi.fn<(request: GenerationStatusQueryDto) => Promise<GenerationStatusDto>>(),
  },
  historyApi: {
    list: vi.fn<(request: RunHistoryQueryDto) => Promise<RunHistoryPageDto>>(),
  },
  promptApi: {
    compilePreview: vi.fn<(request: CompilePromptRequestDto) => Promise<CompiledPromptDto>>(),
  },
  resourceApi: {
    image: vi.fn<(request: GetResourceImageRequestDto) => Promise<ResourceImageDto>>(),
  },
  settingsApi: {
    get: vi.fn<() => Promise<WorkspaceSettingsDto>>(),
  },
  galleryApi: {
    imageReference: vi.fn<(request: GalleryImageReferenceRequestDto) => Promise<unknown>>(),
  },
}));

vi.mock("../platform/atelier", () => ({
  generationApi: mocks.generationApi,
  historyApi: mocks.historyApi,
  promptApi: mocks.promptApi,
  resourceApi: mocks.resourceApi,
  settingsApi: mocks.settingsApi,
  galleryApi: mocks.galleryApi,
  queryKeys: {
    generation: {
      root: () => ["generation"],
      status: (jobId?: string | null) => ["generation", "status", jobId ?? null],
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
  },
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
};

function appEvent(kind: AppEventDto["kind"], sequence = 1): AppEventDto {
  return { sequence, kind };
}

function setup(options?: {
  status?: GenerationStatusDto;
  statusError?: Error;
  history?: RunHistoryPageDto;
  settingsError?: Error;
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
  mocks.generationApi.submit.mockResolvedValue({ kind: "start_job", job_id: "job-submitted" });
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
  mocks.promptApi.compilePreview.mockResolvedValue({
    expanded_prompt: "expanded prompt",
    trace: {
      raw_prompt: "raw prompt",
      expanded_prompt: "expanded prompt",
      function_calls: [],
    },
  });
  mocks.resourceApi.image.mockResolvedValue({
    image_base64: "final-image",
    mime_type: "image/png",
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

  it("submits stream generation work from the current draft and keeps advanced inputs disabled", async () => {
    const { user } = setup();

    await user.type(await screen.findByLabelText("Positive prompt"), "1girl, atelier lighting");
    await user.type(screen.getByLabelText("Undesired content"), "low quality");
    await user.clear(screen.getByLabelText("Steps"));
    await user.type(screen.getByLabelText("Steps"), "28");
    await user.click(screen.getByRole("button", { name: "Queue generation" }));

    await waitFor(() => expect(mocks.generationApi.submit).toHaveBeenCalledTimes(1));
    const request = mocks.generationApi.submit.mock.calls[0]?.[0];
    expect(request).toMatchObject({
      batch_id: "generation-00000000-0000-4000-8000-0000000000aa",
      job_id: "job-00000000-0000-4000-8000-0000000000bb",
      context: { request_count: 1, pending_vibe_encode_count: 0, is_opus: false },
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
    });
    expect(screen.getByRole("button", { name: "Image to image" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Vibe transfer" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Director tools" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Character references" })).toBeDisabled();
  });

  it("prevents duplicate submit requests while queueing", async () => {
    const { user } = setup();
    mocks.generationApi.submit.mockReturnValue(new Promise(() => {}));

    await user.type(await screen.findByLabelText("Positive prompt"), "1girl");
    await user.click(screen.getByRole("button", { name: "Queue generation" }));

    const pendingButton = await screen.findByRole("button", { name: "Queueing generation" });
    expect(pendingButton).toBeDisabled();

    await user.click(pendingButton);
    expect(mocks.generationApi.submit).toHaveBeenCalledTimes(1);
  });

  it("blocks empty prompt submission and keeps the draft after backend failures", async () => {
    mocks.generationApi.submit.mockRejectedValueOnce(new Error("NovelAI key missing"));
    const { user } = setup();

    await user.click(await screen.findByRole("button", { name: "Queue generation" }));
    expect(mocks.generationApi.submit).not.toHaveBeenCalled();
    expect(screen.getByText("Positive prompt is required.")).toBeInTheDocument();

    await user.type(screen.getByLabelText("Positive prompt"), "1girl");
    await user.click(screen.getByRole("button", { name: "Queue generation" }));

    expect(await screen.findByText("NovelAI key missing")).toBeInTheDocument();
    expect(screen.getByLabelText("Positive prompt")).toHaveValue("1girl");
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

    expect(await screen.findByAltText("Active generation preview")).toHaveAttribute(
      "src",
      "data:image/png;base64,stream-frame",
    );

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

    expect(await screen.findByAltText("Final generation preview")).toHaveAttribute(
      "src",
      "data:image/png;base64,final-image",
    );
    const historyRail = screen.getByRole("complementary", { name: "Generation history" });
    expect(within(historyRail).getByText("1girl")).toBeInTheDocument();
    expect(within(historyRail).getByText("succeeded")).toBeInTheDocument();
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

    await waitFor(() => expect(mocks.promptApi.compilePreview).toHaveBeenCalledTimes(2));
    expect(mocks.promptApi.compilePreview).toHaveBeenNthCalledWith(1, {
      prompt: "@chunk(hero)",
      max_depth: 8,
    });
    expect(mocks.promptApi.compilePreview).toHaveBeenNthCalledWith(2, {
      prompt: "bad anatomy",
      max_depth: 8,
    });
    expect(await screen.findAllByText("expanded prompt")).toHaveLength(2);
  });
});
