/* eslint-disable react-perf/jsx-no-new-function-as-prop, react-perf/jsx-no-new-object-as-prop */
import { useVirtualizer } from "@tanstack/react-virtual";
import { AlertTriangle, Check, ChevronRight, Loader2, RotateCcw, Wrench, X } from "lucide-react";
import { useEffect, useMemo, useRef } from "react";
import { useTranslation } from "react-i18next";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";

import { AppButton } from "@/components/ui";
import type { AgentEventDto, AgentTurnEventDto } from "@/types";

import {
  buildDisplayAgentEvents,
  AGENT_TOOL_NAME_KEYS,
  humanizeAgentToolName,
  prettyAgentJson,
  type DisplayAgentEvent,
} from "../model/agent-event-model";

type AgentEventListProps = {
  events: AgentEventDto[];
  liveEvents: AgentTurnEventDto[];
  pendingUserMessage: string | null;
  running: boolean;
  onApproval: (approvalId: string, approved: boolean) => void;
  onUndo: (actionId: string) => void;
};

const MARKDOWN_PLUGINS = [remarkGfm];

export function AgentEventList(props: AgentEventListProps) {
  const { t } = useTranslation("agent");
  const parentRef = useRef<HTMLDivElement>(null);
  const items = useMemo(() => buildDisplayAgentEvents(props), [props]);
  const virtualizer = useVirtualizer({
    count: items.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 108,
    overscan: 6,
  });

  useEffect(() => {
    if (items.length > 0) virtualizer.scrollToIndex(items.length - 1, { align: "end" });
  }, [items.length, virtualizer]);

  if (items.length === 0) {
    return (
      <div className="grid min-h-0 flex-1 place-items-center p-8 text-center text-sm text-app-muted">
        <div>
          <p className="font-semibold text-app-text">{t("emptyConversation")}</p>
          <p className="mt-2 max-w-72 text-xs leading-5">{t("emptyConversationDescription")}</p>
        </div>
      </div>
    );
  }

  return (
    <div ref={parentRef} className="min-h-0 flex-1 overflow-auto px-3 py-4">
      <div className="relative w-full" style={{ height: virtualizer.getTotalSize() }}>
        {virtualizer.getVirtualItems().map((row) => {
          const item = items[row.index];
          return (
            <div
              key={item.id}
              ref={virtualizer.measureElement}
              data-index={row.index}
              className="absolute top-0 left-0 w-full pb-3"
              style={{ transform: `translateY(${row.start}px)` }}
            >
              <AgentEventCard item={item} onApproval={props.onApproval} onUndo={props.onUndo} />
            </div>
          );
        })}
      </div>
    </div>
  );
}

