import { QueryClientProvider } from "@tanstack/react-query";
import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { createAtelierQueryClient } from "@/app/query-client";
import { ExplorePage } from "@/features/explore";
import type {
  ExplorePageDto,
  ExploreSearchRequestDto,
  ExploreItemRefDto,
  ExplorePostDetailDto,
  ResourceImageDto,
  DanbooruAccountDto,
  LexiconSearchItemDto,
  LexiconCompleteRequestDto,
} from "@/types";

import { danbooruDetail, danbooruPage, novelaiDetail, novelaiPage } from "./explore-fixtures";

const mocks = vi.hoisted(() => ({
  search: vi.fn<(request: ExploreSearchRequestDto) => Promise<ExplorePageDto>>(),
  detail: vi.fn<(item: ExploreItemRefDto) => Promise<ExplorePostDetailDto>>(),
  media: vi.fn<() => Promise<ResourceImageDto>>(),
  account: vi.fn<() => Promise<DanbooruAccountDto>>(),
  complete: vi.fn<(request: LexiconCompleteRequestDto) => Promise<LexiconSearchItemDto[]>>(),
  copyText: vi.fn<(text: string) => Promise<void>>(),
}));
vi.mock("@/platform/atelier", async (importOriginal) => ({
  ...(await importOriginal<typeof import("@/platform/atelier")>()),
  exploreApi: {
    sources: async () => [
      {
        id: "danbooru_database",
        name: "Danbooru Database",
        experimental: false,
        supports_account: true,
        available: true,
      },
      {
        id: "novelai_explore_gallery",
        name: "NovelAI Explore Gallery",
        experimental: true,
        supports_account: false,
        available: true,
      },
    ],
    search: mocks.search,
    detail: mocks.detail,
    media: mocks.media,
  },
  danbooruApi: { account: mocks.account },
  desktopApi: {
    copyText: mocks.copyText,
    openExternalUrl: vi.fn<(url: string) => Promise<void>>(),
  },
  lexiconApi: { complete: mocks.complete },
  globalSettingsApi: {
    get: async () => ({ frontend: { gallery: { blur_sensitive_images: true } } }),
  },
}));

beforeEach(() => {
  vi.resetAllMocks();
  window.localStorage.clear();
  vi.stubGlobal(
    "IntersectionObserver",
    class {
      observe() {}
      disconnect() {}
    },
  );
  mocks.account.mockResolvedValue({
    configured: false,
    state: "anonymous",
    username: null,
    level: null,
  });
  mocks.complete.mockResolvedValue([]);
  mocks.copyText.mockResolvedValue(undefined);
  mocks.media.mockResolvedValue({ mime_type: "image/png", image_base64: "AQID" });
  mocks.search.mockImplementation(async ({ query }) =>
    query.source_id === "danbooru_database" ? danbooruPage : novelaiPage,
  );
  mocks.detail.mockImplementation(async (item: ExploreItemRefDto) =>
    item.source_id === "danbooru_database"
      ? { source_id: "danbooru_database", detail: danbooruDetail }
      : { source_id: "novelai_explore_gallery", detail: novelaiDetail },
  );
});
afterEach(() => vi.unstubAllGlobals());
function mount() {
  return render(
    <QueryClientProvider client={createAtelierQueryClient()}>
      <ExplorePage />
    </QueryClientProvider>,
  );
}
async function chooseNovelai(user: ReturnType<typeof userEvent.setup>) {
  await screen.findByRole("option", { name: "NovelAI Explore Gallery" });
  await user.selectOptions(
    await screen.findByRole("combobox", { name: "Source" }),
    "novelai_explore_gallery",
  );
  await screen.findByRole("textbox", { name: "NovelAI tags" });
}

