import { X } from "lucide-react";
import { useCallback, useEffect, useId, useRef, type KeyboardEvent, type ReactNode } from "react";
import { createPortal } from "react-dom";

import { AppIconButton } from "./AppIconButton";

const FOCUSABLE_SELECTOR = [
  "a[href]",
  "button:not([disabled])",
  "input:not([disabled])",
  "select:not([disabled])",
  "textarea:not([disabled])",
  '[tabindex]:not([tabindex="-1"])',
].join(",");

type AppModalProps = {
  open: boolean;
  title: string;
  size?: "default" | "fullscreen";
  children: ReactNode;
  onClose: () => void;
};

export function AppModal({ open, title, size = "default", children, onClose }: AppModalProps) {
  const dialogRef = useRef<HTMLDialogElement>(null);
  const titleId = useId();

  useEffect(() => {
    if (!open) {
      return;
    }
    const previouslyFocused =
      document.activeElement instanceof HTMLElement ? document.activeElement : null;
    dialogRef.current?.focus();
    return () => previouslyFocused?.focus();
  }, [open]);

  const handleKeyDown = useCallback(
    (event: KeyboardEvent<HTMLDialogElement>) => {
      if (event.key === "Escape") {
        event.preventDefault();
        onClose();
        return;
      }
      if (event.key !== "Tab") {
        return;
      }

      const dialog = event.currentTarget;
      const focusable = Array.from(dialog.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR));
      const first = focusable[0];
      const last = focusable.at(-1);
      if (!first || !last) {
        event.preventDefault();
        dialog.focus();
      } else if (
        event.shiftKey &&
        (document.activeElement === first || document.activeElement === dialog)
      ) {
        event.preventDefault();
        last.focus();
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault();
        first.focus();
      }
    },
    [onClose],
  );

  if (!open) {
    return null;
  }

  return createPortal(
    <div className="fixed inset-0 z-50 grid place-items-center bg-black/58 p-4 backdrop-blur-sm">
      <dialog
        ref={dialogRef}
        open
        tabIndex={-1}
        aria-modal="true"
        aria-labelledby={titleId}
        className={[
          "relative m-0 grid w-full grid-rows-[auto_minmax(0,1fr)] overflow-hidden border border-app-border bg-app-panel p-0 text-left text-app-text shadow-app-panel",
          size === "fullscreen"
            ? "h-[calc(100svh-2rem)] max-h-none max-w-[calc(100vw-2rem)]"
            : "max-h-[88svh] max-w-3xl",
        ].join(" ")}
        onKeyDown={handleKeyDown}
      >
        <header className="flex items-center justify-between border-b border-app-border px-4 py-3">
          <h2 id={titleId} className="text-sm font-semibold text-app-text">
            {title}
          </h2>
          <AppIconButton icon={X} label="Close" onClick={onClose} />
        </header>
        <div className="min-h-0 overflow-y-auto p-4">{children}</div>
      </dialog>
    </div>,
    document.body,
  );
}
