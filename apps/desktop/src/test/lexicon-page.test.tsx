import { QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { createAtelierQueryClient } from "../app/query-client";
import { LexiconPage } from "../features/lexicon";
import type {
  AppendLexiconEntitiesRequestDto,
  GenerationDraftDto,
  LexiconBootstrapDto,
  LexiconEntityDetailDto,
  LexiconEntityRequestDto,
  LexiconSearchItemDto,
  LexiconSearchPageDto,
  LexiconSearchRequestDto,
} from "../types";

const mocks = vi.hoisted(() => ({
  navigate: vi.fn<(options: { to: string }) => Promise<void>>(),
  bootstrap: vi.fn<() => Promise<LexiconBootstrapDto>>(),
  search: vi.fn<(request: LexiconSearchRequestDto) => Promise<LexiconSearchPageDto>>(),
  entity: vi.fn<(request: LexiconEntityRequestDto) => Promise<LexiconEntityDetailDto>>(),
  append: vi.fn<(request: AppendLexiconEntitiesRequestDto) => Promise<GenerationDraftDto>>(),
}));

vi.mock("@tanstack/react-router", async (importOriginal) => ({
  ...(await importOriginal<typeof import("@tanstack/react-router")>()),
  useNavigate: () => mocks.navigate,
}));

vi.mock("../platform/atelier", () => ({
  lexiconApi: {
    bootstrap: mocks.bootstrap,
    search: mocks.search,
    entity: mocks.entity,
  },
  generationApi: {
    appendLexiconEntities: mocks.append,
  },
  queryKeys: {
    lexicon: {
      bootstrap: () => ["lexicon", "bootstrap"],
      search: (request: LexiconSearchRequestDto) => ["lexicon", "search", request],
      entity: (entityId: number | null) => ["lexicon", "entity", entityId],
    },
    generation: {
      draft: () => ["generation", "draft"],
    },
  },
}));

const item: LexiconSearchItemDto = {
  entity_id: 123,
  canonical_name: "cinematic_lighting",
  primary_translation: "cinematic lighting",
  kind: "tag",
  category: "general",
  post_count: 12_000,
  rating: "safe",
  matched_text: "cinematic",
  match_reason: "canonical_prefix",
  score: 99,
};

const bootstrap: LexiconBootstrapDto = {
  bundle_version: "2026.07.29",
  status: {
    lexical_available: true,
    semantic_available: true,
    message: null,
  },
  stats: {
    total_entities: 52_100,
    tag_entities: 50_000,
    artist_entities: 2_100,
    sensitive_entities: 500,
    translation_count: 50_000,
    group_count: 1,
  },
  categories: [{ value: "general", label: "General", count: 40_000 }],
  groups: [{ id: "lighting", name: "Lighting", member_count: 100 }],
};

beforeEach(() => {
  localStorage.clear();
  vi.clearAllMocks();
  mocks.bootstrap.mockResolvedValue(bootstrap);
  mocks.search.mockResolvedValue({ items: [item], total: 1, offset: 0, limit: 100 });
  mocks.entity.mockResolvedValue({
    entity: item,
    translations: [{ locale: "zh-CN", text: "电影光效" }],
    aliases: ["cinematic_light"],
    wiki: [{ locale: "en", text: "Dramatic lighting used in cinematic compositions." }],
    groups: [{ id: "lighting", name: "Lighting", member_count: 100 }],
    related: [],
  });
  mocks.append.mockResolvedValue(generationDraft());
  mocks.navigate.mockResolvedValue(undefined);
});

describe("LexiconPage", () => {
  it("keeps entity details in the results split layout", async () => {
    setup();

    await screen.findByText("cinematic_lighting");
    expect(screen.getByText("Select an entity to inspect its metadata")).toBeInTheDocument();
    expect(screen.queryByText("Loading entity details")).not.toBeInTheDocument();
    expect(mocks.entity).not.toHaveBeenCalled();
    expect(screen.getByRole("region", { name: "Entity details" })).toHaveClass(
      "min-h-0",
      "overflow-hidden",
    );
    expect(screen.queryByRole("dialog", { name: "Entity details" })).not.toBeInTheDocument();
  });

  it("searches with local filters and switches to semantic exploration", async () => {
    const user = setup();

    expect(await screen.findByText("cinematic_lighting")).toBeInTheDocument();
    expect(mocks.search).toHaveBeenCalledWith({
      query: "",
      mode: "lexical",
      filters: {
        entity_kinds: [],
        categories: [],
        group_ids: [],
        ratings: [],
      },
      selected_entity_ids: [],
      offset: 0,
      limit: 100,
    });

    await user.type(screen.getByLabelText("Search the lexicon"), "cinematic");
    await user.selectOptions(screen.getByLabelText("Type"), "tag");
    await user.selectOptions(screen.getByLabelText("Content"), "safe");
    await user.click(screen.getByRole("tab", { name: "Semantic explore" }));
    expect(mocks.search.mock.lastCall?.[0].mode).toBe("lexical");
    await user.click(screen.getByRole("button", { name: "Search" }));

    await waitFor(() => {
      const request = mocks.search.mock.lastCall?.[0];
      expect(request?.query).toBe("cinematic");
      expect(request?.mode).toBe("semantic");
      expect(request?.filters.entity_kinds).toEqual(["tag"]);
      expect(request?.filters.ratings).toEqual(["safe"]);
    });
    expect(localStorage.getItem("atelier.lexicon.rating.v1")).toBe("safe");
  });

  it("clears text separately from resetting all active filters", async () => {
    const user = setup();

    await screen.findByText("cinematic_lighting");
    const search = screen.getByLabelText("Search the lexicon");
    await user.type(search, "light");
    await user.selectOptions(screen.getByLabelText("Tag group"), "lighting");
    await user.click(screen.getByRole("button", { name: "Clear search text" }));

    expect(search).toHaveValue("");
    expect(screen.getByLabelText("Tag group")).toHaveValue("lighting");
    await waitFor(() => {
      const request = mocks.search.mock.lastCall?.[0];
      expect(request?.query).toBe("");
      expect(request?.filters.group_ids).toEqual(["lighting"]);
    });

    await user.click(screen.getByRole("button", { name: "Reset search and filters" }));

    expect(screen.getByLabelText("Tag group")).toHaveValue("");
    expect(screen.getByLabelText("Content")).toHaveValue("all");
    expect(localStorage.getItem("atelier.lexicon.rating.v1")).toBe("all");
  });

  it("locks semantic controls while an explicitly submitted search is pending", async () => {
    let resolveSemantic:
      | ((value: LexiconSearchPageDto | PromiseLike<LexiconSearchPageDto>) => void)
      | undefined;
    const semanticResult = new Promise<LexiconSearchPageDto>((resolve) => {
      resolveSemantic = resolve;
    });
    mocks.search.mockImplementation((request) =>
      request.mode === "semantic"
        ? semanticResult
        : Promise.resolve({
            items: [item],
            total: 1,
            offset: request.offset,
            limit: request.limit,
          }),
    );
    const user = setup();

    await screen.findByText("cinematic_lighting");
    const search = screen.getByLabelText("Search the lexicon");
    await user.type(search, "dramatic light");
    await user.click(screen.getByRole("tab", { name: "Semantic explore" }));
    await user.click(screen.getByRole("button", { name: "Search" }));

    await waitFor(() => expect(search).toBeDisabled());
    expect(screen.getByRole("button", { name: "Searching…" })).toBeDisabled();
    expect(screen.getByText("Searching the lexicon")).toBeInTheDocument();

    resolveSemantic?.({ items: [item], total: 1, offset: 0, limit: 100 });
    await waitFor(() => expect(search).toBeEnabled());
  });

  it("pages through the full filtered result count", async () => {
    mocks.search.mockImplementation(async (request) => ({
      items: [{ ...item, entity_id: request.offset + 123 }],
      total: 410,
      offset: request.offset,
      limit: request.limit,
    }));
    const user = setup();

    await screen.findByText("1–1 of 410");
    await user.click(screen.getByRole("button", { name: "Next results" }));

    await waitFor(() => expect(mocks.search.mock.lastCall?.[0].offset).toBe(100));
    expect(await screen.findByText("101–101 of 410")).toBeInTheDocument();
  });

  it("loads details and atomically hands the basket to the generation draft", async () => {
    const user = setup();

    await screen.findByText("cinematic_lighting");
    await user.click(screen.getByText("cinematic_lighting"));
    expect(await screen.findByText("cinematic_light")).toBeInTheDocument();
    const addButtons = screen.getAllByRole("button", { name: "Add to tag basket" });
    const inspectorAddButton = addButtons.at(1);
    if (!inspectorAddButton) throw new Error("inspector basket button was not rendered");
    await user.click(inspectorAddButton);
    await user.click(screen.getByRole("button", { name: /Add to prompt/u }));

    await waitFor(() =>
      expect(mocks.append).toHaveBeenCalledWith({
        target: "positive",
        entity_ids: [123],
      }),
    );
    expect(mocks.navigate).toHaveBeenCalledWith({ to: "/generate" });
  });
});

function setup() {
  const user = userEvent.setup();
  render(
    <QueryClientProvider client={createAtelierQueryClient()}>
      <LexiconPage />
    </QueryClientProvider>,
  );
  return user;
}

function generationDraft(): GenerationDraftDto {
  return {
    model: "nai-diffusion-4-5-full",
    prompt_states: [
      {
        model: "nai-diffusion-4-5-full",
        main_preset_id: null,
        prompt: "cinematic_lighting, ",
        negative_prompt: "",
        characters: [],
        character_position_mode: "global",
      },
    ],
    size: { width: 832, height: 1216 },
    quality: "standard",
    transparent_background: false,
    uc_preset: "light",
    steps: 23,
    scale: 5,
    sampler: "k_euler_ancestral",
    noise_schedule: "karras",
    seed_mode: "random",
    seed: 0,
    n_samples: 1,
    request_count: 1,
    cfg_rescale: 0,
    variety_boost: false,
    image_format: null,
    strict_mode: false,
    stream_enabled: true,
    i2i: null,
    vibe: { enabled: false, strength: 1, slots: [] },
    precise_references: [],
  };
}
