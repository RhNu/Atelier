/* eslint-disable max-lines, max-lines-per-function, react-perf/jsx-no-new-function-as-prop */
import { ChevronRight, Folder, FolderPen, FolderPlus, Home, Trash2 } from "lucide-react";
import { Fragment, useState, type DragEvent, type ReactNode } from "react";
import { useTranslation } from "react-i18next";

import { AppButton, AppIconButton, AppModal, EmptyState } from "@/components/ui";
import type {
  LibraryFolderDto,
  LibraryNamespaceDto,
  LibraryResourceDto,
  UpsertLibraryFolderRequestDto,
} from "@/types";

import {
  useDeleteLibraryFolderMutation,
  useResourceLibraryQuery,
  useUpdateLibraryResourceMutation,
  useUpsertLibraryFolderMutation,
} from "../data/useResourcesData";
import { formatError, matchesSearch, type ResourceViewMode } from "../resource-model";
import { TextInput } from "./ResourceEditorPrimitives";

const DRAG_TYPE = "application/x-atelier-library-node";

type DraggedNode = { kind: "folder" | "resource"; id: string };

export function LibraryNavigator({
  namespace,
  search,
  viewMode,
  children,
}: {
  namespace: LibraryNamespaceDto;
  search: string;
  viewMode: ResourceViewMode;
  children: (
    visibleResourceIds: ReadonlySet<string>,
    startDrag: StartDrag,
    currentFolder: CurrentFolder,
    resources: ReadonlyArray<LibraryResourceDto>,
    folderItems: ReactNode,
  ) => ReactNode;
}) {
  const { t } = useTranslation("resources");
  const snapshotQuery = useResourceLibraryQuery(namespace);
  const upsertFolder = useUpsertLibraryFolderMutation();
  const updateResource = useUpdateLibraryResourceMutation();
  const deleteFolder = useDeleteLibraryFolderMutation();
  const [currentFolderId, setCurrentFolderId] = useState<string | null>(null);
  const [editingFolder, setEditingFolder] = useState<LibraryFolderDto | null | undefined>();
  const [deletingFolder, setDeletingFolder] = useState<LibraryFolderDto | null>(null);
  const [error, setError] = useState<string | null>(null);
  const snapshot = snapshotQuery.data;
  const folders = snapshot?.folders ?? [];
  const resources = snapshot?.resources ?? [];
  const currentFolder = folders.find(({ folder_id }) => folder_id === currentFolderId) ?? null;
  const currentPath = currentFolder?.path ?? "";

  const visibleFolders = (() => {
    const query = search.trim();
    if (!query) return folders.filter(({ parent_id }) => parent_id === currentFolderId);
    return folders.filter(
      (folder) =>
        isInSubtree(folder.path, currentPath) &&
        matchesSearch(query, folder.path, folder.display_name),
    );
  })();
  const visibleResourceIds = (() => {
    const query = search.trim();
    return new Set(
      resources
        .filter((resource) =>
          query
            ? isInSubtree(resource.path, currentPath) &&
              matchesSearch(query, resource.path, resource.display_name, ...resource.aliases)
            : resource.folder_id === currentFolderId,
        )
        .map(({ resource_id }) => resource_id),
    );
  })();
  const breadcrumbs = breadcrumbFolders(currentFolder, folders);

  function report(action: Promise<unknown>) {
    setError(null);
    void action.catch((cause: unknown) => setError(formatError(cause)));
  }

  function dropInto(event: DragEvent, folderId: string | null) {
    event.preventDefault();
    const dragged = readDraggedNode(event);
    if (!dragged || dragged.id === folderId) return;
    if (dragged.kind === "folder") {
      const folder = folders.find(({ folder_id }) => folder_id === dragged.id);
      if (folder) report(upsertFolder.mutateAsync(folderRequest(folder, folderId)));
      return;
    }
    const resource = resources.find(({ resource_id }) => resource_id === dragged.id);
    if (resource) report(updateResource.mutateAsync(resourceRequest(resource, folderId)));
  }

  const startDrag: StartDrag = (event, resourceId) => {
    event.dataTransfer.setData(DRAG_TYPE, JSON.stringify({ kind: "resource", id: resourceId }));
    event.dataTransfer.effectAllowed = "move";
  };
  const folderItems = visibleFolders.map((folder) => (
    <LibraryFolderItem
      key={folder.folder_id}
      folder={folder}
      viewMode={viewMode}
      onOpen={() => setCurrentFolderId(folder.folder_id)}
      onEdit={() => setEditingFolder(folder)}
      onDelete={() => setDeletingFolder(folder)}
      onDragStart={(event) => {
        event.dataTransfer.setData(
          DRAG_TYPE,
          JSON.stringify({ kind: "folder", id: folder.folder_id }),
        );
        event.dataTransfer.effectAllowed = "move";
      }}
      onDrop={(event) => dropInto(event, folder.folder_id)}
    />
  ));

  if (snapshotQuery.isPending) return <EmptyState title={t("loadingFolders")} />;
  if (snapshotQuery.isError) {
    return (
      <EmptyState title={t("foldersUnavailable")} description={formatError(snapshotQuery.error)} />
    );
  }

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div
        className="flex min-h-11 items-center justify-between gap-3 border-b border-app-border bg-app-surface/45 px-3 py-1.5"
        onDragOver={(event) => event.preventDefault()}
        onDrop={(event) => dropInto(event, currentFolderId)}
      >
        <nav className="flex min-w-0 items-center gap-1" aria-label={t("folderNavigation")}>
          <AppButton
            variant="ghost"
            className="h-8 px-2"
            onClick={() => setCurrentFolderId(null)}
            onDragOver={(event) => event.preventDefault()}
            onDrop={(event) => {
              event.stopPropagation();
              dropInto(event, null);
            }}
          >
            <Home aria-hidden="true" className="size-4" />
            {t("library")}
          </AppButton>
          {breadcrumbs.map((folder) => (
            <Fragment key={folder.folder_id}>
              <ChevronRight aria-hidden="true" className="size-3 text-app-muted" />
              <AppButton
                variant="ghost"
                className="h-8 max-w-48 px-2"
                onClick={() => setCurrentFolderId(folder.folder_id)}
                onDragOver={(event) => event.preventDefault()}
                onDrop={(event) => {
                  event.stopPropagation();
                  dropInto(event, folder.folder_id);
                }}
              >
                <span className="truncate">{folder.display_name}</span>
              </AppButton>
            </Fragment>
          ))}
        </nav>
        <AppButton variant="secondary" className="h-8" onClick={() => setEditingFolder(null)}>
          <FolderPlus aria-hidden="true" className="size-4" />
          {t("newFolder")}
        </AppButton>
      </div>
      {error ? (
        <p className="border-b border-rose-500/30 bg-rose-500/10 px-3 py-2 text-xs text-rose-200">
          {error}
        </p>
      ) : null}
      {children(
        visibleResourceIds,
        startDrag,
        { id: currentFolderId, path: currentPath },
        resources,
        folderItems,
      )}
      <FolderDialog
        key={editingFolder?.folder_id ?? (editingFolder === null ? "new" : "closed")}
        folder={editingFolder}
        namespace={namespace}
        parentId={currentFolderId}
        saving={upsertFolder.isPending}
        onClose={() => setEditingFolder(undefined)}
        onSave={(request) =>
          report(upsertFolder.mutateAsync(request).then(() => setEditingFolder(undefined)))
        }
      />
      <DeleteFolderDialog
        folder={deletingFolder}
        deleting={deleteFolder.isPending}
        onClose={() => setDeletingFolder(null)}
        onDelete={(folderId) =>
          report(
            deleteFolder.mutateAsync({ folder_id: folderId }).then(() => setDeletingFolder(null)),
          )
        }
      />
    </div>
  );
}

