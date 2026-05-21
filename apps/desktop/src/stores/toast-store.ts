import { create } from "zustand";

export type ToastLevel = "info" | "success" | "warning" | "error";

export type ToastItem = {
  id: string;
  level: ToastLevel;
  title?: string;
  message: string;
};

type ToastStore = {
  toasts: ToastItem[];
  push: (toast: Omit<ToastItem, "id">) => string;
  remove: (id: string) => void;
};

export const useToastStore = create<ToastStore>((set) => ({
  toasts: [],
  push: (toast) => {
    const id = crypto.randomUUID();
    set((state) => ({ toasts: [...state.toasts, { ...toast, id }] }));
    return id;
  },
  remove: (id) => set((state) => ({ toasts: state.toasts.filter((toast) => toast.id !== id) })),
}));
