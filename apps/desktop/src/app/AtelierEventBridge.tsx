import { useQueryClient } from "@tanstack/react-query";
import { useEffect } from "react";

import { applyAtelierEventInvalidations, listenToAtelierEvents } from "../platform/atelier";

export function AtelierEventBridge() {
  const queryClient = useQueryClient();

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | null = null;

    listenToAtelierEvents((event) => {
      applyAtelierEventInvalidations(queryClient, event);
    })
      .then((nextUnlisten) => {
        if (disposed) {
          nextUnlisten();
          return;
        }

        unlisten = nextUnlisten;
      })
      .catch((error: unknown) => {
        console.error("Failed to attach Atelier event listener", error);
      });

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [queryClient]);

  return null;
}
