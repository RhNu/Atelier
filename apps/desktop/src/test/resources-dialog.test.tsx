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
  upsert: {
    isPending: false,
    mutateAsync: vi.fn<(request?: unknown) => Promise<void>>(async () => undefined),
  },
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
  useResourceLibraryQuery: (namespace: string) => {
    const resourceIds =
      {
        prompt_chunk: ["chunk-1"],
        main_preset: ["preset-main"],
        character_preset: ["preset-character"],
        vibe: [],
      }[namespace] ?? [];
    return {
      data: {
        folders:
          namespace === "prompt_chunk"
            ? [
                {
                  folder_id: "folder-style",
                  namespace,
                  parent_id: null,
                  identifier: "styles",
                  display_name: "Styles",
                  path: "styles",
                  created_at_ms: 1,
                  updated_at_ms: 1,
                },
              ]
            : [],
        resources: resourceIds.map((resourceId) => ({
          resource_id: resourceId,
          namespace,
          folder_id: null,
          identifier: resourceId,
          display_name: resourceId,
          aliases: [],
          path: resourceId,
          created_at_ms: 1,
          updated_at_ms: 1,
        })),
      },
      isPending: false,
      isError: false,
    };
  },
  useUpsertLibraryFolderMutation: () => mocks.upsert,
  useUpdateLibraryResourceMutation: () => mocks.upsert,
  useDeleteLibraryFolderMutation: () => mocks.remove,
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
  path: "Style/lighting",
  folder_id: "folder-style",
  identifier: "lighting",
  display_name: "Lighting",
  aliases: [],
  content: "cinematic lighting",
  description: null,
  preview: null,
  created_at_ms: 1,
  updated_at_ms: 1,
  models: ["nai-diffusion-4-5-full"],
};
const CHUNKS = [chunk];
const preset: PromptPresetDto = {
  preset_id: "preset-1",
  kind: "character",
  path: "Characters/hero",
  folder_id: "folder-characters",
  identifier: "hero",
  display_name: "Hero",
  aliases: [],
  description: "Reusable hero details",
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
const mainPreset: PromptPresetDto = {
  ...preset,
  preset_id: "preset-main",
  kind: "main",
  path: "Main styles/cinematic",
  folder_id: "folder-main-styles",
  identifier: "cinematic",
  display_name: "Cinematic",
};
const characterPreset: PromptPresetDto = {
  ...preset,
  preset_id: "preset-character",
  path: "Character archetypes/hero",
  folder_id: "folder-character-archetypes",
};
const MAIN_PRESETS = [mainPreset];
const CHARACTER_PRESETS = [characterPreset];

describe("Resources dialogs", () => {
  it("opens editing in a dialog", async () => {
    const user = userEvent.setup();
    renderWorkspace(0);

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

  it("edits explicit preset identity metadata without legacy ordering", async () => {
    const user = userEvent.setup();
    renderPresetWorkspace();

    await user.click(screen.getByRole("button", { name: /Hero/ }));

    expect(screen.getByRole("dialog", { name: "Edit Character Preset" })).toBeInTheDocument();
    expect(screen.queryByLabelText("Enabled")).not.toBeInTheDocument();
    expect(screen.getByLabelText("Description")).toBeInstanceOf(HTMLTextAreaElement);
    expect(screen.getByLabelText("Display name (optional)")).toHaveValue("Hero");
    expect(screen.getByLabelText("Identifier")).toHaveValue("hero");
    expect(screen.getByText("Characters")).toBeInTheDocument();
    expect(screen.queryByLabelText("Order")).not.toBeInTheDocument();
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
    expect(screen.getByRole("button", { name: "Library" })).toBeInTheDocument();
    expect(screen.queryByRole("heading", { name: "Library" })).not.toBeInTheDocument();
    expect(listView).toHaveAttribute("aria-pressed", "false");
    expect(gridView).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByText("No preview")).toBeInTheDocument();

    await user.click(listView);

    expect(listView).toHaveAttribute("aria-pressed", "true");
    expect(gridView).toHaveAttribute("aria-pressed", "false");
    expect(screen.getByText("cinematic lighting")).toBeInTheDocument();
    expect(screen.queryByText("No preview")).not.toBeInTheDocument();
  });

  it("places folders before resources in the same view container", async () => {
    expect.hasAssertions();
    const user = userEvent.setup();
    render(
      <QueryClientProvider client={createAtelierQueryClient()}>
        <ResourcesPage />
      </QueryClientProvider>,
    );

    assertFolderPrecedesResource();
    await user.click(screen.getByRole("button", { name: "List view" }));
    assertFolderPrecedesResource();
  });

  it("creates folders in place and treats the display name as optional", async () => {
    const user = userEvent.setup();
    render(
      <QueryClientProvider client={createAtelierQueryClient()}>
        <ResourcesPage />
      </QueryClientProvider>,
    );

    await user.click(screen.getByRole("button", { name: "Styles" }));
    await user.click(screen.getByRole("button", { name: "New folder" }));
    expect(screen.queryByLabelText("Parent folder")).not.toBeInTheDocument();
    expect(screen.getByLabelText("Display name (optional)")).toHaveValue("");
    const save = screen.getByRole("button", { name: "Save" });
    expect(save).toBeDisabled();
    await user.type(screen.getByLabelText("Identifier"), "new-folder");
    expect(save).toBeEnabled();
    await user.click(save);
    expect(mocks.upsert.mutateAsync).toHaveBeenLastCalledWith({
      folder_id: null,
      namespace: "prompt_chunk",
      parent_id: "folder-style",
      identifier: "new-folder",
      display_name: "",
    });
  });

  it("keeps main and character preset libraries separate", async () => {
    const user = userEvent.setup();
    render(
      <QueryClientProvider client={createAtelierQueryClient()}>
        <ResourcesPage />
      </QueryClientProvider>,
    );

    await user.click(screen.getByRole("tab", { name: "Main Presets" }));
    expect(screen.getByRole("button", { name: /Cinematic/ })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Hero/ })).not.toBeInTheDocument();
    await user.click(screen.getByRole("tab", { name: "Character Presets" }));
    expect(screen.getByRole("button", { name: /Hero/ })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Cinematic/ })).not.toBeInTheDocument();
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
        defaultModel="nai-diffusion-4-5-full"
      />
    </QueryClientProvider>,
  );
}

function assertFolderPrecedesResource() {
  const folder = screen.getByRole("button", { name: "Styles" }).closest("article");
  const resource = screen.getByRole("button", { name: /Lighting/ });
  expect(folder?.parentElement).toBe(resource.parentElement);
  expect(folder?.nextElementSibling).toBe(resource);
}
