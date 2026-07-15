import { QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { createAtelierQueryClient } from "../app/query-client";
import { LexiconPage } from "../features/lexicon";
import type {
  PromptLexiconCatalogDto,
  PromptLexiconListQueryDto,
  PromptLexiconPageDto,
  PromptLexiconSearchQueryDto,
} from "../types";

const mocks = vi.hoisted(() => ({
  lexiconCatalog: vi.fn<() => Promise<PromptLexiconCatalogDto>>(),
  lexiconList: vi.fn<(request: PromptLexiconListQueryDto) => Promise<PromptLexiconPageDto>>(),
  lexiconSearch: vi.fn<(request: PromptLexiconSearchQueryDto) => Promise<PromptLexiconPageDto>>(),
}));

vi.mock("../platform/atelier", () => ({
  promptApi: {
    lexiconCatalog: mocks.lexiconCatalog,
    lexiconList: mocks.lexiconList,
    lexiconSearch: mocks.lexiconSearch,
  },
  queryKeys: {
    prompt: {
      lexiconCatalog: () => ["prompt", "lexicon", "catalog"],
      lexiconList: (query: PromptLexiconListQueryDto) => ["prompt", "lexicon", "list", query],
      lexiconSearch: (query: PromptLexiconSearchQueryDto) => ["prompt", "lexicon", "search", query],
    },
  },
}));

const catalog: PromptLexiconCatalogDto = {
  stats: {
    total_tags: 120,
    categorized_tags: 120,
    uncategorized_tags: 0,
    matched_weights: 90,
    total_translations: 180,
    tags_with_aliases: 20,
    max_aliases_per_tag: 3,
    source_count: 2,
    manifest_version: 1,
    primary_from_category_json: 120,
    primary_from_manifest_sources: 0,
    primary_fallback_to_tag: 0,
  },
  categories: [
    {
      name: "Characters",
      tag_count: 100,
      subcategory_count: 1,
      subcategories: [{ name: "Hair", tag_count: 40 }],
    },
  ],
};

beforeEach(() => {
  vi.clearAllMocks();
  mocks.lexiconCatalog.mockResolvedValue(catalog);
  mocks.lexiconList.mockImplementation(async (request) =>
    lexiconPage(request.offset, request.limit, 120, "1girl", "one girl"),
  );
  mocks.lexiconSearch.mockImplementation(async (request) =>
    lexiconPage(0, request.limit, 1, "cinematic_lighting", "cinematic lighting"),
  );
});

describe("LexiconPage", () => {
  it("browses the backend catalog and pages category results", async () => {
    const user = setup();

    expect(await screen.findByText("1girl")).toBeInTheDocument();
    expect(mocks.lexiconList).toHaveBeenCalledWith({
      query: "",
      category: null,
      subcategory: null,
      offset: 0,
      limit: 80,
    });

    await user.click(screen.getByRole("button", { name: /Characters/ }));
    await waitFor(() =>
      expect(mocks.lexiconList).toHaveBeenCalledWith({
        query: "",
        category: "Characters",
        subcategory: null,
        offset: 0,
        limit: 80,
      }),
    );
    await user.click(screen.getByRole("button", { name: /Hair/ }));
    await waitFor(() =>
      expect(mocks.lexiconList).toHaveBeenCalledWith({
        query: "",
        category: "Characters",
        subcategory: "Hair",
        offset: 0,
        limit: 80,
      }),
    );
    await user.click(screen.getByRole("button", { name: "Next lexicon page" }));
    await waitFor(() =>
      expect(mocks.lexiconList).toHaveBeenCalledWith(
        expect.objectContaining({
          category: "Characters",
          subcategory: "Hair",
          offset: 80,
          limit: 80,
        }),
      ),
    );
  });

  it("switches to backend search as the user types", async () => {
    const user = setup();

    await user.type(await screen.findByLabelText("Search tags"), "cinematic light");

    await waitFor(() =>
      expect(mocks.lexiconSearch).toHaveBeenLastCalledWith({
        query: "cinematic light",
        limit: 60,
      }),
    );
    expect(await screen.findByText("cinematic_lighting")).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "Search" })).toHaveAttribute("aria-selected", "true");
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

function lexiconPage(
  offset: number,
  limit: number,
  total: number,
  tag: string,
  translation: string,
): PromptLexiconPageDto {
  return {
    items: [
      {
        tag,
        weight: 100,
        category: "Characters",
        subcategory: "Hair",
        primary_translation: translation,
        matched_translation: translation,
        match_field: "tag",
        match_rank: "prefix",
      },
    ],
    total,
    offset,
    limit,
  };
}
