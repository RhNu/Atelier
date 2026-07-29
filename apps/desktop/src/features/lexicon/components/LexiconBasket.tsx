import { ArrowRight, Trash2, X } from "lucide-react";
import { useCallback } from "react";
import { useTranslation } from "react-i18next";

import { AppButton } from "@/components/ui";
import type { LexiconSearchItemDto } from "@/types";

type Props = {
  items: LexiconSearchItemDto[];
  pending: boolean;
  onRemove: (entityId: number) => void;
  onClear: () => void;
  onSubmit: (target: "positive" | "negative") => void;
};

export function LexiconBasket({ items, pending, onRemove, onClear, onSubmit }: Props) {
  const { t } = useTranslation("lexicon");
  const submitNegative = useCallback(() => onSubmit("negative"), [onSubmit]);
  const submitPositive = useCallback(() => onSubmit("positive"), [onSubmit]);
  return (
    <footer className="shrink-0 border-t border-app-border bg-app-surface px-3 py-2">
      <div className="flex min-h-10 items-center gap-2">
        <span className="shrink-0 text-xs font-semibold text-app-text">
          {t("basket", { count: items.length })}
        </span>
        <div className="flex min-w-0 flex-1 gap-1 overflow-x-auto">
          {items.map((item) => (
            <BasketItem key={item.entity_id} item={item} onRemove={onRemove} />
          ))}
          {items.length === 0 ? (
            <span className="self-center text-xs text-app-muted">{t("basketEmpty")}</span>
          ) : null}
        </div>
        <AppButton
          variant="ghost"
          className="size-8 shrink-0 p-0"
          disabled={items.length === 0 || pending}
          aria-label={t("clearBasket")}
          onClick={onClear}
        >
          <Trash2 aria-hidden="true" className="size-4" />
        </AppButton>
        <AppButton
          variant="secondary"
          disabled={items.length === 0 || pending}
          onClick={submitNegative}
        >
          {t("addNegative")}
          <ArrowRight aria-hidden="true" className="size-4" />
        </AppButton>
        <AppButton
          variant="primary"
          disabled={items.length === 0 || pending}
          onClick={submitPositive}
        >
          {pending ? t("addingToDraft") : t("addPositive")}
          <ArrowRight aria-hidden="true" className="size-4" />
        </AppButton>
      </div>
    </footer>
  );
}

function BasketItem({
  item,
  onRemove,
}: {
  item: LexiconSearchItemDto;
  onRemove: (entityId: number) => void;
}) {
  const { t } = useTranslation("lexicon");
  const handleRemove = useCallback(() => onRemove(item.entity_id), [item.entity_id, onRemove]);
  return (
    <span className="flex shrink-0 items-center gap-1 border border-app-border bg-black/20 px-2 py-1 text-xs text-brand-100">
      {item.canonical_name}
      <button
        type="button"
        aria-label={t("removeTag", { tag: item.canonical_name })}
        onClick={handleRemove}
      >
        <X aria-hidden="true" className="size-3" />
      </button>
    </span>
  );
}
