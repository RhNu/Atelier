import { QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { createAtelierQueryClient } from "../app/query-client";
import { ChunkWorkspace } from "../features/resources/components/ChunkWorkspace";
import type { PromptChunkDto } from "../types";

const mocks = vi.hoisted(() => ({
  upsert: { isPending: false, mutateAsync: vi.fn<() => Promise<never>>() },
  remove: { isPending: false, mutateAsync: vi.fn<() => Promise<never>>() },
  compile: { isPending: false, mutateAsync: vi.fn<() => Promise<never>>() },
  preview: {
    isPending: false,
    isError: false,
    error: null,
    mutateAsync: vi.fn<() => Promise<null>>(),
  },
}));

vi.mock("../features/resources/data/useResourcesData", () => ({
  useUpsertPromptChunkMutation: () => mocks.upsert,
  useDeletePromptChunkMutation: () => mocks.remove,
  useCompilePromptPreviewMutation: () => mocks.compile,
  useImportResourcePreviewMutation: () => mocks.preview,
  useResourceImageQuery: () => ({ data: undefined }),
}));

const chunk: PromptChunkDto = {
  chunk_id: "chunk-1",
  key: "lighting",
  content: "cinematic lighting",
  category: "Style",
  description: null,
  preview: null,
  created_at_ms: 1,
  updated_at_ms: 1,
};
const CHUNKS = [chunk];

describe("Resources dialogs", () => {
  it("keeps the Library primary and opens editing in a dialog", async () => {
    const user = userEvent.setup();
    renderWorkspace(0);

    expect(screen.getByRole("heading", { name: "Library" })).toBeInTheDocument();
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: /lighting/ }));
    expect(screen.getByRole("dialog", { name: "Edit prompt chunk" })).toBeInTheDocument();
    expect(screen.getByLabelText("Content")).toHaveValue("cinematic lighting");
  });

  it("opens a blank creation dialog when the page requests New", () => {
    const view = renderWorkspace(0);
    view.rerender(workspace(1));

    expect(screen.getByRole("dialog", { name: "New prompt chunk" })).toBeInTheDocument();
    expect(screen.getByLabelText("Content")).toHaveValue("");
  });
});

function renderWorkspace(newRequest: number) {
  return render(workspace(newRequest));
}

function workspace(newRequest: number) {
  return (
    <QueryClientProvider client={createAtelierQueryClient()}>
      <ChunkWorkspace
        chunks={CHUNKS}
        pending={false}
        error={null}
        search=""
        newRequest={newRequest}
      />
    </QueryClientProvider>
  );
}
