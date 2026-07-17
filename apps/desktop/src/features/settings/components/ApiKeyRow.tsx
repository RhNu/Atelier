import { PencilLine, Power, Trash2 } from "lucide-react";
import { useCallback } from "react";
import { useTranslation } from "react-i18next";

import { AppButton } from "@/components/ui";
import type { ApiKeyRecordDto } from "@/types";

export type ApiKeyEditState = {
  id: string;
  displayName: string;
  secret: string;
};

export function ApiKeyRow({
  item,
  busy,
  onEdit,
  onSetActive,
  onDelete,
}: {
  item: ApiKeyRecordDto;
  busy: boolean;
  onEdit: (editing: ApiKeyEditState) => void;
  onSetActive: (id: string) => void;
  onDelete: (id: string) => void;
}) {
  const startEdit = useCallback(() => {
    onEdit({ id: item.id, displayName: item.display_name, secret: "" });
  }, [item.display_name, item.id, onEdit]);
  const setActive = useCallback(() => {
    onSetActive(item.id);
  }, [item.id, onSetActive]);
  const deleteKey = useCallback(() => {
    onDelete(item.id);
  }, [item.id, onDelete]);

  return (
    <article className="border-b border-app-border p-3 last:border-b-0">
      <div className="flex flex-col gap-3 lg:flex-row lg:items-center lg:justify-between">
        <ApiKeyIdentity item={item} />
        <ApiKeyActions
          item={item}
          busy={busy}
          onSetActive={setActive}
          onEdit={startEdit}
          onDelete={deleteKey}
        />
      </div>
    </article>
  );
}

function ApiKeyIdentity({ item }: { item: ApiKeyRecordDto }) {
  const { t } = useTranslation("settings");
  return (
    <div className="min-w-0">
      <div className="flex flex-wrap items-center gap-2">
        <p className="truncate text-sm font-semibold text-app-text">{item.display_name}</p>
        {item.is_active ? (
          <span className="border border-emerald-500/45 bg-emerald-500/10 px-2 py-0.5 text-[10px] font-semibold text-emerald-200 uppercase">
            {t("active")}
          </span>
        ) : null}
      </div>
      <p className="mt-1 text-xs break-all text-app-muted">ID {item.id}</p>
    </div>
  );
}

function ApiKeyActions({
  item,
  busy,
  onSetActive,
  onEdit,
  onDelete,
}: {
  item: ApiKeyRecordDto;
  busy: boolean;
  onSetActive: () => void;
  onEdit: () => void;
  onDelete: () => void;
}) {
  const { t } = useTranslation("settings");
  const { t: translateCommon } = useTranslation("common");
  return (
    <div className="flex flex-wrap gap-1.5">
      {!item.is_active ? (
        <AppButton
          variant="ghost"
          className="h-8 px-2 text-xs"
          disabled={busy}
          aria-label={t("setActive", { name: item.display_name })}
          onClick={onSetActive}
        >
          <Power aria-hidden="true" className="size-3.5" />
          {t("activate")}
        </AppButton>
      ) : null}
      <AppButton
        variant="ghost"
        className="h-8 px-2 text-xs"
        disabled={busy}
        aria-label={t("editKey", { name: item.display_name })}
        onClick={onEdit}
      >
        <PencilLine aria-hidden="true" className="size-3.5" />
        {translateCommon("edit")}
      </AppButton>
      <AppButton
        variant="danger"
        className="h-8 px-2 text-xs"
        disabled={busy}
        aria-label={t("deleteKey", { name: item.display_name })}
        onClick={onDelete}
      >
        <Trash2 aria-hidden="true" className="size-3.5" />
        {translateCommon("delete")}
      </AppButton>
    </div>
  );
}
