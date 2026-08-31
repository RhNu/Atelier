import { create } from "zustand";

// A new key namespace prevents requests from a replaced account repopulating active data.
export const useExploreIdentity = create<{ revision: number; advance: () => void }>((set) => ({
  revision: 0,
  advance: () => set((state) => ({ revision: state.revision + 1 })),
}));
