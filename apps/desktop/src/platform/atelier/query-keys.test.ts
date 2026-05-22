import { queryKeys } from "./query-keys";

describe("queryKeys", () => {
  it("keeps command-backed query roots stable", () => {
    expect(queryKeys.workspace.status()).toEqual(["workspace", "status"]);
    expect(queryKeys.account.keyProbe("main")).toEqual(["account", "key-probe", "main"]);
    expect(queryKeys.gallery.list({ offset: 0, limit: 50 })).toEqual([
      "gallery",
      "list",
      { offset: 0, limit: 50 },
    ]);
    expect(queryKeys.resource.image({ id: "resource:1", variant_id: null })).toEqual([
      "resource",
      "image",
      { id: "resource:1", variant_id: null },
    ]);
  });
});
