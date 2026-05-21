import { create } from "zustand";

export type GenerationDraftState = {
  prompt: string;
  negativePrompt: string;
  setPrompt: (prompt: string) => void;
  setNegativePrompt: (negativePrompt: string) => void;
};

export type SelectionState = {
  selectedHistoryItemId: string | null;
  selectedGalleryItemId: string | null;
  setSelectedHistoryItemId: (itemId: string | null) => void;
  setSelectedGalleryItemId: (itemId: string | null) => void;
};

export type TemporaryEditorState = {
  directorNote: string;
  lexiconSearch: string;
  setDirectorNote: (directorNote: string) => void;
  setLexiconSearch: (lexiconSearch: string) => void;
};

export const useGenerationDraftStore = create<GenerationDraftState>((set) => ({
  prompt: "",
  negativePrompt: "",
  setPrompt: (prompt) => set({ prompt }),
  setNegativePrompt: (negativePrompt) => set({ negativePrompt }),
}));

export const useSelectionStore = create<SelectionState>((set) => ({
  selectedHistoryItemId: null,
  selectedGalleryItemId: null,
  setSelectedHistoryItemId: (selectedHistoryItemId) => set({ selectedHistoryItemId }),
  setSelectedGalleryItemId: (selectedGalleryItemId) => set({ selectedGalleryItemId }),
}));

export const useTemporaryEditorStore = create<TemporaryEditorState>((set) => ({
  directorNote: "",
  lexiconSearch: "",
  setDirectorNote: (directorNote) => set({ directorNote }),
  setLexiconSearch: (lexiconSearch) => set({ lexiconSearch }),
}));
