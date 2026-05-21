import { describe, expect, it } from "vitest";

import { appRouteTree, routeNavItems } from "./router";

describe("router", () => {
  it("defines the complete desktop navigation surface", () => {
    expect(routeNavItems.map((item) => item.to)).toEqual([
      "/generate",
      "/director",
      "/resources",
      "/lexicon",
      "/gallery",
      "/settings",
    ]);
  });

  it("redirects root to the generation workbench", () => {
    const rootChildren = appRouteTree.children?.map(
      (child) => (child.options as { path?: string }).path,
    );

    expect(rootChildren).toContain("/");
    expect(appRouteTree.options.component).toBeDefined();
  });
});
