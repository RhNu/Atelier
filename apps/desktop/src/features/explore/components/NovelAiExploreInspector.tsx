import { useCallback, useState } from "react";
import { useTranslation } from "react-i18next";

import { AppButton, AppPanel, EmptyState } from "@/components/ui";
import {
  generationModelDisplayNames,
  generationSamplerDisplayNames,
} from "@/features/generation/model/generation-options";
import { desktopApi } from "@/platform/atelier";
import { useToastStore } from "@/stores/toast-store";
import type { ExploreCharacterCaptionDto, NovelAiExplorePostDetailDto } from "@/types";

import { useCopyExploreText } from "../data/useExploreQueries";
import { formatError } from "../explore-utils";
import { ExploreImage } from "./ExploreImage";

type Props = {
  detail: NovelAiExplorePostDetailDto | undefined;
  pending: boolean;
  error: unknown;
  blurSensitive: boolean;
  onCreator: (id: string) => void;
  onRetry: () => void;
};

export function NovelAiExploreInspector({
  detail,
  pending,
  error,
  blurSensitive,
  onCreator,
  onRetry,
}: Props) {
  const { t } = useTranslation("explore");
  const [revealed, setRevealed] = useState(false);
  const pushToast = useToastStore((state) => state.push);
  const reveal = useCallback(() => setRevealed((current) => !current), []);
  const creator = useCallback(() => {
    if (detail?.post.creator_id) onCreator(detail.post.creator_id);
  }, [detail, onCreator]);
  const open = useCallback(() => {
    if (detail)
      void desktopApi
        .openExternalUrl(detail.page_url)
        .catch((reason: unknown) =>
          pushToast({ level: "error", title: t("openFailed"), message: formatError(reason) }),
        );
  }, [detail, pushToast, t]);
  if (pending)
    return (
      <AppPanel variant="section" className="p-3 text-sm text-app-muted">
        {t("loadingDetail")}
      </AppPanel>
    );
  if (error)
    return (
      <AppPanel variant="section">
        <EmptyState title={t("detailFailed")} description={formatError(error)} />
        <AppButton onClick={onRetry}>{t("retry")}</AppButton>
      </AppPanel>
    );
  if (!detail)
    return (
      <AppPanel variant="section">
        <EmptyState title={t("novelai.selectDetail")} />
      </AppPanel>
    );
  const metadata = detail.metadata;
  return (
    <AppPanel variant="section" className="min-h-0 overflow-auto">
      <header className="grid gap-2 border-b border-app-border p-3">
        <h2 className="text-sm font-semibold">{detail.post.title || t("novelai.untitled")}</h2>
        <div className="flex flex-wrap items-center justify-between gap-2">
          <AppButton variant="ghost" disabled={!detail.post.creator_id} onClick={creator}>
            {detail.post.creator_name ?? t("novelai.unknownCreator")}
          </AppButton>
          <AppButton variant="ghost" onClick={open}>
            {t("novelai.openPost")}
          </AppButton>
        </div>
        <p className="text-xs text-app-muted">
          {detail.created_at} · {detail.post.width}×{detail.post.height}
        </p>
        {detail.post.like_count !== null ? (
          <p className="text-xs text-app-muted">
            {t("novelai.likes", { count: detail.post.like_count })}
          </p>
        ) : null}
      </header>
      <div className="overflow-hidden">
        <ExploreImage
          sourceId="novelai_explore_gallery"
          itemId={detail.post.id}
          variant="preview"
          alt={detail.post.title}
          className="aspect-video max-h-[40vh] w-full bg-app-bg"
          blurred={blurSensitive && !revealed}
          eager
        />
      </div>
      <div className="grid gap-4 p-3">
        {blurSensitive ? (
          <AppButton variant="secondary" onClick={reveal}>
            {revealed ? t("novelai.hideImage") : t("novelai.revealImage")}
          </AppButton>
        ) : null}
        {detail.description ? (
          <p className="text-xs whitespace-pre-wrap">{detail.description}</p>
        ) : null}
        {metadata.status !== "available" ? (
          <output className="text-xs text-app-muted">
            {t(`novelai.metadataStates.${metadata.status}`)}
          </output>
        ) : null}
        {metadata.warnings.map((warning, index) => (
          <p key={`${index}:${warning}`} className="text-xs text-amber-200">
            {warning}
          </p>
        ))}
        {metadata.prompt !== null ? (
          <CopyText title={t("novelai.positive")} text={metadata.prompt} />
        ) : null}
        {metadata.negative_prompt !== null ? (
          <CopyText title={t("novelai.negative")} text={metadata.negative_prompt} />
        ) : null}
        <CharacterPrompts
          title={t("novelai.characters")}
          captions={metadata.characters}
          useCoords={metadata.use_coords}
          useOrder={metadata.use_order}
        />
        <CharacterPrompts
          title={t("novelai.negativeCharacters")}
          captions={metadata.negative_characters}
          useCoords={metadata.negative_use_coords}
          useOrder={metadata.negative_use_order}
        />
        {metadata.parameters.length > 0 ? (
          <section className="border-t border-app-border pt-3">
            <h3 className="mb-2 text-xs font-semibold">{t("novelai.parameters")}</h3>
            <dl className="grid grid-cols-[minmax(0,1fr)_minmax(0,1fr)] gap-2 text-xs">
              {metadata.parameters.map((parameter) => (
                <Parameter key={parameter.name} name={parameter.name} value={parameter.value} />
              ))}
            </dl>
          </section>
        ) : null}
        {metadata.raw ? (
          <details className="border-t border-app-border pt-3">
            <summary className="cursor-pointer text-xs text-app-muted">
              {t("novelai.rawMetadata")}
            </summary>
            <CopyText title={t("novelai.rawMetadata")} text={metadata.raw} />
          </details>
        ) : null}
      </div>
    </AppPanel>
  );
}

