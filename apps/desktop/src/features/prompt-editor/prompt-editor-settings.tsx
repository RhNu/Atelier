import type { ReactNode } from "react";

import { promptEditorSettingsContext } from "./prompt-editor-settings-context";

export function PromptEditorSettingsProvider({
  convertFullWidthPunctuation,
  children,
}: {
  convertFullWidthPunctuation: boolean;
  children: ReactNode;
}) {
  return (
    <promptEditorSettingsContext.Provider value={convertFullWidthPunctuation}>
      {children}
    </promptEditorSettingsContext.Provider>
  );
}
