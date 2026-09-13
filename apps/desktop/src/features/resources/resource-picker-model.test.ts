import type { LibraryFolderDto, LibraryResourceDto } from "@/types";

import { buildResourcePickerView } from "./resource-picker-model";

const folders: LibraryFolderDto[] = [
  folder("characters", null, "Characters"),
  folder("outfits", "characters", "Characters/Outfits"),
  folder("styles", null, "Styles"),
];
const resources: LibraryResourceDto[] = [
  resource("root", null, "root"),
  resource("hero", "characters", "Characters/hero"),
  resource("uniform", "outfits", "Characters/Outfits/uniform"),
  resource("cinematic", "styles", "Styles/cinematic"),
];
const items = resources.map(({ resource_id, display_name }) => ({
  id: resource_id,
  label: display_name,
}));

describe("resource picker model", () => {
  it("shows direct child folders and items at the current level", () => {
    const root = buildResourcePickerView(folders, resources, items, null, "");
    expect(root.folders.map(({ folder_id }) => folder_id)).toEqual(["characters", "styles"]);
    expect(root.items.map(({ item }) => item.id)).toEqual(["root"]);

    const characters = buildResourcePickerView(folders, resources, items, "characters", "");
    expect(characters.breadcrumbs.map(({ folder_id }) => folder_id)).toEqual(["characters"]);
    expect(characters.folders.map(({ folder_id }) => folder_id)).toEqual(["outfits"]);
    expect(characters.items.map(({ item }) => item.id)).toEqual(["hero"]);
  });

  it("searches the current subtree and keeps the canonical resource path", () => {
    const view = buildResourcePickerView(folders, resources, items, "characters", "uniform");
    expect(view.folders).toEqual([]);
    expect(view.items).toHaveLength(1);
    expect(view.items[0]?.resource.path).toBe("Characters/Outfits/uniform");
  });

  it("returns to the root when a requested folder no longer exists", () => {
    const view = buildResourcePickerView(folders, resources, items, "missing", "");
    expect(view.currentFolderId).toBeNull();
    expect(view.breadcrumbs).toEqual([]);
    expect(view.items.map(({ item }) => item.id)).toEqual(["root"]);
  });
});

function folder(id: string, parentId: string | null, path: string): LibraryFolderDto {
  return {
    folder_id: id,
    namespace: "prompt_chunk",
    parent_id: parentId,
    identifier: path.split("/").at(-1)?.toLocaleLowerCase() ?? id,
    display_name: path.split("/").at(-1) ?? id,
    path,
    created_at_ms: 1,
    updated_at_ms: 1,
  };
}

function resource(id: string, folderId: string | null, path: string): LibraryResourceDto {
  return {
    resource_id: id,
    namespace: "prompt_chunk",
    folder_id: folderId,
    identifier: path.split("/").at(-1) ?? id,
    display_name: id,
    aliases: [],
    path,
    created_at_ms: 1,
    updated_at_ms: 1,
  };
}