function AgentEventCard({
  item,
  onApproval,
  onUndo,
}: {
  item: DisplayAgentEvent;
  onApproval: AgentEventListProps["onApproval"];
  onUndo: AgentEventListProps["onUndo"];
}) {
  const { t } = useTranslation("agent");
  if (item.kind === "user" || item.kind === "assistant") {
    return (
      <article
        className={[
          "border px-3 py-2 text-sm leading-6",
          item.kind === "user"
            ? "ml-8 border-brand-400/40 bg-brand-500/12"
            : "mr-4 border-app-border bg-app-panel",
        ].join(" ")}
      >
        <p className="mb-1 text-[10px] font-semibold tracking-wider text-app-muted uppercase">
          {item.kind === "user" ? t("you") : t("assistant")}
          {item.kind === "assistant" && item.interrupted ? ` · ${t("interrupted")}` : ""}
        </p>
        <div className="agent-markdown break-words">
          <ReactMarkdown remarkPlugins={MARKDOWN_PLUGINS} skipHtml>
            {item.content}
          </ReactMarkdown>
        </div>
      </article>
    );
  }
  if (item.kind === "warning") {
    return (
      <div className="flex gap-2 border border-amber-500/40 bg-amber-500/10 p-3 text-xs text-amber-100">
        <AlertTriangle aria-hidden="true" className="mt-0.5 size-4 shrink-0" />
        {item.content}
      </div>
    );
  }
  if (item.kind === "approval_status") {
    return (
      <div className="flex items-center gap-2 border border-app-border bg-black/10 px-2.5 py-2 text-xs text-app-muted">
        {item.approved ? (
          <Check aria-hidden="true" className="size-3.5 text-emerald-300" />
        ) : (
          <X aria-hidden="true" className="size-3.5 text-rose-300" />
        )}
        {toolDisplayName(item.name, t)} · {item.approved ? t("approved") : t("denied")}
      </div>
    );
  }
  if (item.kind === "approval") {
    return (
      <article className="border border-amber-500/50 bg-amber-500/8 p-3">
        <p className="text-xs font-semibold text-amber-100">{t("approvalTitle")}</p>
        <p className="mt-1 text-sm text-white">{toolDisplayName(item.name, t)}</p>
        <ToolDetails label={t("toolArguments")} value={item.details} />
        <div className="mt-3 flex justify-end gap-2">
          <AppButton variant="ghost" onClick={() => onApproval(item.approvalId, false)}>
            {t("deny")}
          </AppButton>
          <AppButton onClick={() => onApproval(item.approvalId, true)}>{t("approve")}</AppButton>
        </div>
      </article>
    );
  }
  if (item.kind === "generation") {
    return (
      <div className="flex items-center gap-2 border border-emerald-500/40 bg-emerald-500/10 p-3 text-xs text-emerald-100">
        <Check aria-hidden="true" className="size-4" />
        {t("generationSubmitted")}
      </div>
    );
  }
  return (
    <article className="border border-app-border bg-black/15 px-2.5 py-2">
      <div className="flex items-center gap-2 text-xs font-semibold text-app-text">
        {item.state === "running" ? (
          <Loader2 aria-hidden="true" className="size-4 animate-spin text-brand-200" />
        ) : item.state === "failed" ? (
          <X aria-hidden="true" className="size-4 text-rose-300" />
        ) : (
          <Wrench aria-hidden="true" className="size-4 text-brand-200" />
        )}
        <span className="min-w-0 flex-1 truncate">{toolDisplayName(item.name, t)}</span>
        <span className="shrink-0 text-[10px] font-normal text-app-muted">
          {t(`toolState.${item.state}`)}
        </span>
      </div>
      {item.arguments ? <ToolDetails label={t("toolArguments")} value={item.arguments} /> : null}
      {item.result ? <ToolDetails label={t("toolResult")} value={item.result} /> : null}
      {item.actionId ? <UndoButton actionId={item.actionId} onUndo={onUndo} /> : null}
    </article>
  );
}

function ToolDetails({ label, value }: { label: string; value: string }) {
  return (
    <details className="group mt-1.5 border-t border-app-border/70 pt-1.5">
      <summary className="flex cursor-pointer list-none items-center gap-1 text-[10px] text-app-muted select-none hover:text-app-text">
        <ChevronRight
          aria-hidden="true"
          className="size-3 transition-transform group-open:rotate-90"
        />
        {label}
      </summary>
      <pre className="mt-1.5 max-h-40 overflow-auto bg-black/20 p-2 text-[11px] whitespace-pre-wrap text-app-muted">
        {prettyAgentJson(value)}
      </pre>
    </details>
  );
}

function toolDisplayName(name: string, t: ReturnType<typeof useTranslation<"agent">>["t"]): string {
  const key = AGENT_TOOL_NAME_KEYS[name];
  return key ? t(key) : humanizeAgentToolName(name);
}

function UndoButton({
  actionId,
  onUndo,
}: {
  actionId: string;
  onUndo: (actionId: string) => void;
}) {
  const { t } = useTranslation("agent");
  return (
    <AppButton className="mt-2 h-8" variant="ghost" onClick={() => onUndo(actionId)}>
      <RotateCcw aria-hidden="true" className="size-3.5" />
      {t("undo")}
    </AppButton>
  );
}