type StartDrag = (event: DragEvent, resourceId: string) => void;
type CurrentFolder = { id: string | null; path: string };

function LibraryFolderItem({
  folder,
  viewMode,
  onOpen,
  onEdit,
  onDelete,
  onDragStart,
  onDrop,
}: {
  folder: LibraryFolderDto;
  viewMode: ResourceViewMode;
  onOpen: () => void;
  onEdit: () => void;
  onDelete: () => void;
  onDragStart: (event: DragEvent<HTMLElement>) => void;
  onDrop: (event: DragEvent<HTMLElement>) => void;
}) {
  const { t } = useTranslation("resources");
  const actions = (
    <span className="flex shrink-0 gap-0.5">
      <AppIconButton
        icon={FolderPen}
        label={t("editNamedFolder", { name: folder.display_name })}
        size="sm"
        onClick={onEdit}
      />
      <AppIconButton
        icon={Trash2}
        label={t("deleteNamedFolder", { name: folder.display_name })}
        size="sm"
        variant="danger"
        onClick={onDelete}
      />
    </span>
  );
  const dragProps = {
    draggable: true,
    onDragStart,
    onDragOver: (event: DragEvent<HTMLElement>) => event.preventDefault(),
    onDrop: (event: DragEvent<HTMLElement>) => {
      event.stopPropagation();
      onDrop(event);
    },
  };
  if (viewMode === "list") {
    return (
      <article
        {...dragProps}
        className="grid grid-cols-[44px_minmax(0,1fr)_auto] items-center gap-3 border border-app-border bg-app-surface px-2 py-1.5 hover:border-brand-400/60"
      >
        <button
          type="button"
          aria-label={t("openNamedFolder", { name: folder.display_name })}
          className="grid size-11 place-items-center border border-app-border bg-black/20 text-brand-300"
          onClick={onOpen}
        >
          <Folder aria-hidden="true" className="size-5" />
        </button>
        <button
          type="button"
          aria-label={folder.display_name}
          className="min-w-0 text-left"
          onClick={onOpen}
        >
          <span className="block truncate text-sm font-semibold text-app-text">
            {folder.display_name}
          </span>
          <span className="mt-0.5 block truncate text-xs text-app-muted">{folder.path}</span>
        </button>
        {actions}
      </article>
    );
  }
  return (
    <article
      {...dragProps}
      className="group grid content-start border border-app-border bg-app-surface hover:border-brand-400/60"
    >
      <button
        type="button"
        aria-label={t("openNamedFolder", { name: folder.display_name })}
        className="grid aspect-square w-full place-items-center bg-black/20 text-brand-300"
        onClick={onOpen}
      >
        <Folder aria-hidden="true" className="size-12" />
      </button>
      <span className="flex min-w-0 items-center gap-1 border-t border-app-border px-2 py-1.5">
        <button
          type="button"
          aria-label={folder.display_name}
          className="min-w-0 flex-1 text-left"
          onClick={onOpen}
        >
          <span className="block truncate text-sm font-semibold text-app-text">
            {folder.display_name}
          </span>
          <span className="mt-1 block truncate text-xs text-app-muted">{folder.path}</span>
        </button>
        {actions}
      </span>
    </article>
  );
}

