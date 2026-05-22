import { describe, expect, it } from "vitest";

import { appRouteTree, routeNavItems } from "./router";

function getRoutePath(route: unknown): string | undefined {
  if (typeof route !== "object" || route === null || !("options" in route)) {
    return undefined;
  }

  const { options } = route;

  if (typeof options !== "object" || options === null || !("path" in options)) {
    return undefined;
  }

  return typeof options.path === "string" ? options.path : undefined;
}

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
    const rootChildren = appRouteTree.children?.map((child: unknown) => getRoutePath(child));

    expect(rootChildren).toContain("/");
    expect(appRouteTree.options.component).toBeDefined();
  });
});
