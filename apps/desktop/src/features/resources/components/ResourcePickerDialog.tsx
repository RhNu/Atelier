/* eslint-disable react-perf/jsx-no-new-function-as-prop */
import { Check, Folder, Home, LoaderCircle, Search } from "lucide-react";
import { Fragment, useDeferredValue, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

import { AppButton, AppModal, ResourceImage } from "@/components/ui";
import { resourceImageToDataUrl } from "@/platform/atelier";
import type { LibraryNamespaceDto, ResourceRefDto } from "@/types";

import { useResourceImageQuery, useResourceLibraryQuery } from "../data/useResourcesData";
import {
  buildResourcePickerView,
  folderName,
  type ResourcePickerItem,
} from "../resource-picker-model";

export function ResourcePickerDialog<Item extends ResourcePickerItem>({
  open,
  title,
  namespace,
  items,
  selectedItemId,
  pending = false,
  error,
  emptyLabel,
  noMatchesLabel,
  onClose,
  onSelect,
}: {
  open: boolean;
  title: string;
  namespace: LibraryNamespaceDto;
  items: ReadonlyArray<Item>;
  selectedItemId?: string | null;
  pending?: boolean;
  error?: unknown;
  emptyLabel: string;
  noMatchesLabel: string;
  onClose: () => void;
  onSelect: (item: Item) => void;
}) {
  const { t } = useTranslation("resources");
  const [currentFolderId, setCurrentFolderId] = useState<string | null>(null);
  const [search, setSearch] = useState("");
  const deferredSearch = useDeferredValue(search);
  const libraryQuery = useResourceLibraryQuery(namespace, open);
  const view = useMemo(
    () =>
      buildResourcePickerView(
        libraryQuery.data?.folders ?? [],
        libraryQuery.data?.resources ?? [],
        items,
        currentFolderId,
        deferredSearch,
      ),
    [currentFolderId, deferredSearch, items, libraryQuery.data],
  );

  function close() {
    setCurrentFolderId(null);
    setSearch("");
    onClose();
  }

  const loading = pending || libraryQuery.isPending;
  const failure = error ?? libraryQuery.error;
  const hasEntries = view.folders.length > 0 || view.items.length > 0;

  return (
    <AppModal open={open} title={title} onClose={close}>
      <div className="grid gap-3">
        <label className="flex min-w-0 items-center gap-2 border border-app-border bg-black/20 px-3 text-app-muted">
          <Search aria-hidden="true" className="size-4 shrink-0" />
          <input
            aria-label={t("search")}
            value={search}
            placeholder={t("search")}
            className="h-9 min-w-0 flex-1 bg-transparent text-sm text-app-text outline-none placeholder:text-app-muted"
            onChange={(event) => setSearch(event.target.value)}
          />
        </label>

        <PickerBreadcrumbs breadcrumbs={view.breadcrumbs} onOpen={setCurrentFolderId} />

        {loading ? (
          <div className="flex min-h-48 items-center justify-center gap-2 text-sm text-app-muted">
            <LoaderCircle aria-hidden="true" className="size-4 animate-spin" />
            {t("loading")}
          </div>
        ) : failure ? (
          <p className="border border-rose-500/40 bg-rose-950/30 p-3 text-sm text-rose-100">
            {failure instanceof Error ? failure.message : t("unavailable")}
          </p>
        ) : !hasEntries ? (
          <div className="grid min-h-48 place-items-center border border-dashed border-app-border text-sm text-app-muted">
            {deferredSearch.trim() ? noMatchesLabel : emptyLabel}
          </div>
        ) : (
          <div className="grid grid-cols-2 gap-2 sm:grid-cols-3 lg:grid-cols-4">
            {view.folders.map((folder) => (
              <button
                key={folder.folder_id}
                type="button"
                aria-label={t("openNamedFolder", { name: folderName(folder) })}
                className="grid min-w-0 content-start border border-app-border bg-black/20 text-left hover:border-brand-400/60 hover:bg-app-surface"
                onClick={() => setCurrentFolderId(folder.folder_id)}
              >
                <span className="grid aspect-square w-full place-items-center text-brand-300">
                  <Folder aria-hidden="true" className="size-12" />
                </span>
                <span className="min-w-0 border-t border-app-border px-2 py-2">
                  <span className="block truncate text-xs font-semibold text-app-text">
                    {folderName(folder)}
                  </span>
                  <span className="mt-0.5 block truncate text-[11px] text-app-muted">
                    {folder.path}
                  </span>
                </span>
              </button>
            ))}
            {view.items.map(({ item, resource }) => {
              const selected = item.id === selectedItemId;
              return (
                <button
                  key={item.id}
                  type="button"
                  aria-pressed={selected}
                  className={[
                    "group relative grid min-w-0 content-start border bg-black/20 text-left",
                    selected
                      ? "border-brand-400/70 bg-brand-500/10"
                      : "border-app-border hover:border-brand-400/60 hover:bg-app-surface",
                  ].join(" ")}
                  onClick={() => {
                    onSelect(item);
                    close();
                  }}
                >
                  {selected ? (
                    <span className="absolute top-2 right-2 z-10 grid size-5 place-items-center bg-brand-500 text-white">
                      <Check aria-hidden="true" className="size-3.5" />
                    </span>
                  ) : null}
                  <PickerThumbnail resource={item.preview ?? null} label={item.label} />
                  <span className="min-w-0 border-t border-app-border px-2 py-2">
                    <span className="block truncate text-xs font-semibold text-app-text">
                      {item.label || resource.identifier}
                    </span>
                    <span className="mt-0.5 block truncate text-[11px] text-app-muted">
                      {resource.path}
                    </span>
                    {item.description ? (
                      <span className="mt-1 line-clamp-2 block text-[11px] text-app-muted/80">
                        {item.description}
                      </span>
                    ) : null}
                  </span>
                </button>
              );
            })}
          </div>
        )}
      </div>
    </AppModal>
  );
}

function PickerBreadcrumbs({
  breadcrumbs,
  onOpen,
}: {
  breadcrumbs: ReturnType<typeof buildResourcePickerView>["breadcrumbs"];
  onOpen: (folderId: string | null) => void;
}) {
  const { t } = useTranslation("resources");
  return (
    <nav
      className="flex min-h-9 min-w-0 items-center gap-1 border-b border-app-border pb-2"
      aria-label={t("folderNavigation")}
    >
      <AppButton variant="ghost" className="h-8 px-2" onClick={() => onOpen(null)}>
        <Home aria-hidden="true" className="size-4" />
        {t("library")}
      </AppButton>
      {breadcrumbs.map((folder) => (
        <Fragment key={folder.folder_id}>
          <span aria-hidden="true" className="text-app-muted">
            /
          </span>
          <AppButton
            variant="ghost"
            className="h-8 max-w-48 px-2"
            onClick={() => onOpen(folder.folder_id)}
          >
            <span className="truncate">{folderName(folder)}</span>
          </AppButton>
        </Fragment>
      ))}
    </nav>
  );
}

function PickerThumbnail({ resource, label }: { resource: ResourceRefDto | null; label: string }) {
  const query = useResourceImageQuery(resource);
  return (
    <ResourceImage
      src={query.data ? resourceImageToDataUrl(query.data) : null}
      alt={label}
      fallbackLabel=""
      className="aspect-square w-full border-0 bg-black/20"
    />
  );
}
