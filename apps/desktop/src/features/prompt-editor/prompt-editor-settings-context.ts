import { createContext, useContext } from "react";

export const promptEditorSettingsContext = createContext(false);

export function usePromptEditorSettings(): boolean {
  return useContext(promptEditorSettingsContext);
}
