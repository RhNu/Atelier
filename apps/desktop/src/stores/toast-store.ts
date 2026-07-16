import { create } from "zustand";

export type ToastLevel = "info" | "success" | "warning" | "error";

export type ToastItem = {
  id: string;
  level: ToastLevel;
  title?: string;
  message: string;
  durationMs: number | null;
  action?: {
    label: string;
    onClick: () => void;
  };
};

type ToastStore = {
  toasts: ToastItem[];
  push: (toast: Omit<ToastItem, "id" | "durationMs"> & { durationMs?: number | null }) => string;
  remove: (id: string) => void;
};

export const useToastStore = create<ToastStore>((set) => ({
  toasts: [],
  push: (toast) => {
    const id = crypto.randomUUID();
    set((state) => ({
      toasts: [...state.toasts, { durationMs: 5_000, ...toast, id }],
    }));
    return id;
  },
  remove: (id) => set((state) => ({ toasts: state.toasts.filter((toast) => toast.id !== id) })),
}));