function FolderDialog({
  folder,
  namespace,
  parentId,
  saving,
  onClose,
  onSave,
}: {
  folder: LibraryFolderDto | null | undefined;
  namespace: LibraryNamespaceDto;
  parentId: string | null;
  saving: boolean;
  onClose: () => void;
  onSave: (request: UpsertLibraryFolderRequestDto) => void;
}) {
  const { t } = useTranslation("resources");
  const [identifier, setIdentifier] = useState(folder?.identifier ?? "");
  const [displayName, setDisplayName] = useState(folder?.display_name ?? "");
  if (folder === undefined) return null;
  return (
    <AppModal
      open
      title={folder ? t("editFolder") : t("newFolder")}
      density="compact"
      onClose={onClose}
    >
      <div className="grid gap-3">
        <TextInput label={t("identifier")} value={identifier} onChange={setIdentifier} />
        <TextInput label={t("displayNameOptional")} value={displayName} onChange={setDisplayName} />
        <div className="flex justify-end gap-2">
          <AppButton variant="ghost" onClick={onClose}>
            {t("cancel")}
          </AppButton>
          <AppButton
            disabled={saving || !identifier.trim()}
            onClick={() =>
              onSave({
                folder_id: folder?.folder_id ?? null,
                namespace,
                parent_id: folder ? folder.parent_id : parentId,
                identifier: identifier.trim(),
                display_name: displayName.trim(),
              })
            }
          >
            {t("save")}
          </AppButton>
        </div>
      </div>
    </AppModal>
  );
}

function DeleteFolderDialog({
  folder,
  deleting,
  onClose,
  onDelete,
}: {
  folder: LibraryFolderDto | null;
  deleting: boolean;
  onClose: () => void;
  onDelete: (folderId: string) => void;
}) {
  const { t } = useTranslation("resources");
  if (!folder) return null;
  return (
    <AppModal open title={t("deleteFolderTitle")} density="compact" onClose={onClose}>
      <div className="grid gap-4">
        <p className="text-sm text-app-muted">
          {t("deleteFolderDescription", { name: folder.display_name })}
        </p>
        <div className="flex justify-end gap-2">
          <AppButton variant="ghost" onClick={onClose}>
            {t("cancel")}
          </AppButton>
          <AppButton
            variant="danger"
            disabled={deleting}
            onClick={() => onDelete(folder.folder_id)}
          >
            {t("deleteFolder")}
          </AppButton>
        </div>
      </div>
    </AppModal>
  );
}

function breadcrumbFolders(
  folder: LibraryFolderDto | null,
  folders: ReadonlyArray<LibraryFolderDto>,
) {
  const result: LibraryFolderDto[] = [];
  let current = folder;
  while (current) {
    result.unshift(current);
    current = folders.find(({ folder_id }) => folder_id === current?.parent_id) ?? null;
  }
  return result;
}

function isInSubtree(path: string, folderPath: string) {
  return !folderPath || path === folderPath || path.startsWith(`${folderPath}/`);
}
function folderRequest(folder: LibraryFolderDto, parentId: string | null) {
  return {
    folder_id: folder.folder_id,
    namespace: folder.namespace,
    parent_id: parentId,
    identifier: folder.identifier,
    display_name: folder.display_name,
  };
}
function resourceRequest(resource: LibraryResourceDto, folderId: string | null) {
  return {
    resource_id: resource.resource_id,
    folder_id: folderId,
    identifier: resource.identifier,
    display_name: resource.display_name,
    aliases: resource.aliases,
  };
}
function readDraggedNode(event: DragEvent): DraggedNode | null {
  try {
    const value: unknown = JSON.parse(event.dataTransfer.getData(DRAG_TYPE));
    if (
      typeof value !== "object" ||
      value === null ||
      !("kind" in value) ||
      !("id" in value) ||
      (value.kind !== "folder" && value.kind !== "resource") ||
      typeof value.id !== "string"
    ) {
      return null;
    }
    return { kind: value.kind, id: value.id };
  } catch {
    return null;
  }
}
