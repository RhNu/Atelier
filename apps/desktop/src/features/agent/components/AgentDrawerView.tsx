/* eslint-disable react-perf/jsx-no-new-function-as-prop, react-perf/jsx-no-new-object-as-prop */
import { Bot, MessageSquarePlus, PanelRightClose, Send, Square, Trash2 } from "lucide-react";
import type { ChangeEvent, KeyboardEvent, PointerEvent } from "react";
import { useTranslation } from "react-i18next";

import { AppButton, AppIconButton, AppSelect, type SelectOption } from "@/components/ui";
import type { AgentEventDto, AgentTurnEventDto } from "@/types";

import { AgentEventList } from "./AgentEventList";

type AgentDrawerViewProps = {
  open: boolean;
  width: number;
  activeSessionId: string | null;
  defaultModelId: string | null;
  running: boolean;
  creating: boolean;
  deleting: boolean;
  sessionOptions: SelectOption[];
  events: AgentEventDto[];
  liveEvents: AgentTurnEventDto[];
  pendingUserMessage: string | null;
  error: string | null;
  message: string;
  onMessageChange: (message: string) => void;
  onClose: () => void;
  onResize: (event: PointerEvent<HTMLDivElement>) => void;
  onSelectSession: (sessionId: string) => void;
  onCreateSession: () => void;
  onDeleteSession: () => void;
  onOpenSettings: () => void;
  onApproval: (approvalId: string, approved: boolean) => void;
  onUndo: (actionId: string) => void;
  onSend: () => void;
  onStop: () => void;
};

export function AgentDrawerView(props: AgentDrawerViewProps) {
  const { t } = useTranslation("agent");
  const changeMessage = (event: ChangeEvent<HTMLTextAreaElement>) =>
    props.onMessageChange(event.target.value);
  const handleMessageKey = (event: KeyboardEvent<HTMLTextAreaElement>) => {
    if (event.key === "Enter" && (event.ctrlKey || event.metaKey)) {
      event.preventDefault();
      props.onSend();
    }
  };
  return (
    <aside
      aria-label={t("drawerLabel")}
      aria-hidden={!props.open}
      className={[
        "relative min-h-0 shrink-0 border-l border-app-border bg-app-bg shadow-2xl transition-[width]",
        props.open ? "" : "w-0 overflow-hidden border-l-0",
      ].join(" ")}
      style={props.open ? { width: props.width } : undefined}
    >
      <div
        aria-hidden="true"
        className="absolute inset-y-0 left-0 z-20 w-1 cursor-col-resize hover:bg-brand-400/60"
        onPointerDown={props.onResize}
      />
      <div className="flex h-full min-w-[360px] flex-col">
        <header className="flex h-14 shrink-0 items-center gap-2 border-b border-app-border px-3">
          <Bot aria-hidden="true" className="size-5 text-brand-200" />
          <div className="min-w-0 flex-1">
            <p className="truncate text-sm font-semibold text-white">{t("title")}</p>
            <p className="truncate text-[10px] text-app-muted">{t("subtitle")}</p>
          </div>
          <AppIconButton
            icon={PanelRightClose}
            label={t("close")}
            size="sm"
            onClick={props.onClose}
          />
        </header>
        <div className="flex items-center gap-1 border-b border-app-border p-2">
          <AppSelect
            aria-label={t("session")}
            value={props.activeSessionId ?? ""}
            options={props.sessionOptions}
            disabled={props.running || props.sessionOptions.length === 0}
            containerClassName="min-w-0 flex-1"
            onValueChange={props.onSelectSession}
          />
          <AppIconButton
            icon={MessageSquarePlus}
            label={t("newSession")}
            size="sm"
            disabled={props.running || !props.defaultModelId || props.creating}
            onClick={props.onCreateSession}
          />
          <AppIconButton
            icon={Trash2}
            label={t("deleteSession")}
            size="sm"
            variant="danger"
            disabled={props.running || !props.activeSessionId || props.deleting}
            onClick={props.onDeleteSession}
          />
        </div>
        <DrawerConversation {...props} />
        {props.error ? (
          <p className="border-t border-rose-500/40 bg-rose-500/10 px-3 py-2 text-xs text-rose-100">
            {props.error}
          </p>
        ) : null}
        <footer className="shrink-0 border-t border-app-border p-3">
          <label className="sr-only" htmlFor="agent-message">
            {t("message")}
          </label>
          <textarea
            id="agent-message"
            value={props.message}
            rows={3}
            placeholder={t("messagePlaceholder")}
            disabled={props.running || !props.activeSessionId}
            className="w-full resize-none border border-app-border bg-app-surface p-3 text-sm text-app-text outline-none placeholder:text-app-muted focus:border-brand-400 disabled:opacity-60"
            onChange={changeMessage}
            onKeyDown={handleMessageKey}
          />
          <div className="mt-2 flex items-center justify-between gap-2">
            <span className="text-[10px] text-app-muted">{t("sendHint")}</span>
            {props.running ? (
              <AppButton variant="danger" onClick={props.onStop}>
                <Square aria-hidden="true" className="size-3.5 fill-current" />
                {t("stop")}
              </AppButton>
            ) : (
              <AppButton
                disabled={!props.message.trim() || !props.activeSessionId}
                onClick={props.onSend}
              >
                <Send aria-hidden="true" className="size-4" />
                {t("send")}
              </AppButton>
            )}
          </div>
        </footer>
      </div>
    </aside>
  );
}

function DrawerConversation(props: AgentDrawerViewProps) {
  const { t } = useTranslation("agent");
  if (!props.defaultModelId) {
    return (
      <div className="grid min-h-0 flex-1 place-items-center p-6 text-center">
        <div>
          <p className="text-sm font-semibold text-white">{t("modelRequired")}</p>
          <p className="mt-2 text-xs leading-5 text-app-muted">{t("modelRequiredDescription")}</p>
          <AppButton className="mt-4" onClick={props.onOpenSettings}>
            {t("openSettings")}
          </AppButton>
        </div>
      </div>
    );
  }
  if (!props.activeSessionId) {
    return (
      <div className="grid min-h-0 flex-1 place-items-center p-6 text-center">
        <AppButton onClick={props.onCreateSession}>{t("startSession")}</AppButton>
      </div>
    );
  }
  return (
    <AgentEventList
      events={props.events}
      liveEvents={props.liveEvents}
      pendingUserMessage={props.pendingUserMessage}
      running={props.running}
      onApproval={props.onApproval}
      onUndo={props.onUndo}
    />
  );
}
