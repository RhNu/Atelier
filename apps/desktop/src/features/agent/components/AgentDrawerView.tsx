/* eslint-disable max-lines, max-lines-per-function, react-perf/jsx-no-new-function-as-prop, react-perf/jsx-no-new-object-as-prop, react-perf/jsx-no-new-array-as-prop */
import {
  Bot,
  MessageSquarePlus,
  MessagesSquare,
  PanelRightClose,
  Send,
  Square,
  Trash2,
} from "lucide-react";
import {
  useEffect,
  useRef,
  useState,
  type ChangeEvent,
  type KeyboardEvent,
  type PointerEvent,
} from "react";
import { useTranslation } from "react-i18next";

import { AppButton, AppIconButton, AppSelect, type SelectOption } from "@/components/ui";
import type { AgentEventDto, AgentPermissionModeDto, AgentTurnEventDto } from "@/types";

import { AgentEventList } from "./AgentEventList";

type AgentDrawerViewProps = {
  open: boolean;
  width: number;
  activeSessionId: string | null;
  activeSessionTitle: string;
  contextWindow: number | null;
  contextInputTokens: number | null;
  permissionMode: AgentPermissionModeDto;
  updatingPermission: boolean;
  defaultModelId: string | null;
  running: boolean;
  creating: boolean;
  deleting: boolean;
  renaming: boolean;
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
  onRenameSession: (title: string) => void;
  onPermissionModeChange: (mode: AgentPermissionModeDto) => void;
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
  const changeSession = (event: ChangeEvent<HTMLSelectElement>) =>
    props.onSelectSession(event.target.value);
  const changePermission = (value: string) =>
    props.onPermissionModeChange(parsePermissionMode(value));
  const contextUsage = formatContextUsage(props.contextInputTokens, props.contextWindow);

  return (
    <aside
      aria-label={t("drawerLabel")}
      aria-hidden={!props.open}
      inert={!props.open ? true : undefined}
      className={[
        "absolute inset-y-0 right-0 z-30 min-h-0 border-l border-app-border bg-app-bg shadow-2xl transition-[width]",
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
        <header className="flex h-12 shrink-0 items-center gap-2 border-b border-app-border px-3">
          <Bot aria-hidden="true" className="size-4 shrink-0 text-brand-200" />
          <p className="shrink-0 text-sm font-semibold text-white">{t("title")}</p>
          <span aria-hidden="true" className="h-4 w-px bg-app-border" />
          <SessionTitleEditor
            key={`${props.activeSessionId ?? "none"}-${props.activeSessionTitle}`}
            title={props.activeSessionTitle}
            disabled={!props.activeSessionId || props.running || props.renaming}
            onRename={props.onRenameSession}
          />
          <AppIconButton
            icon={PanelRightClose}
            label={t("close")}
            size="sm"
            onClick={props.onClose}
          />
        </header>
        <div className="flex h-10 shrink-0 items-center justify-end gap-1 border-b border-app-border px-2">
          <label
            className="relative inline-flex size-8 items-center justify-center text-app-muted hover:bg-app-surface hover:text-app-text"
            title={t("switchSession")}
          >
            <span className="sr-only">{t("switchSession")}</span>
            <MessagesSquare aria-hidden="true" className="size-4" />
            <select
              aria-label={t("switchSession")}
              value={props.activeSessionId ?? ""}
              disabled={props.running || props.sessionOptions.length === 0}
              className="absolute inset-0 cursor-pointer opacity-0 disabled:cursor-not-allowed"
              onChange={changeSession}
            >
              {props.sessionOptions.map((option) => (
                <option key={option.value} value={option.value}>
                  {option.label}
                </option>
              ))}
            </select>
          </label>
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
        <footer className="shrink-0 border-t border-app-border p-2.5">
          <label className="sr-only" htmlFor="agent-message">
            {t("message")}
          </label>
          <textarea
            id="agent-message"
            value={props.message}
            rows={3}
            placeholder={t("messagePlaceholder")}
            disabled={props.running || !props.activeSessionId}
            className="w-full resize-none border border-app-border bg-app-surface p-2.5 text-sm text-app-text outline-none placeholder:text-app-muted focus:border-brand-400 disabled:opacity-60"
            onChange={changeMessage}
            onKeyDown={handleMessageKey}
          />
          <div className="mt-1.5 flex items-center justify-end gap-1.5">
            <span
              className="mr-auto truncate text-[10px] text-app-muted tabular-nums"
              title={t("contextUsage")}
            >
              {props.contextInputTokens === null
                ? t("contextUsageEmpty", { limit: contextUsage.limit })
                : t("contextUsageValue", contextUsage)}
            </span>
            <AppSelect
              aria-label={t("permissionMode")}
              value={props.permissionMode}
              options={[
                { value: "standard", label: t("permissionStandard") },
                { value: "ask", label: t("permissionAsk") },
                { value: "bypass_all", label: t("permissionBypass") },
              ]}
              disabled={props.running || props.updatingPermission}
              className="h-8 px-2 pr-6 text-xs"
              containerClassName="w-32 shrink-0"
              onValueChange={changePermission}
            />
            {props.running ? (
              <AppIconButton
                icon={Square}
                label={t("stop")}
                size="sm"
                variant="danger"
                onClick={props.onStop}
              />
            ) : (
              <AppIconButton
                icon={Send}
                label={t("send")}
                size="sm"
                disabled={!props.message.trim() || !props.activeSessionId}
                onClick={props.onSend}
              />
            )}
          </div>
        </footer>
      </div>
    </aside>
  );
}

function SessionTitleEditor({
  title,
  disabled,
  onRename,
}: {
  title: string;
  disabled: boolean;
  onRename: (title: string) => void;
}) {
  const { t } = useTranslation("agent");
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(title);
  const inputRef = useRef<HTMLInputElement>(null);
  useEffect(() => {
    if (editing) inputRef.current?.focus();
  }, [editing]);

  const commit = () => {
    const nextTitle = draft.trim();
    setEditing(false);
    if (nextTitle && nextTitle !== title) onRename(nextTitle);
    else setDraft(title);
  };
  const handleKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === "Enter") commit();
    if (event.key === "Escape") {
      setDraft(title);
      setEditing(false);
    }
  };

  if (editing) {
    return (
      <input
        ref={inputRef}
        aria-label={t("editSessionTitle")}
        value={draft}
        className="h-7 min-w-0 flex-1 border border-brand-400 bg-black/20 px-2 text-xs text-app-text outline-none"
        onBlur={commit}
        onChange={(event) => setDraft(event.target.value)}
        onKeyDown={handleKeyDown}
      />
    );
  }
  return (
    <button
      type="button"
      title={t("editSessionTitle")}
      disabled={disabled}
      className="min-w-0 flex-1 truncate text-left text-xs text-app-muted hover:text-white disabled:cursor-default disabled:hover:text-app-muted"
      onClick={() => setEditing(true)}
    >
      {title || t("newSessionTitle")}
    </button>
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

function formatContextUsage(used: number | null, limit: number | null) {
  return {
    used: used === null ? "—" : compactNumber(used),
    limit: limit === null ? "—" : compactNumber(limit),
  };
}

function compactNumber(value: number): string {
  if (value < 1_000) return value.toLocaleString();
  const digits = value < 10_000 ? 1 : 0;
  return `${(value / 1_000).toFixed(digits)}k`;
}

function parsePermissionMode(value: string): AgentPermissionModeDto {
  if (value === "ask") return "ask";
  if (value === "bypass_all") return "bypass_all";
  return "standard";
}
