import { useCallback } from "react";

import { formatGenerationError } from "../generation-page-utils";

type QueueActionHandlersProps = {
  pause: () => Promise<unknown>;
  resume: () => Promise<unknown>;
  stop: () => Promise<unknown>;
  setQueueError: (error: string | null) => void;
};

export function useQueueActionHandlers({
  pause,
  resume,
  stop,
  setQueueError,
}: QueueActionHandlersProps) {
  const runQueueCommand = useCallback(
    (command: () => Promise<unknown>) => {
      setQueueError(null);
      void command().catch((error: unknown) => {
        setQueueError(formatGenerationError(error));
      });
    },
    [setQueueError],
  );
  const handlePause = useCallback(() => runQueueCommand(pause), [pause, runQueueCommand]);
  const handleResume = useCallback(() => runQueueCommand(resume), [resume, runQueueCommand]);
  const handleStop = useCallback(() => runQueueCommand(stop), [runQueueCommand, stop]);
  return { handlePause, handleResume, handleStop };
}
