import { X } from "lucide-react";
import type { ReactNode } from "react";

import { AppIconButton } from "./AppIconButton";

type AppModalProps = {
  open: boolean;
  title: string;
  children: ReactNode;
  onClose: () => void;
};

export function AppModal({ open, title, children, onClose }: AppModalProps) {
  if (!open) {
    return null;
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/58 px-4 backdrop-blur-sm">
      <dialog
        open
        aria-modal="true"
        aria-label={title}
        className="max-h-[88vh] w-full max-w-3xl overflow-hidden border border-app-border bg-app-panel shadow-app-panel"
      >
        <header className="flex items-center justify-between border-b border-app-border px-4 py-3">
          <h2 className="text-sm font-semibold text-app-text">{title}</h2>
          <AppIconButton icon={X} label="Close" onClick={onClose} />
        </header>
        <div className="max-h-[calc(88vh-56px)] overflow-y-auto p-4">{children}</div>
      </dialog>
    </div>
  );
}