describe("ExplorePage", () => {
  it("uses local completion and only submits remote searches explicitly", async () => {
    const user = userEvent.setup();
    mount();
    await user.type(await screen.findByRole("textbox", { name: "Danbooru tag query" }), "blue");
    await waitFor(() =>
      expect(mocks.complete).toHaveBeenLastCalledWith({ query: "blue", limit: 8 }),
    );
    expect(mocks.search).not.toHaveBeenCalled();
    await user.click(screen.getByRole("button", { name: "Search" }));
    await waitFor(() =>
      expect(mocks.search).toHaveBeenCalledExactlyOnceWith({
        query: {
          source_id: "danbooru_database",
          query: { query: "blue", ratings: ["general", "sensitive"] },
        },
        cursor: null,
      }),
    );
    expect(await screen.findByText("#42")).toBeInTheDocument();
  });
  it("blocks rating metatags before remote requests", async () => {
    const user = userEvent.setup();
    mount();
    await user.type(await screen.findByRole("textbox", { name: "Danbooru tag query" }), "rating:e");
    expect(screen.getByRole("button", { name: "Search" })).toBeDisabled();
    expect(mocks.search).not.toHaveBeenCalled();
  });
  it("migrates the adult preference and persists further changes", async () => {
    window.localStorage.setItem("atelier.inspiration.show-adult.v1", "true");
    const user = userEvent.setup();
    mount();
    const toggle = await screen.findByRole("button", { name: "Include questionable and explicit" });
    expect(toggle).toHaveAttribute("aria-pressed", "true");
    await user.click(toggle);
    expect(window.localStorage.getItem("atelier.explore.danbooru.show-adult.v1")).toBe("false");
    await user.click(screen.getByRole("button", { name: "Search" }));
    await waitFor(() =>
      expect(mocks.search).toHaveBeenLastCalledWith(
        expect.objectContaining({
          query: {
            source_id: "danbooru_database",
            query: { query: "", ratings: ["general", "sensitive"] },
          },
        }),
      ),
    );
  });
  it("keeps source drafts isolated when an inactive request completes", async () => {
    let finish!: (page: ExplorePageDto) => void;
    mocks.search.mockImplementationOnce(
      () =>
        new Promise((resolve) => {
          finish = resolve;
        }),
    );
    const user = userEvent.setup();
    mount();
    await user.type(
      await screen.findByRole("textbox", { name: "Danbooru tag query" }),
      "blue_eyes",
    );
    await user.click(screen.getByRole("button", { name: "Search" }));
    await chooseNovelai(user);
    await user.type(screen.getByRole("textbox", { name: "NovelAI tags" }), "landscape");
    await user.click(screen.getByRole("button", { name: "Search" }));
    await screen.findByRole("button", { name: "Inspect NovelAI post: Synthetic sky" });
    await act(async () => finish(danbooruPage));
    expect(
      screen.queryByRole("button", { name: "Inspect Danbooru post 42" }),
    ).not.toBeInTheDocument();
    expect(screen.getByRole("textbox", { name: "NovelAI tags" })).toHaveValue("landscape");
    await user.selectOptions(screen.getByRole("combobox", { name: "Source" }), "danbooru_database");
    expect(screen.getByRole("textbox", { name: "Danbooru tag query" })).toHaveValue("blue_eyes");
    expect(screen.getByRole("button", { name: "Inspect Danbooru post 42" })).toBeInTheDocument();
    expect(mocks.search).toHaveBeenCalledTimes(2);
  });
  it("copies exact weighted prompts and retains independent character metadata", async () => {
    const user = userEvent.setup();
    mount();
    await chooseNovelai(user);
    await user.click(screen.getByRole("button", { name: "Search" }));
    await user.click(await screen.findByRole("button", { name: "Copy Positive prompt" }));
    await waitFor(() => expect(mocks.copyText).toHaveBeenCalledWith(novelaiDetail.metadata.prompt));
    expect(screen.getByText("1.5::blue eyes::")).toBeInTheDocument();
    expect(screen.getByText("red eyes")).toBeInTheDocument();
    expect(screen.getByText("unknown-future-model")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Show this image" })).toBeInTheDocument();
    expect(
      screen.getByText("Some metadata could not be parsed. Available fields are shown below."),
    ).toBeInTheDocument();
  });
  it("preserves random salt on pagination and existing cards after an append failure", async () => {
    mocks.search
      .mockResolvedValueOnce({ ...novelaiPage, next_cursor: "opaque-next" })
      .mockRejectedValueOnce(new Error("rate limited"))
      .mockResolvedValueOnce(novelaiPage)
      .mockResolvedValueOnce(novelaiPage);
    const user = userEvent.setup();
    mount();
    await chooseNovelai(user);
    await user.type(screen.getByRole("textbox", { name: "NovelAI tags" }), "draft");
    await user.selectOptions(screen.getByRole("combobox", { name: "Sort" }), "random");
    expect(screen.getByRole("textbox", { name: "NovelAI tags" })).toBeDisabled();
    await user.click(screen.getByRole("button", { name: "Shuffle" }));
    await user.click(await screen.findByRole("button", { name: "Load more" }));
    await screen.findByText(/Could not load the next page/);
    expect(
      screen.getByRole("button", { name: "Inspect NovelAI post: Synthetic sky" }),
    ).toBeInTheDocument();
    const first = mocks.search.mock.calls[0][0];
    expect(first.query.query).toMatchObject({
      tags: [],
      creator_id: null,
    });
    if (first.query.source_id !== "novelai_explore_gallery") throw new Error("wrong source");
    expect(first.query.query.random_salt).toMatch(/^[a-z0-9]{6}$/);
    expect(mocks.search.mock.calls[1][0]).toEqual({ ...first, cursor: "opaque-next" });
    await user.click(screen.getByRole("button", { name: "Load more" }));
    await waitFor(() => expect(mocks.search).toHaveBeenCalledTimes(3));
    await user.click(screen.getByRole("button", { name: "Shuffle" }));
    await waitFor(() => expect(mocks.search).toHaveBeenCalledTimes(4));
    expect(mocks.search.mock.calls[3][0].query).not.toEqual(first.query);
    await user.selectOptions(screen.getByRole("combobox", { name: "Sort" }), "new");
    expect(screen.getByRole("textbox", { name: "NovelAI tags" })).toHaveValue("draft");
  });
});
