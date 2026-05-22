import { Grid3X3, ListFilter } from "lucide-react";

import {
  AppButton,
  AppPanel,
  AppToolbar,
  EmptyState,
  ResourceImage,
  SafetyBadge,
} from "../../components/ui";
import { useGalleryPageQuery } from "./data/useGalleryPageQuery";

function formatError(error: unknown): string {
  return error instanceof Error ? error.message : "Command failed";
}

export function GalleryPage() {
  const galleryQuery = useGalleryPageQuery();
  const items = galleryQuery.data?.items ?? [];

  return (
    <div className="flex h-full min-h-0 flex-col">
      <AppToolbar>
        <div>
          <p className="text-xs font-semibold text-brand-200 uppercase">Gallery</p>
          <h1 className="text-lg font-semibold text-white">Workspace Gallery</h1>
        </div>
        <div className="flex items-center gap-2">
          <AppButton variant="ghost">
            <ListFilter aria-hidden="true" className="size-4" />
            Filter
          </AppButton>
          <AppButton variant="secondary">
            <Grid3X3 aria-hidden="true" className="size-4" />
            Grid
          </AppButton>
        </div>
      </AppToolbar>

      <AppPanel className="m-3 min-h-0 flex-1 overflow-hidden">
        <div className="h-full overflow-auto p-3">
          {galleryQuery.isPending ? (
            <p className="text-sm text-app-muted">Loading gallery</p>
          ) : galleryQuery.isError ? (
            <EmptyState title="Gallery unavailable" description={formatError(galleryQuery.error)} />
          ) : items.length === 0 ? (
            <EmptyState title="No gallery items" />
          ) : (
            <div className="grid grid-cols-[repeat(auto-fill,minmax(220px,1fr))] gap-3">
              {items.map((item) => (
                <article key={item.item_id} className="border border-app-border bg-app-surface">
                  <ResourceImage
                    src={null}
                    fallbackLabel={item.artifact_kind}
                    className="aspect-square w-full"
                  />
                  <div className="grid gap-2 p-3">
                    <div className="flex items-start justify-between gap-2">
                      <p className="truncate text-sm font-semibold text-app-text">
                        {item.model_name ?? item.artifact_id}
                      </p>
                      <SafetyBadge label={item.safety?.effective_label ?? "unknown"} />
                    </div>
                    <p className="text-xs text-app-muted">
                      {item.source_kind} / seed {item.seed ?? "auto"}
                    </p>
                  </div>
                </article>
              ))}
            </div>
          )}
        </div>
      </AppPanel>
    </div>
  );
}
