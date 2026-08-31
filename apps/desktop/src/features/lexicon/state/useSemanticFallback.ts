import { useEffect } from "react";
import { useTranslation } from "react-i18next";

import { useToastStore } from "@/stores/toast-store";
import type { LexiconBootstrapDto, LexiconSearchModeDto } from "@/types";

import { SemanticSearchTimeoutError } from "../data/useLexiconQueries";

export function useSemanticFallback(
  search: {
    mode: LexiconSearchModeDto;
    changeMode: (mode: LexiconSearchModeDto) => void;
    results: { error: Error | null; fetchStatus: "fetching" | "paused" | "idle" };
  },
  bootstrap: { data: LexiconBootstrapDto | undefined; refetch: () => Promise<unknown> },
) {
  const { t } = useTranslation("lexicon");
  const pushToast = useToastStore((state) => state.push);
  const { mode, changeMode } = search;
  // A retry can retain the previous query error while the new request is running.
  const error = search.results.fetchStatus === "fetching" ? null : search.results.error;
  const available = bootstrap.data?.status.semantic_available;
  const message = bootstrap.data?.status.message;
  const refresh = bootstrap.refetch;

  useEffect(() => {
    if (mode !== "semantic" || (!error && available !== false)) return;
    changeMode("lexical");
    if (error) void refresh();
    const reason = error
      ? error instanceof SemanticSearchTimeoutError
        ? t("semanticTimeout")
        : error.message
      : message;
    pushToast({
      level: "warning",
      message: `${t("semanticFallback")}${reason ? ` ${reason}` : ""}`,
    });
  }, [available, changeMode, error, message, mode, pushToast, refresh, t]);
}
