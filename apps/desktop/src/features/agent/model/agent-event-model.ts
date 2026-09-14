import type { AgentEventDto, AgentTurnEventDto } from "@/types";

export type DisplayAgentEvent =
  | { id: string; kind: "user"; content: string }
  | { id: string; kind: "assistant"; content: string; interrupted?: boolean }
  | { id: string; kind: "warning"; content: string }
  | {
      id: string;
      kind: "tool";
      name: string;
      details: string;
      state: "requested" | "running" | "succeeded" | "failed";
      actionId?: string;
    }
  | {
      id: string;
      kind: "approval";
      approvalId: string;
      name: string;
      details: string;
    }
  | { id: string; kind: "generation" };

export function buildDisplayAgentEvents(input: {
  events: AgentEventDto[];
  liveEvents: AgentTurnEventDto[];
  pendingUserMessage: string | null;
  running: boolean;
}): DisplayAgentEvent[] {
  const persisted = input.events.map(mapPersistedEvent);
  if (!input.running) return persisted;
  const live: DisplayAgentEvent[] = input.pendingUserMessage
    ? [{ id: "live-user", kind: "user", content: input.pendingUserMessage }]
    : [];
  let assistantText = "";
  const resolvedApprovals = new Set(
    input.liveEvents
      .filter((event) => event.kind === "approval_resolved")
      .map((event) => event.approval_id),
  );
  for (const [index, event] of input.liveEvents.entries()) {
    if (event.kind === "assistant_text_delta") {
      assistantText += event.text;
      continue;
    }
    if (assistantText) {
      live.push({ id: `live-assistant-${index}`, kind: "assistant", content: assistantText });
      assistantText = "";
    }
    const mapped = mapLiveEvent(event, index, resolvedApprovals);
    if (mapped) live.push(mapped);
  }
  if (assistantText) {
    live.push({ id: "live-assistant-tail", kind: "assistant", content: assistantText });
  }
  return [...persisted, ...live];
}

export function prettyAgentJson(value: string): string {
  try {
    return JSON.stringify(JSON.parse(value), null, 2);
  } catch {
    return value;
  }
}

export function humanizeAgentToolName(value: string): string {
  return value.replaceAll("_", " ");
}

function mapPersistedEvent(value: AgentEventDto): DisplayAgentEvent {
  const event = value.event;
  switch (event.kind) {
    case "user_message":
      return { id: value.id, kind: "user", content: event.content };
    case "assistant_message":
      return {
        id: value.id,
        kind: "assistant",
        content: event.content,
        interrupted: event.interrupted,
      };
    case "warning":
      return { id: value.id, kind: "warning", content: event.content };
    case "tool_call":
      return {
        id: value.id,
        kind: "tool",
        name: event.tool_name,
        details: event.arguments_json,
        state: "requested",
      };
    case "tool_result":
      return {
        id: value.id,
        kind: "tool",
        name: event.tool_name,
        details: event.result_json,
        state: event.failed ? "failed" : "succeeded",
        actionId: event.failed ? undefined : readActionId(event.result_json),
      };
    case "approval":
      return {
        id: value.id,
        kind: "warning",
        content: `${humanizeAgentToolName(event.tool_name)} · ${event.approved ? "approved" : "denied"}`,
      };
  }
}

function mapLiveEvent(
  event: AgentTurnEventDto,
  index: number,
  resolvedApprovals: ReadonlySet<string>,
): DisplayAgentEvent | null {
  if (event.kind === "tool_started") {
    return {
      id: `live-tool-start-${index}`,
      kind: "tool",
      name: event.name,
      details: event.arguments_json,
      state: "running",
    };
  }
  if (event.kind === "tool_finished") {
    return {
      id: `live-tool-finish-${index}`,
      kind: "tool",
      name: event.name,
      details: event.result_json,
      state: event.failed ? "failed" : "succeeded",
      actionId: event.failed ? undefined : readActionId(event.result_json),
    };
  }
  if (event.kind === "approval_requested") {
    if (resolvedApprovals.has(event.approval_id)) return null;
    return {
      id: `live-approval-${event.approval_id}`,
      kind: "approval",
      approvalId: event.approval_id,
      name: event.name,
      details: event.arguments_json,
    };
  }
  if (event.kind === "generation_submitted") {
    return { id: `live-generation-${index}`, kind: "generation" };
  }
  return null;
}

function readActionId(value: string): string | undefined {
  try {
    const parsed: unknown = JSON.parse(value);
    if (typeof parsed === "object" && parsed && "action_id" in parsed) {
      const actionId = parsed.action_id;
      return typeof actionId === "string" ? actionId : undefined;
    }
  } catch {
    return undefined;
  }
  return undefined;
}
