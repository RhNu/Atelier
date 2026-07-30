import { queryKeys } from "./query-keys";

describe("queryKeys", () => {
  it("keeps command-backed query roots stable", () => {
    expect(queryKeys.app.bootstrap()).toEqual(["app", "bootstrap"]);
    expect(queryKeys.workspace.status()).toEqual(["app", "workspace-status"]);
    expect(queryKeys.account.activeSummary()).toEqual(["app", "account", "active-summary"]);
    expect(queryKeys.gallery.list({ offset: 0, limit: 50 })).toEqual([
      "workspace",
      "gallery",
      "list",
      { offset: 0, limit: 50 },
    ]);
    expect(queryKeys.resource.image({ id: "resource:1", variant_id: null })).toEqual([
      "workspace",
      "resource",
      "image",
      { id: "resource:1", variant_id: null },
    ]);
  });
});
