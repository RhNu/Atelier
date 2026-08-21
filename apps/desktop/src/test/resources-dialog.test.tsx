import { QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { createAtelierQueryClient } from "../app/query-client";
import { ResourcesPage } from "../features/resources";
import { ChunkWorkspace } from "../features/resources/components/ChunkWorkspace";
import { PresetWorkspace } from "../features/resources/components/PresetWorkspace";
import type { ListPromptPresetsRequestDto, PromptChunkDto, PromptPresetDto } from "../types";
import { promptEditorText, typeInPromptEditor } from "./prompt-editor-test-utils";

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
  usePromptChunksQuery: () => ({
    data: { items: CHUNKS, total: CHUNKS.length },
    isPending: false,
    isError: false,
  }),
  usePromptPresetsQuery: (request: ListPromptPresetsRequestDto) => ({
    data: {
      items: request.kind === "main" ? MAIN_PRESETS : CHARACTER_PRESETS,
      total: request.kind === "main" ? MAIN_PRESETS.length : CHARACTER_PRESETS.length,
    },
    isPending: false,
    isError: false,
  }),
  useVibeDocumentsQuery: () => ({
    data: { items: [], total: 0 },
    isPending: false,
    isError: false,
  }),
  useUpsertPromptChunkMutation: () => mocks.upsert,
  useDeletePromptChunkMutation: () => mocks.remove,
  useUpsertPromptPresetMutation: () => mocks.upsert,
  useDeletePromptPresetMutation: () => mocks.remove,
  useCompilePromptPreviewMutation: () => mocks.compile,
  useImportResourcePreviewMutation: () => mocks.preview,
  useResourceImageQuery: () => ({ data: undefined }),
}));