function CopyText({ title, text }: { title: string; text: string }) {
  const { t } = useTranslation("explore");
  const copy = useCopyExploreText();
  const copyText = useCallback(() => copy.mutate(text), [copy, text]);
  return (
    <section className="min-w-0 border-t border-app-border pt-3">
      <div className="mb-2 flex items-center justify-between gap-2">
        <h3 className="text-xs font-semibold">{title}</h3>
        <AppButton
          variant="ghost"
          disabled={copy.isPending}
          onClick={copyText}
          aria-label={t("novelai.copySection", { section: title })}
        >
          {t("novelai.copy")}
        </AppButton>
      </div>
      <pre className="max-h-64 overflow-auto border border-app-border bg-app-bg p-2 text-xs break-words whitespace-pre-wrap">
        {text}
      </pre>
    </section>
  );
}

function CharacterPrompts({
  title,
  captions,
  useCoords,
  useOrder,
}: {
  title: string;
  captions: ExploreCharacterCaptionDto[];
  useCoords: boolean | null;
  useOrder: boolean | null;
}) {
  const { t } = useTranslation("explore");
  if (captions.length === 0) return null;
  return (
    <section className="grid gap-2">
      <h3 className="text-xs font-semibold">{title}</h3>
      <p className="text-xs text-app-muted">
        {useCoords !== null ? t("novelai.coordinateMode", { value: String(useCoords) }) : ""}{" "}
        {useOrder !== null ? t("novelai.orderMode", { value: String(useOrder) }) : ""}
      </p>
      {captions.map((caption, index) => (
        <div key={index}>
          <CopyText title={`${title} ${index + 1}`} text={caption.text} />
          {caption.centers.length > 0 ? (
            <p className="pt-1 text-xs text-app-muted">
              {t("novelai.coordinates")}:{" "}
              {caption.centers.map((p) => `(${p.x}, ${p.y})`).join(" · ")}
            </p>
          ) : null}
        </div>
      ))}
    </section>
  );
}

function Parameter({ name, value }: { name: string; value: string }) {
  const models: Record<string, string> = generationModelDisplayNames;
  const samplers: Record<string, string> = generationSamplerDisplayNames;
  const display =
    name === "sampler"
      ? (samplers[value] ?? value)
      : name === "model" || name === "model_name"
        ? (models[value] ?? value)
        : value;
  return (
    <>
      <dt className="break-words text-app-muted">{name}</dt>
      <dd className="break-words">{display}</dd>
    </>
  );
}
