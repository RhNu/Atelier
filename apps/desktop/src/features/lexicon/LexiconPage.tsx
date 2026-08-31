import { useNavigate } from "@tanstack/react-router";
import { useCallback, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

import { AppButton, EmptyState } from "@/components/ui";
import { useToastStore } from "@/stores/toast-store";
import type { LexiconSearchItemDto } from "@/types";

import { useInstallDownloadableResourceGroupMutation } from "../settings/data/useDownloadableResources";
import { LexiconBasket } from "./components/LexiconBasket";
import { LexiconFilters } from "./components/LexiconFilters";
import { LexiconInspector } from "./components/LexiconInspector";
import { LexiconResults } from "./components/LexiconResults";
import {
  SemanticSearchTimeoutError,
  useAppendLexiconEntitiesMutation,
  useLexiconBootstrapQuery,
  useLexiconEntityQuery,
} from "./data/useLexiconQueries";
import { useLexiconSearchState } from "./state/useLexiconSearchState";
import { useSemanticFallback } from "./state/useSemanticFallback";

export function LexiconPage() {
  const { t } = useTranslation("lexicon");
  const navigate = useNavigate();
  const pushToast = useToastStore((state) => state.push);
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [basket, setBasket] = useState<Map<number, LexiconSearchItemDto>>(() => new Map());
  const search = useLexiconSearchState(basket);
  const bootstrap = useLexiconBootstrapQuery();
  const detail = useLexiconEntityQuery(selectedId);
  const append = useAppendLexiconEntitiesMutation();
  const installResources = useInstallDownloadableResourceGroupMutation();
  const basketIds = useMemo(() => new Set(basket.keys()), [basket]);
  const basketItems = useMemo(() => [...basket.values()], [basket]);
  useSemanticFallback(search, bootstrap);

  const toggleBasket = useCallback((item: LexiconSearchItemDto) => {
    setBasket((current) => {
      const next = new Map(current);
      if (next.has(item.entity_id)) next.delete(item.entity_id);
      else next.set(item.entity_id, item);
      return next;
    });
  }, []);
  const closeInspector = useCallback(() => setSelectedId(null), []);
  const removeBasketItem = useCallback((entityId: number) => {
    setBasket((current) => {
      const next = new Map(current);
      next.delete(entityId);
      return next;
    });
  }, []);
  const clearBasket = useCallback(() => setBasket(new Map()), []);
  const submit = useCallback(
    (target: "positive" | "negative") => {
      void append
        .mutateAsync({ target, entity_ids: [...basket.keys()] })
        .then(async () => {
          setBasket(new Map());
          pushToast({ level: "success", message: t("addedToDraft") });
          await navigate({ to: "/generate" });
        })
        .catch((error: unknown) => {
          pushToast({ level: "error", message: formatError(error) });
        });
    },
    [append, basket, navigate, pushToast, t],
  );
  const installLexicon = useCallback(() => {
    installResources.mutate(
      { request: { group_id: "semantic-search" }, onProgress: () => undefined },
      { onSuccess: () => void bootstrap.refetch() },
    );
  }, [bootstrap, installResources]);
  const installAction = useMemo(
    () => (
      <AppButton disabled={installResources.isPending} onClick={installLexicon}>
        {installResources.isPending ? t("installingLexicon") : t("installLexicon")}
      </AppButton>
    ),
    [installLexicon, installResources.isPending, t],
  );

  if (bootstrap.isPending) {
    return <p className="p-4 text-sm text-app-muted">{t("loadingCatalog")}</p>;
  }
  if (bootstrap.isError || !bootstrap.data?.status.lexical_available) {
    return (
      <EmptyState
        title={t("unavailable")}
        description={
          bootstrap.data?.status.message ??
          (bootstrap.error ? formatError(bootstrap.error) : undefined)
        }
        action={installAction}
      />
    );
  }

  return (
    <div className="flex h-full min-h-0 flex-col">
      <LexiconFilters
        bootstrap={bootstrap.data}
        mode={search.mode}
        query={search.query}
        kind={search.kind}
        category={search.category}
        groupId={search.groupId}
        rating={search.rating}
        pending={search.semanticBusy}
        hasActiveSearch={search.hasActiveSearch}
        onModeChange={search.changeMode}
        onQueryChange={search.changeQuery}
        onQueryClear={search.clearQuery}
        onKindChange={search.changeKind}
        onCategoryChange={search.changeCategory}
        onGroupChange={search.changeGroup}
        onRatingChange={search.changeRating}
        onReset={search.reset}
        onSemanticSubmit={search.submitSemantic}
      />
      <div className="grid min-h-0 flex-1 grid-cols-[minmax(0,1fr)_clamp(320px,24vw,420px)]">
        <LexiconResults
          page={search.results.data}
          pending={search.results.isPending && search.results.fetchStatus === "fetching"}
          fetching={search.results.isFetching}
          idle={search.mode === "semantic" && search.semanticQuery.length === 0}
          error={
            search.results.error
              ? search.results.error instanceof SemanticSearchTimeoutError
                ? t("semanticTimeout")
                : formatError(search.results.error)
              : null
          }
          selectedId={selectedId}
          basketIds={basketIds}
          onInspect={setSelectedId}
          onToggleBasket={toggleBasket}
          onPageChange={search.changeOffset}
        />
        <section aria-label={t("details")} className="min-h-0 overflow-hidden outline-none">
          <LexiconInspector
            selected={selectedId !== null}
            detail={detail.data}
            pending={detail.fetchStatus === "fetching"}
            error={detail.error ? formatError(detail.error) : null}
            inBasket={detail.data ? basket.has(detail.data.entity.entity_id) : false}
            onClose={closeInspector}
            onToggleBasket={toggleBasket}
            onInspectRelated={setSelectedId}
          />
        </section>
      </div>
      <LexiconBasket
        items={basketItems}
        pending={append.isPending}
        onRemove={removeBasketItem}
        onClear={clearBasket}
        onSubmit={submit}
      />
    </div>
  );
}

function formatError(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
