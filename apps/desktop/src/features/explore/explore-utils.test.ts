import { describe, expect, it } from "vitest";

import type { DanbooruTagDto } from "@/types";

import {
  formatPromptTags,
  formatQueryTags,
  hasRatingMetatag,
  orderSelectedTags,
  selectedRatings,
  shouldBlurRating,
} from "./explore-utils";

describe("Danbooru explore utilities", () => {
  it("keeps rating policy outside the free-form query", () => {
    expect(hasRatingMetatag("1girl rating:e")).toBe(true);
    expect(hasRatingMetatag("1girl -rating:s")).toBe(true);
    expect(hasRatingMetatag("1girl is:nsfw")).toBe(true);
    expect(hasRatingMetatag("1girl order:score")).toBe(false);
    expect(selectedRatings(false)).toEqual(["general", "sensitive"]);
    expect(selectedRatings(true)).toEqual(["general", "sensitive", "questionable", "explicit"]);
  });

  it("orders copied tags by category while preserving order within each group", () => {
    const tags = [
      tag("blue_eyes", "general"),
      tag("character_b", "character"),
      tag("artist_a", "artist"),
      tag("1girl", "general"),
      tag("series_a", "copyright"),
    ];
    const ordered = orderSelectedTags(tags);

    expect(formatPromptTags(ordered)).toBe("artist_a, series_a, character_b, blue_eyes, 1girl");
    expect(formatQueryTags(ordered)).toBe("artist_a series_a character_b blue_eyes 1girl");
  });

  it("blurs every non-general rating when the global setting is enabled", () => {
    expect(shouldBlurRating("general", true)).toBe(false);
    expect(shouldBlurRating("sensitive", true)).toBe(true);
    expect(shouldBlurRating("questionable", true)).toBe(true);
    expect(shouldBlurRating("explicit", false)).toBe(false);
  });
});

function tag(canonicalName: string, category: DanbooruTagDto["category"]): DanbooruTagDto {
  return {
    canonical_name: canonicalName,
    category,
    translation: null,
    post_count: null,
  };
}
