import { useEffect } from "react";

const selections = new Map<symbol, string>();

export function useSelectedPromptResource(resourceId: string | null | undefined) {
  useEffect(() => {
    if (!resourceId) return;
    const owner = Symbol();
    selections.set(owner, resourceId);
    return () => {
      selections.delete(owner);
    };
  }, [resourceId]);
}

export function selectedPromptResourceIds(): string[] {
  return [...new Set(selections.values())];
}
