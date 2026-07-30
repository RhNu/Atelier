import { QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { createAtelierQueryClient } from "@/app/query-client";
import { InspirationPage } from "@/features/inspiration";
import type {
  DanbooruAccountDto,
  DanbooruMediaRequestDto,
  DanbooruPostDetailDto,
  DanbooruPostDetailRequestDto,
  DanbooruPostPageDto,
  DanbooruSearchRequestDto,
  GlobalSettingsDto,
  LexiconCompleteRequestDto,
  LexiconSearchItemDto,
  ResourceImageDto,
} from "@/types";

const mocks = vi.hoisted(() => ({
  danbooruApi: {
    account: vi.fn<() => Promise<DanbooruAccountDto>>(),
    search: vi.fn<(request: DanbooruSearchRequestDto) => Promise<DanbooruPostPageDto>>(),
    detail: vi.fn<(request: DanbooruPostDetailRequestDto) => Promise<DanbooruPostDetailDto>>(),
    media: vi.fn<(request: DanbooruMediaRequestDto) => Promise<ResourceImageDto>>(),
  },
  desktopApi: {
    copyText: vi.fn<(text: string) => Promise<void>>(),
    openExternalUrl: vi.fn<(url: string) => Promise<void>>(),
  },
  globalSettingsApi: {
    get: vi.fn<() => Promise<GlobalSettingsDto>>(),
  },
  lexiconApi: {
    complete: vi.fn<(request: LexiconCompleteRequestDto) => Promise<LexiconSearchItemDto[]>>(),
  },
}));

vi.mock("@/platform/atelier", () => ({
  danbooruApi: mocks.danbooruApi,
  desktopApi: mocks.desktopApi,
  globalSettingsApi: mocks.globalSettingsApi,
  lexiconApi: mocks.lexiconApi,
  resourceImageToDataUrl: () => "data:image/png;base64,AQID",
  queryKeys: {
    app: {
      globalSettings: () => ["app", "settings"],
    },
    danbooru: {
      root: () => ["app", "danbooru"],
      account: () => ["app", "danbooru", "account"],
      search: (request: Omit<DanbooruSearchRequestDto, "before_id">) => [
        "app",
        "danbooru",
        "search",
        request,
      ],
      detail: (postId: number | null) => ["app", "danbooru", "detail", postId],
      media: (postId: number, variant: string) => ["app", "danbooru", "media", postId, variant],
    },
    lexicon: {
      completion: (query: string, limit: number) => [
        "workspace",
        "lexicon",
        "completion",
        query,
        limit,
      ],
    },
  },
}));

beforeEach(() => {
  vi.clearAllMocks();
  window.localStorage.clear();
  mocks.danbooruApi.account.mockResolvedValue({
    configured: false,
    state: "anonymous",
    username: null,
    level: null,
  });
  mocks.globalSettingsApi.get.mockResolvedValue({
    last_workspace: "D:/atelier",
    frontend: {
      language: "en",
      developer_mode: false,
      gallery: { blur_sensitive_images: false },
    },
    safety: { wd_auto_review_enabled: false },
  });
  mocks.lexiconApi.complete.mockResolvedValue([]);
  mocks.danbooruApi.search.mockResolvedValue({
    items: [postSummary],
    next_before_id: null,
    auth_mode: "anonymous",
  });
  mocks.danbooruApi.detail.mockResolvedValue(postDetail);
});

describe("InspirationPage", () => {
  it("uses local completion while typing and submits one Danbooru search explicitly", async () => {
    const user = userEvent.setup();
    render(
      <QueryClientProvider client={createAtelierQueryClient()}>
        <InspirationPage />
      </QueryClientProvider>,
    );

    const input = await screen.findByRole("textbox", { name: "Danbooru tag query" });
    await user.type(input, "blue");

    expect(mocks.danbooruApi.search).not.toHaveBeenCalled();
    await waitFor(() =>
      expect(mocks.lexiconApi.complete).toHaveBeenLastCalledWith({
        query: "blue",
        limit: 8,
      }),
    );

    await user.click(screen.getByRole("button", { name: "Search" }));

    await waitFor(() =>
      expect(mocks.danbooruApi.search).toHaveBeenCalledWith({
        query: "blue",
        ratings: ["general", "sensitive"],
        before_id: null,
      }),
    );
    expect(mocks.danbooruApi.search).toHaveBeenCalledTimes(1);
    expect(await screen.findByText("#42")).toBeInTheDocument();
  });

  it("blocks rating metatags before any remote request", async () => {
    const user = userEvent.setup();
    render(
      <QueryClientProvider client={createAtelierQueryClient()}>
        <InspirationPage />
      </QueryClientProvider>,
    );

    await user.type(await screen.findByRole("textbox", { name: "Danbooru tag query" }), "rating:e");

    expect(screen.getByRole("button", { name: "Search" })).toBeDisabled();
    expect(mocks.danbooruApi.search).not.toHaveBeenCalled();
  });

  it("persists the adult-rating toggle and uses it for the next search", async () => {
    const user = userEvent.setup();
    const firstRender = render(
      <QueryClientProvider client={createAtelierQueryClient()}>
        <InspirationPage />
      </QueryClientProvider>,
    );

    const toggle = await screen.findByRole("button", { name: "Include questionable and explicit" });
    await user.click(toggle);
    expect(toggle).toHaveAttribute("aria-pressed", "true");
    expect(window.localStorage.getItem("atelier.inspiration.show-adult.v1")).toBe("true");

    firstRender.unmount();
    render(
      <QueryClientProvider client={createAtelierQueryClient()}>
        <InspirationPage />
      </QueryClientProvider>,
    );

    await user.type(await screen.findByRole("textbox", { name: "Danbooru tag query" }), "blue");
    await user.click(screen.getByRole("button", { name: "Search" }));
    await waitFor(() =>
      expect(mocks.danbooruApi.search).toHaveBeenLastCalledWith({
        query: "blue",
        ratings: ["general", "sensitive", "questionable", "explicit"],
        before_id: null,
      }),
    );
  });
});

const postSummary = {
  id: 42,
  rating: "general" as const,
  width: 1024,
  height: 768,
  score: 9,
  favorite_count: 3,
  file_extension: "jpg",
  tag_count: 2,
  has_preview: false,
  has_sample: false,
};

const postDetail: DanbooruPostDetailDto = {
  post: postSummary,
  created_at: "2026-07-30T00:00:00Z",
  file_size: 1024,
  source_url: null,
  danbooru_url: "https://danbooru.donmai.us/posts/42",
  tags: [
    {
      canonical_name: "blue_eyes",
      category: "general",
      translation: "蓝眼睛",
      post_count: 100,
    },
  ],
};
