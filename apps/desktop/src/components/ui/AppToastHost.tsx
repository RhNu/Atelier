import { X } from "lucide-react";
import { useCallback, useEffect } from "react";
import { useTranslation } from "react-i18next";

import { useToastStore, type ToastItem as Toast, type ToastLevel } from "@/stores/toast-store";

const levelClasses: Record<ToastLevel, string> = {
  info: "border-app-border bg-app-panel text-app-text",
  success: "border-emerald-500/50 bg-emerald-500/10 text-emerald-100",
  warning: "border-amber-500/50 bg-amber-500/10 text-amber-100",
  error: "border-rose-500/60 bg-rose-500/10 text-rose-100",
};

export function AppToastHost() {
  const toasts = useToastStore((state) => state.toasts);
  const remove = useToastStore((state) => state.remove);

  return (
    <div className="pointer-events-none fixed top-12 right-4 z-[80] flex w-[min(420px,calc(100vw-2rem))] flex-col gap-2">
      {toasts.map((toast) => (
        <ToastItem key={toast.id} toast={toast} onRemove={remove} />
      ))}
    </div>
  );
}

function ToastItem({ toast, onRemove }: { toast: Toast; onRemove: (id: string) => void }) {
  const { t } = useTranslation("shell");
  const handleRemove = useCallback(() => {
    onRemove(toast.id);
  }, [onRemove, toast.id]);
  const handleAction = useCallback(() => {
    toast.action?.onClick();
    onRemove(toast.id);
  }, [onRemove, toast]);

  useEffect(() => {
    if (toast.durationMs === null) return undefined;
    const timeout = window.setTimeout(handleRemove, toast.durationMs);
    return () => window.clearTimeout(timeout);
  }, [handleRemove, toast.durationMs]);

  return (
    <article
      role={toast.level === "error" ? "alert" : "status"}
      aria-live={toast.level === "error" ? "assertive" : "polite"}
      className={[
        "pointer-events-auto border p-3 shadow-app-panel backdrop-blur",
        levelClasses[toast.level],
      ].join(" ")}
    >
      <header className="flex items-start justify-between gap-3">
        <p className="text-sm font-semibold">{toast.title ?? t(`toastLevel.${toast.level}`)}</p>
        <button
          type="button"
          aria-label={t("closeToast")}
          className="text-current/70 transition-colors hover:text-current"
          onClick={handleRemove}
        >
          <X aria-hidden="true" className="size-4" />
        </button>
      </header>
      <p className="mt-1 text-sm text-current/90">{toast.message}</p>
      {toast.action ? (
        <button
          type="button"
          className="mt-2 border border-current/35 px-2 py-1 text-xs font-semibold hover:bg-white/10"
          onClick={handleAction}
        >
          {toast.action.label}
        </button>
      ) : null}
    </article>
  );
}
