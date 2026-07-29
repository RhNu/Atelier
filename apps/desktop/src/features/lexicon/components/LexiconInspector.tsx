import { Check, Loader2, Plus, X } from "lucide-react";
import { type ReactNode, useCallback } from "react";
import { useTranslation } from "react-i18next";

import { AppButton, AppIconButton, EmptyState } from "@/components/ui";
import type { LexiconEntityDetailDto, LexiconSearchItemDto } from "@/types";

type Props = {
  selected: boolean;
  detail: LexiconEntityDetailDto | undefined;
  pending: boolean;
  error: string | null;
  inBasket: boolean;
  onClose: () => void;
  onToggleBasket: (item: LexiconSearchItemDto) => void;
  onInspectRelated: (entityId: number) => void;
};

export function LexiconInspector({
  selected,
  detail,
  pending,
  error,
  inBasket,
  onClose,
  onToggleBasket,
  onInspectRelated,
}: Props) {
  const { t } = useTranslation("lexicon");
  const toggleEntity = useCallback(() => {
    if (detail) onToggleBasket(detail.entity);
  }, [detail, onToggleBasket]);
  return (
    <aside className="flex h-full min-h-0 flex-col border-l border-app-border bg-app-surface">
      <header className="flex h-12 shrink-0 items-center justify-between border-b border-app-border px-3">
        <h2 className="text-xs font-semibold tracking-wide text-app-text uppercase">
          {t("details")}
        </h2>
        {selected ? (
          <AppIconButton
            icon={X}
            label={t("closeDetails")}
            size="sm"
            className="[&>svg]:size-5"
            onClick={onClose}
          />
        ) : null}
      </header>
      <div className="min-h-0 flex-1 overflow-auto p-4">
        {!selected ? (
          <EmptyState title={t("selectForDetails")} />
        ) : pending ? (
          <EmptyState
            title={t("loadingDetails")}
            icon={Loader2}
            iconClassName="animate-spin text-brand-200"
          />
        ) : error ? (
          <EmptyState title={t("detailsFailed")} description={error} />
        ) : detail ? (
          <div className="grid gap-5">
            <section>
              <code className="text-base font-semibold text-brand-100">
                {detail.entity.canonical_name}
              </code>
              <p className="mt-1 text-sm text-app-text">{detail.entity.primary_translation}</p>
              <AppButton
                variant={inBasket ? "secondary" : "primary"}
                className="mt-3 w-full"
                onClick={toggleEntity}
              >
                {inBasket ? (
                  <Check aria-hidden="true" className="size-5" />
                ) : (
                  <Plus aria-hidden="true" className="size-5" />
                )}
                {inBasket ? t("removeFromBasket") : t("addToBasket")}
              </AppButton>
            </section>
            <DetailSection title={t("wiki")}>
              {detail.wiki.length > 0
                ? detail.wiki.map((wiki) => <p key={wiki.locale}>{wiki.text}</p>)
                : t("noWiki")}
            </DetailSection>
            <DetailSection title={t("aliases")}>
              {detail.aliases.length > 0 ? detail.aliases.join(", ") : "—"}
            </DetailSection>
            <DetailSection title={t("groups")}>
              {detail.groups.length > 0 ? detail.groups.map((group) => group.name).join(", ") : "—"}
            </DetailSection>
            <section>
              <h3 className="mb-2 text-[10px] tracking-wide text-app-muted uppercase">
                {t("related")}
              </h3>
              <div className="grid gap-1">
                {detail.related.length === 0 ? (
                  <p className="text-xs text-app-muted">—</p>
                ) : (
                  detail.related.map((related) => (
                    <RelatedEntityButton
                      key={related.entity.entity_id}
                      item={related.entity}
                      onInspect={onInspectRelated}
                    />
                  ))
                )}
              </div>
            </section>
          </div>
        ) : null}
      </div>
    </aside>
  );
}

function RelatedEntityButton({
  item,
  onInspect,
}: {
  item: LexiconSearchItemDto;
  onInspect: (entityId: number) => void;
}) {
  const handleClick = useCallback(() => onInspect(item.entity_id), [item.entity_id, onInspect]);
  return (
    <button
      type="button"
      className="border border-app-border px-2 py-1.5 text-left hover:border-brand-400/50"
      onClick={handleClick}
    >
      <code className="text-xs text-brand-100">{item.canonical_name}</code>
      <span className="ml-2 text-[10px] text-app-muted">{item.primary_translation}</span>
    </button>
  );
}

function DetailSection({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section>
      <h3 className="mb-2 text-[10px] tracking-wide text-app-muted uppercase">{title}</h3>
      <div className="text-xs leading-5 text-app-text">{children}</div>
    </section>
  );
}
