import type { ChangeEvent } from "react";
import { useCallback, useMemo } from "react";
import { useTranslation } from "react-i18next";

import { AppSelect } from "@/components/ui";
import { useImageModelCatalog } from "@/features/generation/data/useImageModelCatalog";
import {
  generationModelDisplayNames,
  generationModelOptions,
  toImageModel,
} from "@/features/generation/model/generation-options";
import type { ImageModelDto } from "@/types";

const MODEL_GROUPS: ReadonlyArray<{
  label: string;
  models: ReadonlyArray<ImageModelDto>;
}> = [
  { label: "V5", models: ["nai-diffusion-5-full", "nai-diffusion-5-curated"] },
  { label: "V4.5", models: ["nai-diffusion-4-5-full", "nai-diffusion-4-5-curated"] },
  { label: "V4", models: ["nai-diffusion-4-full", "nai-diffusion-4-curated"] },
  { label: "V3", models: ["nai-diffusion-3", "nai-diffusion-furry-3"] },
];

export function ModelBindingField({
  models,
  onChange,
}: {
  models: ImageModelDto[];
  onChange: (models: ImageModelDto[]) => void;
}) {
  const { t } = useTranslation("resources");
  const catalog = useImageModelCatalog();
  const available = new Set(catalog.data?.map(({ model }) => model) ?? generationModelOptions);
  return (
    <fieldset className="grid gap-3 border border-app-border p-3">
      <legend className="px-1 text-xs font-semibold text-app-muted uppercase">{t("models")}</legend>
      {MODEL_GROUPS.map((group) => (
        <div key={group.label} className="grid gap-1">
          <span className="text-[11px] font-semibold text-app-muted">{group.label}</span>
          <div className="grid grid-cols-2 gap-2">
            {group.models
              .filter((model) => available.has(model))
              .map((model) => (
                <ModelCheckbox key={model} model={model} models={models} onChange={onChange} />
              ))}
          </div>
        </div>
      ))}
    </fieldset>
  );
}

function ModelCheckbox({
  model,
  models,
  onChange,
}: {
  model: ImageModelDto;
  models: ImageModelDto[];
  onChange: (models: ImageModelDto[]) => void;
}) {
  const handleChange = useCallback(
    (event: ChangeEvent<HTMLInputElement>) => {
      onChange(event.target.checked ? [...models, model] : models.filter((item) => item !== model));
    },
    [model, models, onChange],
  );
  return (
    <label className="flex items-center gap-2 text-xs text-app-text">
      <input
        aria-label={generationModelDisplayNames[model]}
        type="checkbox"
        checked={models.includes(model)}
        onChange={handleChange}
      />
      {generationModelDisplayNames[model]}
    </label>
  );
}

export function PreviewModelField({
  models,
  value,
  onChange,
}: {
  models: ImageModelDto[];
  value: ImageModelDto;
  onChange: (model: ImageModelDto) => void;
}) {
  const { t } = useTranslation("resources");
  const options = useMemo(
    () =>
      models.map((model) => ({
        value: model,
        label: generationModelDisplayNames[model],
      })),
    [models],
  );
  const handleChange = useCallback((model: string) => onChange(toImageModel(model)), [onChange]);
  return models.length > 1 ? (
    <label className="grid gap-1 text-xs font-semibold text-app-muted">
      {t("previewModel")}
      <AppSelect
        aria-label={t("previewModel")}
        value={value}
        options={options}
        onValueChange={handleChange}
      />
    </label>
  ) : null;
}
