import { Plus, Save, Trash2 } from "lucide-react";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";

import { AppButton } from "@/components/ui";
import type { CompiledPromptDto } from "@/types";

export function EditorPanel({
  error,
  actions,
  children,
}: {
  error: string | null;
  actions: ReactNode;
  children: ReactNode;
}) {
  return (
    <div className="grid gap-3">
      {error ? (
        <p className="border border-rose-500/50 bg-rose-950/40 px-3 py-2 text-sm text-rose-100">
          {error}
        </p>
      ) : null}
      <div className="grid gap-3">{children}</div>
      <footer className="flex items-center justify-end border-t border-app-border pt-3">
        {actions}
      </footer>
    </div>
  );
}

export function EditorActions({
  canDelete,
  saving,
  deleting,
  onSave,
  onDelete,
}: {
  canDelete: boolean;
  saving: boolean;
  deleting: boolean;
  onSave: () => void;
  onDelete: () => void;
}) {
  const { t } = useTranslation("common");
  return (
    <div className="flex w-full items-center justify-between gap-2">
      <span>
        {canDelete ? (
          <AppButton variant="danger" onClick={onDelete} disabled={deleting || saving}>
            <Trash2 aria-hidden="true" className="size-4" />
            {t("delete")}
          </AppButton>
        ) : null}
      </span>
      <AppButton onClick={onSave} disabled={saving || deleting}>
        {canDelete ? (
          <Save aria-hidden="true" className="size-4" />
        ) : (
          <Plus aria-hidden="true" className="size-4" />
        )}
        {saving ? t("saving") : t(canDelete ? "save" : "create")}
      </AppButton>
    </div>
  );
}

export function CompiledPreview({ preview }: { preview: CompiledPromptDto | null }) {
  const { t } = useTranslation("resources");
  if (!preview) {
    return null;
  }
  return (
    <article className="border border-app-border bg-black/20 p-3">
      <p className="text-xs font-semibold text-app-muted uppercase">{t("compiledPreview")}</p>
      <p className="mt-2 text-sm leading-6 whitespace-pre-wrap text-app-text">
        {preview.expanded_prompt || t("empty")}
      </p>
      <p className="mt-2 text-xs text-app-muted">
        {t("functionCalls", { count: preview.trace.function_calls.length })}
      </p>
    </article>
  );
}
