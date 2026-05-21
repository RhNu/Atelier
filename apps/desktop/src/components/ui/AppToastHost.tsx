import { X } from "lucide-react";

import { useToastStore, type ToastLevel } from "../../stores/toast-store";

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
    <div className="pointer-events-none fixed right-4 top-12 z-[80] flex w-[min(420px,calc(100vw-2rem))] flex-col gap-2">
      {toasts.map((toast) => (
        <article
          key={toast.id}
          className={[
            "pointer-events-auto border p-3 shadow-app-panel backdrop-blur",
            levelClasses[toast.level],
          ].join(" ")}
        >
          <header className="flex items-start justify-between gap-3">
            <p className="text-sm font-semibold">{toast.title ?? toast.level}</p>
            <button
              type="button"
              aria-label="Close toast"
              className="text-current/70 transition-colors hover:text-current"
              onClick={() => remove(toast.id)}
            >
              <X aria-hidden="true" className="size-4" />
            </button>
          </header>
          <p className="mt-1 text-sm text-current/90">{toast.message}</p>
        </article>
      ))}
    </div>
  );
}
