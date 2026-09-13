import type { LibraryFolderDto, LibraryResourceDto, ResourceRefDto } from "@/types";

export type ResourcePickerItem = {
  id: string;
  label: string;
  aliases?: ReadonlyArray<string>;
  description?: string | null;
  preview?: ResourceRefDto | null;
};

export type ResolvedResourcePickerItem<Item extends ResourcePickerItem> = {
  item: Item;
  resource: LibraryResourceDto;
};

export type ResourcePickerView<Item extends ResourcePickerItem> = {
  breadcrumbs: ReadonlyArray<LibraryFolderDto>;
  currentFolderId: string | null;
  folders: ReadonlyArray<LibraryFolderDto>;
  items: ReadonlyArray<ResolvedResourcePickerItem<Item>>;
};

export function buildResourcePickerView<Item extends ResourcePickerItem>(
  folders: ReadonlyArray<LibraryFolderDto>,
  resources: ReadonlyArray<LibraryResourceDto>,
  items: ReadonlyArray<Item>,
  requestedFolderId: string | null,
  search: string,
): ResourcePickerView<Item> {
  const itemById = new Map(items.map((item) => [item.id, item]));
  const currentFolder = folders.find(({ folder_id }) => folder_id === requestedFolderId) ?? null;
  const currentFolderId = currentFolder?.folder_id ?? null;
  const currentPath = currentFolder?.path ?? "";
  const query = search.trim().toLocaleLowerCase();
  const visibleFolders = folders.filter((folder) =>
    query
      ? isInSubtree(folder.path, currentPath) && matches(query, folder.path, folderName(folder))
      : folder.parent_id === currentFolderId,
  );
  const visibleItems = resources.flatMap((resource) => {
    const item = itemById.get(resource.resource_id);
    if (!item) return [];
    const visible = query
      ? isInSubtree(resource.path, currentPath) &&
        matches(
          query,
          resource.path,
          resource.display_name,
          resource.identifier,
          ...resource.aliases,
          item.label,
          ...(item.aliases ?? []),
          item.description,
        )
      : resource.folder_id === currentFolderId;
    return visible ? [{ item, resource }] : [];
  });

  return {
    breadcrumbs: breadcrumbFolders(currentFolder, folders),
    currentFolderId,
    folders: visibleFolders,
    items: visibleItems,
  };
}

export function folderName(folder: LibraryFolderDto): string {
  return folder.display_name || folder.identifier;
}

function breadcrumbFolders(
  folder: LibraryFolderDto | null,
  folders: ReadonlyArray<LibraryFolderDto>,
): LibraryFolderDto[] {
  const result: LibraryFolderDto[] = [];
  const visited = new Set<string>();
  let current = folder;
  while (current && !visited.has(current.folder_id)) {
    visited.add(current.folder_id);
    result.unshift(current);
    current = folders.find(({ folder_id }) => folder_id === current?.parent_id) ?? null;
  }
  return result;
}

function isInSubtree(path: string, folderPath: string): boolean {
  return !folderPath || path === folderPath || path.startsWith(`${folderPath}/`);
}

function matches(query: string, ...values: Array<string | null | undefined>): boolean {
  return values.some((value) => value?.toLocaleLowerCase().includes(query));
}