vi.mock("../features/generation/data/useImageModelCatalog", () => ({
  useImageModelCatalog: () => ({
    data: [
      { model: "nai-diffusion-5-full" },
      { model: "nai-diffusion-4-5-full" },
      { model: "nai-diffusion-3" },
    ],
  }),
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
  models: ["nai-diffusion-4-5-full"],
};
const CHUNKS = [chunk];
const CHUNK_CATEGORIES = ["Style"];
const preset: PromptPresetDto = {
  preset_id: "preset-1",
  kind: "character",
  name: "Hero",
  category: "Characters",
  description: "Reusable hero details",
  order: 2,
  prompt_behavior: { mode: "surround", before: "hero", after: "" },
  uc_behavior: { mode: "surround", before: "", after: "" },
  quality_override: null,
  uc_preset_override: null,
  preview: null,
  created_at_ms: 1,
  updated_at_ms: 1,
  models: ["nai-diffusion-4-5-full"],
};
const PRESETS = [preset];
const PRESET_CATEGORIES = ["Characters", "Style"];
const mainPreset: PromptPresetDto = {
  ...preset,
  preset_id: "preset-main",
  kind: "main",
  name: "Cinematic",
  category: "Main styles",
};
const characterPreset: PromptPresetDto = {
  ...preset,
  preset_id: "preset-character",
  category: "Character archetypes",
};
const MAIN_PRESETS = [mainPreset];
const CHARACTER_PRESETS = [characterPreset];

describe("Resources dialogs", () => {
  it("keeps the Library primary and opens editing in a dialog", async () => {
    const user = userEvent.setup();
    renderWorkspace(0);

    expect(screen.getByRole("heading", { name: "Library" })).toBeInTheDocument();
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: /lighting/ }));
    expect(screen.getByRole("dialog", { name: "Edit prompt chunk" })).toBeInTheDocument();
    expect(promptEditorText(screen.getByLabelText("Content"))).toBe("cinematic lighting");
    expect(screen.queryByText("Reusable $chunk(...) source")).not.toBeInTheDocument();
  });

  it("opens a blank creation dialog when the page requests New", () => {
    const view = renderWorkspace(0);
    view.rerender(workspace(1));

    expect(screen.getByRole("dialog", { name: "New prompt chunk" })).toBeInTheDocument();
    expect(screen.getAllByText("New prompt chunk")).toHaveLength(1);
    expect(screen.queryByRole("button", { name: "New" })).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Create" })).toBeInTheDocument();
    expect(promptEditorText(screen.getByLabelText("Content"))).toBe("");
  });

  it("uses compact preset metadata and keeps order in advanced settings", async () => {
    const user = userEvent.setup();
    renderPresetWorkspace();

    await user.click(screen.getByRole("button", { name: /Hero/ }));

    expect(screen.getByRole("dialog", { name: "Edit Character Preset" })).toBeInTheDocument();
    expect(screen.queryByLabelText("Enabled")).not.toBeInTheDocument();
    expect(screen.getByLabelText("Description")).toBeInstanceOf(HTMLTextAreaElement);
    const category = screen.getByRole("combobox", { name: "Category" });
    expect(category).toHaveAttribute("aria-autocomplete", "list");
    expect(category).toHaveValue("Characters");
    await user.click(category);
    expect(screen.getByRole("listbox", { name: "Category" })).toHaveClass("bg-app-panel");
    expect(screen.getByRole("option", { name: "Style" })).toBeInTheDocument();

    const advanced = screen.getByText("Advanced settings").closest("details");
    expect(advanced).not.toHaveAttribute("open");
  });

  it("preserves both prompt behavior buffers while switching tabs", async () => {
    const user = userEvent.setup();
    renderPresetWorkspace();

    await user.click(screen.getByRole("button", { name: /Hero/ }));
    expect(screen.getByLabelText("Before")).toBeInTheDocument();
    expect(screen.getByLabelText("After")).toBeInTheDocument();
    typeInPromptEditor(screen.getByLabelText("After"), "detailed");

    await user.click(screen.getByRole("tab", { name: "Replace entirely" }));

    expect(screen.getByLabelText("Replace")).toBeInTheDocument();
    expect(screen.queryByLabelText("Before")).not.toBeInTheDocument();
    expect(screen.queryByLabelText("After")).not.toBeInTheDocument();
    typeInPromptEditor(screen.getByLabelText("Replace"), "villain");

    await user.click(screen.getByRole("tab", { name: "Add before / after" }));
    expect(promptEditorText(screen.getByLabelText("Before"))).toBe("hero");
    expect(promptEditorText(screen.getByLabelText("After"))).toBe("detailed");

    await user.click(screen.getByRole("tab", { name: "Replace entirely" }));
    expect(promptEditorText(screen.getByLabelText("Replace"))).toBe("villain");
  });

  it("uses an icon-only placeholder for an empty resource list", () => {
    render(workspace(0, []));

    expect(screen.getByRole("img", { name: "No prompt chunks" })).toBeInTheDocument();
    expect(screen.queryByText("No prompt chunks")).not.toBeInTheDocument();
  });

  it("defaults to preview-first grid cards and switches to information-dense list rows", async () => {
    const user = userEvent.setup();
    render(
      <QueryClientProvider client={createAtelierQueryClient()}>
        <ResourcesPage />
      </QueryClientProvider>,
    );

    const listView = screen.getByRole("button", { name: "List view" });
    const gridView = screen.getByRole("button", { name: "Grid view" });
    expect(listView).toHaveAttribute("aria-pressed", "false");
    expect(gridView).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByText("No preview")).toBeInTheDocument();

    await user.click(listView);

    expect(listView).toHaveAttribute("aria-pressed", "true");
    expect(gridView).toHaveAttribute("aria-pressed", "false");
    expect(screen.getByText("cinematic lighting")).toBeInTheDocument();
    expect(screen.queryByText("No preview")).not.toBeInTheDocument();
  });

  it("keeps main and character preset category suggestions separate", async () => {
    const user = userEvent.setup();
    render(
      <QueryClientProvider client={createAtelierQueryClient()}>
        <ResourcesPage />
      </QueryClientProvider>,
    );

    await user.click(screen.getByRole("tab", { name: "Main Presets" }));
    await user.click(screen.getByRole("button", { name: "New" }));
    await user.click(screen.getByRole("combobox", { name: "Category" }));

    expect(screen.getByRole("option", { name: "Main styles" })).toBeInTheDocument();
    expect(screen.queryByRole("option", { name: "Character archetypes" })).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Close" }));
    await user.click(screen.getByRole("tab", { name: "Character Presets" }));
    await user.click(screen.getByRole("button", { name: "New" }));
    await user.click(screen.getByRole("combobox", { name: "Category" }));

    expect(screen.getByRole("option", { name: "Character archetypes" })).toBeInTheDocument();
    expect(screen.queryByRole("option", { name: "Main styles" })).not.toBeInTheDocument();
  });
});

function renderWorkspace(newRequest: number) {
  return render(workspace(newRequest));
}

function workspace(newRequest: number, chunks: ReadonlyArray<PromptChunkDto> = CHUNKS) {
  return (
    <QueryClientProvider client={createAtelierQueryClient()}>
      <ChunkWorkspace
        chunks={chunks}
        pending={false}
        error={null}
        search=""
        newRequest={newRequest}
        viewMode="list"
        categorySuggestions={CHUNK_CATEGORIES}
        defaultModel="nai-diffusion-4-5-full"
      />
    </QueryClientProvider>
  );
}

function renderPresetWorkspace() {
  return render(
    <QueryClientProvider client={createAtelierQueryClient()}>
      <PresetWorkspace
        kind="character"
        presets={PRESETS}
        pending={false}
        error={null}
        search=""
        newRequest={0}
        viewMode="list"
        categorySuggestions={PRESET_CATEGORIES}
        defaultModel="nai-diffusion-4-5-full"
      />
    </QueryClientProvider>,
  );
}
