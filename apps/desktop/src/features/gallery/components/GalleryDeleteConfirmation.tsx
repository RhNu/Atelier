import { AppButton, AppModal } from "../../../components/ui";
import type { GalleryItemDto } from "../../../types";

export function GalleryDeleteConfirmation({
  targetId,
  target,
  deleting,
  error,
  onClose,
  onConfirm,
}: {
  targetId: string | null;
  target: GalleryItemDto | null;
  deleting: boolean;
  error: string | null;
  onClose: () => void;
  onConfirm: () => void;
}) {
  return (
    <AppModal open={Boolean(targetId)} title="Delete gallery item" onClose={onClose}>
      <div className="grid gap-4">
        <div className="grid gap-2 text-sm text-app-text">
          <p>
            Delete <span className="font-semibold text-white">{targetId}</span> permanently from
            Gallery.
          </p>
          <p className="text-app-muted">
            This also removes linked output metadata and may delete unreferenced workspace image
            files from disk.
          </p>
          {target ? <p className="text-xs text-app-muted">Artifact: {target.artifact_id}</p> : null}
        </div>
        {error ? (
          <p className="border border-rose-500/50 bg-rose-950/40 px-3 py-2 text-sm text-rose-100">
            {error}
          </p>
        ) : null}
        <div className="flex justify-end gap-2">
          <AppButton variant="secondary" onClick={onClose} disabled={deleting}>
            Cancel
          </AppButton>
          <AppButton variant="danger" onClick={onConfirm} disabled={deleting}>
            Delete permanently
          </AppButton>
        </div>
      </div>
    </AppModal>
  );
}
