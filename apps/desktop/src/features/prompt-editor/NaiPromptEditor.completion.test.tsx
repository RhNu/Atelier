import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";

import { promptEditorText, typeInPromptEditor } from "@/test/prompt-editor-test-utils";
import type {
  LexiconCompleteRequestDto,
  LexiconSearchItemDto,
  LibrarySnapshotDto,
  PromptChunkPageDto,
} from "@/types";

import { NaiPromptEditor } from "./NaiPromptEditor";

const mocks = vi.hoisted(() => ({
  listChunks: vi.fn<() => Promise<PromptChunkPageDto>>(),
  lexiconComplete: vi.fn<(request: LexiconCompleteRequestDto) => Promise<LexiconSearchItemDto[]>>(),
  librarySnapshot: vi.fn<() => Promise<LibrarySnapshotDto>>(),
}));

vi.mock("@/platform/atelier", () => ({
  promptApi: { listChunks: mocks.listChunks },
  lexiconApi: { complete: mocks.lexiconComplete },
  resourceApi: { librarySnapshot: mocks.librarySnapshot },
  queryKeys: {
    prompt: {
      chunks: (request: unknown) => ["prompt", "chunks", request],
    },
    lexicon: {
      completion: (query: string, limit: number) => ["lexicon", "completion", query, limit],
    },
    resource: {
      library: (namespace: string) => ["resource", "library", namespace],
      image: (resource: unknown) => ["resource", "image", resource],
    },
  },
}));

beforeEach(() => {
  vi.clearAllMocks();
  mocks.listChunks.mockResolvedValue(chunkPage());
  mocks.lexiconComplete.mockImplementation(async (request) => [lexiconItem(request.query)]);
  mocks.librarySnapshot.mockResolvedValue(librarySnapshot());
});

describe("NaiPromptEditor completion interactions", () => {
  it("opens and closes manual completion through the editor keyboard", async () => {
    const user = userEvent.setup();
    renderEditor();
    const prompt = screen.getByLabelText("Completion prompt");

    await user.click(prompt);
    await user.keyboard("{Control>} {/Control}");
    expect(await screen.findByRole("option", { name: /lighting/u })).toBeInTheDocument();

    await user.keyboard("{Escape}");
    await waitFor(() => expect(screen.queryByRole("listbox")).not.toBeInTheDocument());
  });

  it.each(["{Enter}", "{Tab}"])("accepts a tag completion with %s", async (key) => {
    const user = userEvent.setup();
    renderEditor();
    const prompt = screen.getByLabelText("Completion prompt");

    typeInPromptEditor(prompt, "cine");
    await screen.findByRole("option", { name: /cinematic_lighting/u });
    await user.keyboard(key);

    expect(promptEditorText(prompt)).toBe("cinematic_lighting, ");
  });

  it("inserts a selected library chunk at the caret", async () => {
    const user = userEvent.setup();
    renderEditor();
    const prompt = screen.getByLabelText("Completion prompt");
    typeInPromptEditor(prompt, "1girl, ");

    await user.keyboard("{Alt>}c{/Alt}");
    const dialog = await screen.findByRole("dialog", { name: "Prompt chunk library" });
    await user.click(within(dialog).getByRole("button", { name: /Lighting\/lighting/u }));

    expect(promptEditorText(prompt)).toBe("1girl, $chunk(Lighting/lighting), ");
    expect(screen.queryByRole("dialog", { name: "Prompt chunk library" })).not.toBeInTheDocument();
  });
});

function renderEditor() {
  return render(
    <QueryClientProvider client={queryClient()}>
      <StatefulEditor />
    </QueryClientProvider>,
  );
}

function StatefulEditor() {
  const [value, setValue] = useState("");
  return <NaiPromptEditor aria-label="Completion prompt" value={value} onChange={setValue} />;
}

function queryClient() {
  return new QueryClient({ defaultOptions: { queries: { retry: false } } });
}

function lexiconItem(query: string): LexiconSearchItemDto {
  return {
    entity_id: 1,
    canonical_name: "cinematic_lighting",
    primary_translation: "cinematic lighting",
    kind: "tag",
    category: "general",
    post_count: 1000,
    rating: "safe",
    matched_text: query,
    match_reason: "canonical_prefix",
    score: 97,
  };
}

function chunkPage(): PromptChunkPageDto {
  return {
    items: [
      {
        chunk_id: "chunk-lighting",
        path: "Lighting/lighting",
        folder_id: null,
        identifier: "lighting",
        display_name: "Lighting",
        aliases: [],
        content: "dramatic light",
        description: null,
        preview: null,
        created_at_ms: 1,
        updated_at_ms: 1,
        models: ["nai-diffusion-4-5-full"],
      },
    ],
    total: 1,
    offset: 0,
    limit: 200,
  };
}

function librarySnapshot(): LibrarySnapshotDto {
  return {
    folders: [],
    resources: [
      {
        resource_id: "chunk-lighting",
        namespace: "prompt_chunk",
        folder_id: null,
        identifier: "lighting",
        display_name: "Lighting",
        aliases: [],
        path: "Lighting/lighting",
        created_at_ms: 1,
        updated_at_ms: 1,
      },
    ],
  };
}
