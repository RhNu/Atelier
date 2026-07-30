import { ClipboardCopy, Trash2, X } from "lucide-react";
import { useCallback, type MouseEvent } from "react";
import { useTranslation } from "react-i18next";

import { AppButton } from "@/components/ui";
import type { DanbooruTagDto } from "@/types";

type Props = {
  tags: DanbooruTagDto[];
  copying: boolean;
  onRemove: (name: string) => void;
  onClear: () => void;
  onCopyPrompt: () => void;
  onCopyQuery: () => void;
};

export function DanbooruTagBasket({
  tags,
  copying,
  onRemove,
  onClear,
  onCopyPrompt,
  onCopyQuery,
}: Props) {
  const { t } = useTranslation("inspiration");
  const removeTag = useCallback(
    (event: MouseEvent<HTMLButtonElement>) => {
      const name = event.currentTarget.dataset.tag;
      if (name) onRemove(name);
    },
    [onRemove],
  );
  return (
    <footer className="flex min-h-14 items-center gap-2 border-t border-app-border bg-app-surface px-3 py-2">
      <span className="shrink-0 text-xs font-semibold">{t("basket", { count: tags.length })}</span>
      <div className="flex min-w-0 flex-1 gap-1 overflow-x-auto">
        {tags.length === 0 ? (
          <span className="self-center text-xs text-app-muted">{t("basketEmpty")}</span>
        ) : null}
        {tags.map((tag) => (
          <span
            key={tag.canonical_name}
            className="flex shrink-0 items-center gap-1 border border-app-border bg-black/20 px-2 py-1 text-xs text-brand-100"
          >
            {tag.canonical_name}
            <button
              type="button"
              data-tag={tag.canonical_name}
              aria-label={t("removeTag", { tag: tag.canonical_name })}
              onClick={removeTag}
            >
              <X aria-hidden="true" className="size-3" />
            </button>
          </span>
        ))}
      </div>
      <AppButton
        variant="ghost"
        className="size-8 shrink-0 p-0"
        disabled={tags.length === 0 || copying}
        aria-label={t("clearBasket")}
        onClick={onClear}
      >
        <Trash2 aria-hidden="true" className="size-4" />
      </AppButton>
      <AppButton variant="secondary" disabled={tags.length === 0 || copying} onClick={onCopyQuery}>
        <ClipboardCopy aria-hidden="true" className="size-4" />
        {t("copyQuery")}
      </AppButton>
      <AppButton disabled={tags.length === 0 || copying} onClick={onCopyPrompt}>
        <ClipboardCopy aria-hidden="true" className="size-4" />
        {t("copyPrompt")}
      </AppButton>
    </footer>
  );
}
