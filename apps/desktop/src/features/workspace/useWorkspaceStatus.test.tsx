import { QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ReactNode } from "react";

import { createAtelierQueryClient } from "../../app/query-client";
import type {
  CloseWorkspaceResponseDto,
  OpenWorkspaceRequestDto,
  WorkspaceStatusDto,
} from "../../types";
import {
  recordGenerationEvent,
  resetGenerationEventState,
  useGenerationEventStore,
} from "../generation/state/generation-event-store";
import { useWorkspaceStatus } from "./useWorkspaceStatus";

const mocks = vi.hoisted(() => {
  class MockAtelierCommandError extends Error {
    code: string;
    details: unknown;

    constructor(payload: { code: string; message: string; details?: unknown }) {
      super(payload.message);
      this.name = "AtelierCommandError";
      this.code = payload.code;
      this.details = payload.details ?? null;
    }
  }

  return {
    AtelierCommandError: MockAtelierCommandError,
    clearWorkspaceScopedQueryCache: vi.fn<() => Promise<void>>(),
    desktopApi: {
      pickWorkspaceDirectory: vi.fn<() => Promise<string | null>>(),
    },
    workspaceApi: {
      status: vi.fn<() => Promise<WorkspaceStatusDto>>(),
      open: vi.fn<(request: OpenWorkspaceRequestDto) => Promise<WorkspaceStatusDto>>(),
      close: vi.fn<() => Promise<CloseWorkspaceResponseDto>>(),
    },
  };
});

vi.mock("../../platform/atelier", () => ({
  AtelierCommandError: mocks.AtelierCommandError,
  clearWorkspaceScopedQueryCache: mocks.clearWorkspaceScopedQueryCache,
  desktopApi: mocks.desktopApi,
  workspaceApi: mocks.workspaceApi,
  queryKeys: {
    workspace: {
      root: () => ["workspace"],
      status: () => ["workspace", "status"],
    },
  },
}));

beforeEach(() => {
  vi.clearAllMocks();
  resetGenerationEventState();
  mocks.clearWorkspaceScopedQueryCache.mockResolvedValue(undefined);
  mocks.desktopApi.pickWorkspaceDirectory.mockResolvedValue("D:/atelier-next");
  mocks.workspaceApi.status.mockRejectedValue(
    new mocks.AtelierCommandError({
      code: "workspace_not_open",
      message: "workspace is not open",
    }),
  );
  mocks.workspaceApi.open.mockResolvedValue({
    root: "D:/atelier-next",
    schema_version: 4,
    locked: false,
  });
  mocks.workspaceApi.close.mockResolvedValue({ was_open: true });
});

describe("useWorkspaceStatus", () => {
  it("clears generation event state before publishing an opened workspace", async () => {
    const user = userEvent.setup();
    mocks.workspaceApi.status
      .mockRejectedValueOnce(
        new mocks.AtelierCommandError({
          code: "workspace_not_open",
          message: "workspace is not open",
        }),
      )
      .mockResolvedValue({
        root: "D:/atelier-next",
        schema_version: 4,
        locked: false,
      });
    recordGenerationEvent({
      sequence: 1,
      kind: {
        kind: "generation_stream_chunk",
        batch_id: "old-batch",
        job_id: "old-job",
        event_type: "intermediate",
        sample_index: 0,
        step_index: 1,
        generation_id: 1,
        sigma: null,
        image: "old-frame",
      },
    });

    renderWithClient(<WorkspaceProbe />);

    expect(await screen.findByText("old-job")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Open workspace" }));

    expect(await screen.findByText("D:/atelier-next")).toBeInTheDocument();
    expect(screen.getByText("no-preview")).toBeInTheDocument();
    expect(useGenerationEventStore.getState().activePreview).toBeNull();
  });
});

function WorkspaceProbe() {
  const workspace = useWorkspaceStatus();
  const activePreview = useGenerationEventStore((state) => state.activePreview);

  return (
    <div>
      <p>{workspace.workspaceStatus?.root ?? "closed"}</p>
      <p>{activePreview?.jobId ?? "no-preview"}</p>
      <button type="button" onClick={workspace.openWorkspace}>
        Open workspace
      </button>
    </div>
  );
}

function renderWithClient(children: ReactNode) {
  return render(
    <QueryClientProvider client={createAtelierQueryClient()}>{children}</QueryClientProvider>,
  );
}
