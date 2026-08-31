import type { DanbooruPostDetailDto, NovelAiExplorePostDetailDto, ExplorePageDto } from "@/types";

export const danbooruDetail: DanbooruPostDetailDto = {
  post: {
    id: 42,
    rating: "general",
    width: 1024,
    height: 768,
    score: 9,
    favorite_count: 3,
    file_extension: "jpg",
    tag_count: 2,
    has_preview: false,
    has_sample: false,
  },
  created_at: "2026-08-31T00:00:00Z",
  file_size: 1024,
  source_url: null,
  danbooru_url: "https://danbooru.donmai.us/posts/42",
  tags: [
    { canonical_name: "blue_eyes", category: "general", translation: "蓝眼睛", post_count: 100 },
  ],
};
export const novelaiDetail: NovelAiExplorePostDetailDto = {
  post: {
    id: "00000000-0000-0000-0000-000000000001",
    title: "Synthetic sky",
    creator_id: "creator-one",
    creator_name: "Example creator",
    width: 832,
    height: 1216,
    like_count: 5,
  },
  created_at: "2026-08-31T00:00:00Z",
  description: "Synthetic test metadata",
  page_url: "https://novelai.net/explore/gallery",
  metadata: {
    status: "partial",
    prompt: "  1.2::blue sky::,\n landscape  ",
    negative_prompt: "low quality",
    characters: [
      { text: "1.5::blue eyes::", centers: [{ x: 0.2, y: 0.7 }] },
      { text: "green eyes", centers: [] },
    ],
    negative_characters: [{ text: "red eyes", centers: [] }],
    use_coords: true,
    use_order: false,
    negative_use_coords: null,
    negative_use_order: null,
    parameters: [
      { name: "model_name", value: "unknown-future-model" },
      { name: "seed", value: "123" },
    ],
    raw: "synthetic raw metadata",
    warnings: ["Synthetic partial metadata warning"],
  },
};
export const danbooruPage: ExplorePageDto = {
  items: [{ source_id: "danbooru_database", post: danbooruDetail.post }],
  next_cursor: null,
  total: null,
  authenticated: false,
};
export const novelaiPage: ExplorePageDto = {
  items: [{ source_id: "novelai_explore_gallery", post: novelaiDetail.post }],
  next_cursor: null,
  total: 1,
  authenticated: false,
};
