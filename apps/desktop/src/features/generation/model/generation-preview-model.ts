import type {
  GenerationHistoryBatchDetailDto,
  GenerationHistoryRequestDto,
  GenerationStatusDto,
  ResourceRefDto,
  RunHistoryOutputDto,
} from "@/types";

import { generationPreviewKey, type GenerationPreview } from "../state/generation-event-store";

export type GenerationSampleState =
  | "pending"
  | "streaming"
  | "ready"
  | "failed"
  | "stopped"
  | "deleted"
  | "missing";

export type GenerationSampleSlot = {
  sampleIndex: number;
  state: GenerationSampleState;
  streamSrc: string | null;
  resource: ResourceRefDto | null;
  artifactId: string | null;
  galleryItemId: string | null;
  updatedSequence: number;
};

export type GenerationRequestUnit = {
  jobId: string;
  runId: string | null;
  requestIndex: number;
  expectedSamples: number;
  status: string;
  title: string | null;
  lastError: string | null;
  samples: GenerationSampleSlot[];
  updatedSequence: number;
};

export type GenerationBatchView = {
  batchId: string;
  status: string;
  requests: GenerationRequestUnit[];
};

type BuildGenerationBatchViewOptions = {
  batchId: string | null;
  detail: GenerationHistoryBatchDetailDto | undefined;
  status: GenerationStatusDto | undefined;
  previews: Record<string, GenerationPreview>;
};

type RequestSeed = {
  jobId: string;
  runId: string | null;
  requestIndex: number;
  expectedSamples: number;
  status: string;
  title: string | null;
  lastError: string | null;
  outputs: RunHistoryOutputDto[];
};

export function buildGenerationBatchView({
  batchId,
  detail,
  status,
  previews,
}: BuildGenerationBatchViewOptions): GenerationBatchView | null {
  if (!batchId) {
    return null;
  }
  const seeds = new Map<string, RequestSeed>();
  if (detail?.batch.batch_id === batchId) {
    for (const request of detail.requests) {
      seeds.set(request.job_id, historyRequestSeed(request));
    }
  }
  if (status?.batch_id === batchId) {
    for (const request of status.requests) {
      const existing = seeds.get(request.job_id);
      seeds.set(request.job_id, {
        jobId: request.job_id,
        runId: existing?.runId ?? null,
        requestIndex: request.request_index,
        expectedSamples: Math.max(1, request.expected_samples),
        status: request.status,
        title: existing?.title ?? null,
        lastError: existing?.lastError ?? null,
        outputs: existing?.outputs ?? [],
      });
    }
  }
  for (const preview of Object.values(previews)) {
    if (preview.batchId !== batchId || seeds.has(preview.jobId)) {
      continue;
    }
    seeds.set(preview.jobId, {
      jobId: preview.jobId,
      runId: null,
      requestIndex: seeds.size,
      expectedSamples: preview.sampleIndex + 1,
      status: preview.kind === "stream" ? "running" : "succeeded",
      title: null,
      lastError: null,
      outputs: [],
    });
  }
  const requests = [...seeds.values()]
    .map((seed) => buildRequestUnit(batchId, seed, previews))
    .sort(
      (left, right) =>
        left.requestIndex - right.requestIndex || left.jobId.localeCompare(right.jobId),
    );
  return {
    batchId,
    status:
      status?.batch_id === batchId
        ? (status.batch_status ?? "queued")
        : (detail?.batch.status ?? "queued"),
    requests,
  };
}

export function selectDefaultRequest(
  batch: GenerationBatchView | null,
  selectedJobId: string | null,
  latestJobId: string | null,
): GenerationRequestUnit | null {
  if (!batch?.requests.length) {
    return null;
  }
  return (
    batch.requests.find((request) => request.jobId === selectedJobId) ??
    batch.requests.find((request) => request.jobId === latestJobId) ??
    [...batch.requests].reverse().find((request) => request.status === "running") ??
    batch.requests[0] ??
    null
  );
}

function historyRequestSeed(request: GenerationHistoryRequestDto): RequestSeed {
  return {
    jobId: request.job_id,
    runId: request.run_id,
    requestIndex: request.request_index,
    expectedSamples: Math.max(1, request.expected_samples),
    status: request.status,
    title: request.title,
    lastError: request.last_error,
    outputs: request.outputs,
  };
}

function buildRequestUnit(
  batchId: string,
  seed: RequestSeed,
  previews: Record<string, GenerationPreview>,
): GenerationRequestUnit {
  const outputBySample = new Map<number, RunHistoryOutputDto>();
  for (const output of seed.outputs) {
    const sampleIndex = output.sample_index ?? outputBySample.size;
    if (!outputBySample.has(sampleIndex)) {
      outputBySample.set(sampleIndex, output);
    }
  }
  let expectedSamples = Math.max(1, seed.expectedSamples);
  for (const outputIndex of outputBySample.keys()) {
    expectedSamples = Math.max(expectedSamples, outputIndex + 1);
  }
  for (const preview of Object.values(previews)) {
    if (preview.batchId === batchId && preview.jobId === seed.jobId) {
      expectedSamples = Math.max(expectedSamples, preview.sampleIndex + 1);
    }
  }
  const samples = Array.from({ length: expectedSamples }, (_, sampleIndex) => {
    const output = outputBySample.get(sampleIndex);
    const preview = previews[generationPreviewKey(batchId, seed.jobId, sampleIndex)];
    return buildSampleSlot(sampleIndex, seed.status, output, preview);
  });
  return {
    jobId: seed.jobId,
    runId: seed.runId,
    requestIndex: seed.requestIndex,
    expectedSamples,
    status: seed.status,
    title: seed.title,
    lastError: seed.lastError,
    samples,
    updatedSequence: samples.reduce(
      (latest, sample) => Math.max(latest, sample.updatedSequence),
      0,
    ),
  };
}

function buildSampleSlot(
  sampleIndex: number,
  requestStatus: string,
  output: RunHistoryOutputDto | undefined,
  preview: GenerationPreview | undefined,
): GenerationSampleSlot {
  if (output?.state === "deleted") {
    return {
      sampleIndex,
      state: "deleted",
      streamSrc: null,
      resource: null,
      artifactId: output.artifact_id,
      galleryItemId: null,
      updatedSequence: 0,
    };
  }
  if (preview?.kind === "resource") {
    return {
      sampleIndex,
      state: "ready",
      streamSrc: null,
      resource: preview.resource,
      artifactId: preview.artifactId,
      galleryItemId: preview.galleryItemId,
      updatedSequence: preview.sequence,
    };
  }
  if (output?.state === "available" && output.resource) {
    return {
      sampleIndex,
      state: "ready",
      streamSrc: null,
      resource: output.resource,
      artifactId: output.artifact_id,
      galleryItemId: output.item_id,
      updatedSequence: 0,
    };
  }
  if (preview?.kind === "stream") {
    return {
      sampleIndex,
      state: "streaming",
      streamSrc: preview.src,
      resource: null,
      artifactId: null,
      galleryItemId: null,
      updatedSequence: preview.sequence,
    };
  }
  return {
    sampleIndex,
    state: sampleStateFromRequest(requestStatus),
    streamSrc: null,
    resource: null,
    artifactId: null,
    galleryItemId: null,
    updatedSequence: 0,
  };
}

function sampleStateFromRequest(status: string): GenerationSampleState {
  if (status === "failed") {
    return "failed";
  }
  if (status === "skipped" || status === "stopped") {
    return "stopped";
  }
  if (status === "succeeded") {
    return "missing";
  }
  return "pending";
}
