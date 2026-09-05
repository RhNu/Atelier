/* eslint-disable max-lines-per-function */
import { useCallback, useEffect, useRef, useState } from "react";

import { reportBackgroundPromise, runLoggedAction } from "@/app/logger";
import { desktopApi, resourceApi, uniqueImportedImageResources } from "@/platform/atelier";
import type { DirectorToolDto, DirectorToolResultDto } from "@/types";

import { formatError } from "../gallery/gallery-utils";
import {
  DirectorInputPanel,
  DirectorPreviewPanel,
  DirectorRunPanel,
} from "./components/DirectorPanels";
import {
  useDirectorImageQuery,
  usePickDirectorImageMutation,
  useReleaseDirectorImagesMutation,
  useRunDirectorToolMutation,
  useSaveDirectorImageMutation,
  useSetDirectorSafetyOverrideMutation,
} from "./data/useDirectorActions";
import { useDirectorReadinessQuery } from "./data/useDirectorReadinessQuery";
import {
  buildDirectorRunRequest,
  clampDefry,
  parseDirectorTool,
  parseSafetyOverride,
  type DirectorInput,
} from "./director-model";
import { useDirectorHandoffStore } from "./state/director-handoff-store";

export function DirectorPage() {
  const readinessQuery = useDirectorReadinessQuery();
  const refetchReadiness = readinessQuery.refetch;
  const pickInputMutation = usePickDirectorImageMutation();
  const runToolMutation = useRunDirectorToolMutation();
  const releaseImagesMutation = useReleaseDirectorImagesMutation();
  const saveImageMutation = useSaveDirectorImageMutation();
  const safetyMutation = useSetDirectorSafetyOverrideMutation();
  const consumeHandoff = useDirectorHandoffStore((state) => state.consumePendingInput);

  const [input, setInput] = useState<DirectorInput | null>(null);
  const [tool, setTool] = useState<DirectorToolDto>("lineart");
  const [prompt, setPrompt] = useState("");
  const [defry, setDefry] = useState(0);
  const [result, setResult] = useState<DirectorToolResultDto | null>(null);
  const [safetyOverride, setSafetyOverride] = useState("");
  const [actionError, setActionError] = useState<string | null>(null);
  const latestInput = useRef(input);
  latestInput.current = input;

  useEffect(
    () => () => {
      const resource =
        latestInput.current?.kind === "resource" ? latestInput.current.resource : null;
      const resources = uniqueImportedImageResources([resource]);
      if (resources.length > 0) {
        reportBackgroundPromise(
          resourceApi.releaseImportedImages({ resources }),
          "Release Director input resource on unmount",
          { count: resources.length },
        );
      }
    },
    [],
  );

  const sourceResource = input?.kind === "resource" ? input.resource : null;
  const sourceImageQuery = useDirectorImageQuery(sourceResource);
  const resultImageQuery = useDirectorImageQuery(result?.resource ?? null);
  const sourceImageSrc = input?.kind === "inline" ? input.src : (sourceImageQuery.data ?? null);
  const resultImageSrc = resultImageQuery.data ?? null;
  const showsPrompt = tool === "colorize" || tool === "emotion";
  const promptRequired = tool === "emotion";
  const canRun =
    Boolean(input) && !runToolMutation.isPending && (!promptRequired || prompt.trim().length > 0);

  useEffect(() => {
    const resource = consumeHandoff();
    if (resource) {
      const replaced =
        latestInput.current?.kind === "resource" ? latestInput.current.resource : null;
      setInput({ kind: "resource", resource, label: resource.id });
      setResult(null);
      setActionError(null);
      reportBackgroundPromise(
        releaseImagesMutation.mutateAsync([replaced]),
        "Release replaced Director input resource",
      );
    }
  }, [consumeHandoff, releaseImagesMutation]);

  useEffect(() => {
    setSafetyOverride(result?.item.manual_safety_override ?? "");
  }, [result?.item.item_id, result?.item.manual_safety_override]);

  const handlePickInput = useCallback(() => {
    setActionError(null);
    void pickInputMutation
      .mutateAsync()
      .then((resource) => {
        if (resource) {
          const replaced = input?.kind === "resource" ? input.resource : null;
          setInput({ kind: "resource", resource, label: resource.id });
          setResult(null);
          reportBackgroundPromise(
            releaseImagesMutation.mutateAsync([replaced]),
            "Release replaced Director input resource",
          );
        }
      })
      .catch((error: unknown) => setActionError(formatError(error)));
  }, [input, pickInputMutation, releaseImagesMutation]);

  const handlePasteInput = useCallback(() => {
    setActionError(null);
    void runLoggedAction("Read Director input image from clipboard", () =>
      desktopApi.readClipboardImage(),
    )
      .then((pasted) => {
        const replaced = input?.kind === "resource" ? input.resource : null;
        setInput({
          kind: "inline",
          imageBase64: pasted.imageBase64,
          src: `data:${pasted.mimeType};base64,${pasted.imageBase64}`,
          label: "Clipboard image",
        });
        setResult(null);
        reportBackgroundPromise(
          releaseImagesMutation.mutateAsync([replaced]),
          "Release replaced Director input resource",
        );
      })
      .catch((error: unknown) => setActionError(formatError(error)));
  }, [input, releaseImagesMutation]);

  const handleClearInput = useCallback(() => {
    const replaced = input?.kind === "resource" ? input.resource : null;
    setInput(null);
    setResult(null);
    setActionError(null);
    reportBackgroundPromise(
      releaseImagesMutation.mutateAsync([replaced]),
      "Release cleared Director input resource",
    );
  }, [input, releaseImagesMutation]);

  const handleToolChange = useCallback((value: string) => {
    setTool(parseDirectorTool(value));
    setResult(null);
  }, []);

  const handlePromptChange = useCallback((value: string) => setPrompt(value), []);

  const handleDefryChange = useCallback((value: number) => {
    setDefry(clampDefry(value));
  }, []);

  const handleRefreshReadiness = useCallback(() => {
    reportBackgroundPromise(refetchReadiness(), "Refresh Director readiness");
  }, [refetchReadiness]);

  const handleRunTool = useCallback(() => {
    if (!input || !canRun) {
      return;
    }
    setActionError(null);
    void runToolMutation
      .mutateAsync(buildDirectorRunRequest(input, tool, prompt, defry))
      .then(setResult)
      .catch((error: unknown) => setActionError(formatError(error)));
  }, [canRun, defry, input, prompt, runToolMutation, tool]);

  const handleSaveResult = useCallback(() => {
    if (!result) {
      return;
    }
    setActionError(null);
    void saveImageMutation
      .mutateAsync({
        resource: result.resource,
        format: null,
        suggested_file_name: `${result.item_id}-${tool}`,
      })
      .catch((error: unknown) => setActionError(formatError(error)));
  }, [result, saveImageMutation, tool]);

  const handleSafetyChange = useCallback((value: string) => {
    setSafetyOverride(value);
  }, []);

  const handleApplySafety = useCallback(() => {
    if (!result) {
      return;
    }
    setActionError(null);
    void safetyMutation
      .mutateAsync({
        item_id: result.item_id,
        manual_safety_override: parseSafetyOverride(safetyOverride),
      })
      .then((item) => setResult({ ...result, item }))
      .catch((error: unknown) => setActionError(formatError(error)));
  }, [result, safetyMutation, safetyOverride]);

  return (
    <div className="flex h-full min-h-0 flex-col">
      {actionError ? (
        <p className="border-b border-app-border bg-rose-950/40 px-3 py-2 text-sm text-rose-100">
          {actionError}
        </p>
      ) : null}

      <div className="grid min-h-0 flex-1 grid-cols-[340px_minmax(0,1fr)_340px] divide-x divide-app-border">
        <DirectorInputPanel
          input={input}
          imageSrc={sourceImageSrc}
          loadingImage={sourceImageQuery.isPending && Boolean(sourceResource)}
          imageError={sourceImageQuery.isError ? formatError(sourceImageQuery.error) : null}
          pickPending={pickInputMutation.isPending}
          onPick={handlePickInput}
          onPaste={handlePasteInput}
          onClear={handleClearInput}
        />

        <DirectorPreviewPanel
          resultSrc={resultImageSrc}
          resultPending={resultImageQuery.isPending && Boolean(result)}
          resultError={resultImageQuery.isError ? formatError(resultImageQuery.error) : null}
        />

        <DirectorRunPanel
          tool={tool}
          anlas={readinessQuery.data?.anlas_balance ?? null}
          showsPrompt={showsPrompt}
          promptRequired={promptRequired}
          prompt={prompt}
          defry={defry}
          canRun={canRun}
          runPending={runToolMutation.isPending}
          result={result}
          safetyOverride={safetyOverride}
          readinessPending={readinessQuery.isFetching}
          readinessError={readinessQuery.isError ? formatError(readinessQuery.error) : null}
          savePending={saveImageMutation.isPending}
          safetyPending={safetyMutation.isPending}
          onToolChange={handleToolChange}
          onPromptChange={handlePromptChange}
          onDefryChange={handleDefryChange}
          onRefresh={handleRefreshReadiness}
          onRun={handleRunTool}
          onSave={handleSaveResult}
          onSafetyChange={handleSafetyChange}
          onApplySafety={handleApplySafety}
        />
      </div>
    </div>
  );
}
