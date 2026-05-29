import { useCallback } from "react";

import type { GenerationDraft } from "../model/generation-draft";
import {
  toImageFormat,
  toImageModel,
  toNoiseSchedule,
  toSampler,
  toUcPreset,
} from "../model/generation-options";

type GenerationParamHandlerProps = {
  onPatch: (patch: Partial<GenerationDraft>) => void;
  onPatchSize: (patch: Partial<GenerationDraft["size"]>) => void;
};

export function useGenerationParamHandlers({ onPatch, onPatchSize }: GenerationParamHandlerProps) {
  const handleModelChange = useCallback(
    (value: string) => {
      onPatch({ model: toImageModel(value) });
    },
    [onPatch],
  );
  const handleWidthChange = useCallback(
    (width: number) => {
      onPatchSize({ width });
    },
    [onPatchSize],
  );
  const handleHeightChange = useCallback(
    (height: number) => {
      onPatchSize({ height });
    },
    [onPatchSize],
  );
  const handleStepsChange = useCallback(
    (steps: number) => {
      onPatch({ steps });
    },
    [onPatch],
  );
  const handleScaleChange = useCallback(
    (scale: number) => {
      onPatch({ scale });
    },
    [onPatch],
  );
  const handleSeedChange = useCallback(
    (seed: number) => {
      onPatch({ seed });
    },
    [onPatch],
  );
  const handleSeedModeChange = useCallback(
    (value: string) => {
      onPatch({ seedMode: value === "fixed" ? "fixed" : "random" });
    },
    [onPatch],
  );
  const handleSamplesChange = useCallback(
    (nSamples: number) => {
      onPatch({ nSamples });
    },
    [onPatch],
  );
  const handleRequestCountChange = useCallback(
    (requestCount: number) => {
      onPatch({ requestCount });
    },
    [onPatch],
  );
  const handleSamplerChange = useCallback(
    (value: string) => {
      onPatch({ sampler: toSampler(value) });
    },
    [onPatch],
  );
  const handleNoiseScheduleChange = useCallback(
    (value: string) => {
      onPatch({ noiseSchedule: toNoiseSchedule(value) });
    },
    [onPatch],
  );
  const handleUcPresetChange = useCallback(
    (value: string) => {
      onPatch({ ucPreset: toUcPreset(value) });
    },
    [onPatch],
  );
  const handleImageFormatChange = useCallback(
    (value: string) => {
      onPatch({ imageFormat: toImageFormat(value) });
    },
    [onPatch],
  );
  const handleCfgRescaleChange = useCallback(
    (cfgRescale: number) => {
      onPatch({ cfgRescale });
    },
    [onPatch],
  );
  const handleQualityChange = useCallback(
    (quality: boolean) => {
      onPatch({ quality });
    },
    [onPatch],
  );
  const handleVarietyBoostChange = useCallback(
    (varietyBoost: boolean) => {
      onPatch({ varietyBoost });
    },
    [onPatch],
  );
  const handleStrictModeChange = useCallback(
    (strictMode: boolean) => {
      onPatch({ strictMode });
    },
    [onPatch],
  );
  const handleStreamEnabledChange = useCallback(
    (streamEnabled: boolean) => {
      onPatch({ streamEnabled });
    },
    [onPatch],
  );

  return {
    handleModelChange,
    handleWidthChange,
    handleHeightChange,
    handleStepsChange,
    handleScaleChange,
    handleSeedChange,
    handleSeedModeChange,
    handleSamplesChange,
    handleRequestCountChange,
    handleSamplerChange,
    handleNoiseScheduleChange,
    handleUcPresetChange,
    handleImageFormatChange,
    handleCfgRescaleChange,
    handleQualityChange,
    handleVarietyBoostChange,
    handleStrictModeChange,
    handleStreamEnabledChange,
  };
}
