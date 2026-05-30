import { create } from "zustand";

import type { ResourceRefDto } from "../../../types";

type DirectorHandoffState = {
  pendingInput: ResourceRefDto | null;
  setPendingInput: (resource: ResourceRefDto) => void;
  consumePendingInput: () => ResourceRefDto | null;
};

export const useDirectorHandoffStore = create<DirectorHandoffState>((set, get) => ({
  pendingInput: null,
  setPendingInput: (resource) => set({ pendingInput: resource }),
  consumePendingInput: () => {
    const resource = get().pendingInput;
    set({ pendingInput: null });
    return resource;
  },
}));

export function setDirectorHandoffInput(resource: ResourceRefDto): void {
  useDirectorHandoffStore.getState().setPendingInput(resource);
}
