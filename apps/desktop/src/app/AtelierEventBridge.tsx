import { useQueryClient } from "@tanstack/react-query";
import { useEffect } from "react";

import { recordGenerationEvent } from "../features/generation/state/generation-event-store";
import {
  applyAtelierEventInvalidations,
  listenToAtelierEvents,
  recoverAtelierEvents,
} from "../platform/atelier";
import type { AppEventDto } from "../types";

export function AtelierEventBridge() {
  const queryClient = useQueryClient();

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | null = null;
    let cursor = 0;
    let recovering = true;
    let pending: AppEventDto[] = [];

    const deliver = (event: AppEventDto) => {
      if (disposed || event.sequence <= cursor) {
        return;
      }
      cursor = event.sequence;
      recordGenerationEvent(event);
      applyAtelierEventInvalidations(queryClient, event);
    };

    const flushPending = () => {
      pending.sort((left, right) => left.sequence - right.sequence);
      for (const event of pending) {
        deliver(event);
      }
      pending = [];
    };

    const recover = async () => {
      try {
        cursor = await recoverAtelierEvents(cursor, deliver);
      } catch (error: unknown) {
        console.error("Failed to recover Atelier events", error);
      } finally {
        recovering = false;
        flushPending();
      }
    };

    listenToAtelierEvents((event) => {
      if (event.sequence === 1 && cursor > 1) {
        cursor = 0;
        recovering = true;
        pending.push(event);
        void recover();
        return;
      }
      if (recovering || event.sequence > cursor + 1) {
        pending.push(event);
        if (!recovering) {
          recovering = true;
          void recover();
        }
        return;
      }
      deliver(event);
    })
      .then((nextUnlisten) => {
        if (disposed) {
          nextUnlisten();
          return;
        }

        unlisten = nextUnlisten;
        void recover();
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
