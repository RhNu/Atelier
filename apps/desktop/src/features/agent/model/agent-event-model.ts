import type { AgentEventDto, AgentTurnEventDto } from "@/types";

export type DisplayAgentEvent =
  | { id: string; kind: "user"; content: string }
  | { id: string; kind: "assistant"; content: string; interrupted?: boolean }
  | { id: string; kind: "warning"; content: string }
  | {
      id: string;
      kind: "tool";
      name: string;
      arguments: string | null;
      result: string | null;
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
  | { id: string; kind: "approval_status"; name: string; approved: boolean }
  | { id: string; kind: "generation" };

export type AgentToolNameKey =
  | "toolNames.getGenerationContext"
  | "toolNames.editGenerationDraft"
  | "toolNames.searchPromptResources"
  | "toolNames.getPromptResource"
  | "toolNames.getPromptLibrary"
  | "toolNames.createPromptResource"
  | "toolNames.editPromptResource"
  | "toolNames.copyPromptResource"
  | "toolNames.deletePromptResource"
  | "toolNames.getLexiconContext"
  | "toolNames.searchLexicon"
  | "toolNames.getLexiconEntity"
  | "toolNames.previewGeneration"
  | "toolNames.submitGeneration"
  | "toolNames.undoAgentAction";

export const AGENT_TOOL_NAME_KEYS: Readonly<Record<string, AgentToolNameKey>> = {
  get_generation_context: "toolNames.getGenerationContext",
  edit_generation_draft: "toolNames.editGenerationDraft",
  search_prompt_resources: "toolNames.searchPromptResources",
  get_prompt_resource: "toolNames.getPromptResource",
  get_prompt_library: "toolNames.getPromptLibrary",
  create_prompt_resource: "toolNames.createPromptResource",
  edit_prompt_resource: "toolNames.editPromptResource",
  copy_prompt_resource: "toolNames.copyPromptResource",
  delete_prompt_resource: "toolNames.deletePromptResource",
  get_lexicon_context: "toolNames.getLexiconContext",
  search_lexicon: "toolNames.searchLexicon",
  get_lexicon_entity: "toolNames.getLexiconEntity",
  preview_generation: "toolNames.previewGeneration",
  submit_generation: "toolNames.submitGeneration",
  undo_agent_action: "toolNames.undoAgentAction",
};

export function buildDisplayAgentEvents(input: {
  events: AgentEventDto[];
  liveEvents: AgentTurnEventDto[];
  pendingUserMessage: string | null;
  running: boolean;
}): DisplayAgentEvent[] {
  const persisted = projectPersistedEvents(input.events);
  if (!input.running) return persisted;
  return [...persisted, ...projectLiveEvents(input.liveEvents, input.pendingUserMessage)];
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

function projectPersistedEvents(events: AgentEventDto[]): DisplayAgentEvent[] {
  const projected: DisplayAgentEvent[] = [];
  const pendingTools = new Map<string, number[]>();
  for (const value of events) {
    const event = value.event;
    if (event.kind === "tool_call") {
      const index = projected.length;
      projected.push({
        id: value.id,
        kind: "tool",
        name: event.tool_name,
        arguments: event.arguments_json,
        result: null,
        state: "requested",
      });
      const pending = pendingTools.get(event.tool_name) ?? [];
      pending.push(index);
      pendingTools.set(event.tool_name, pending);
      continue;
    }
    if (event.kind === "tool_result") {
      const pending = pendingTools.get(event.tool_name);
      const index = pending?.shift();
      const result = {
        result: event.result_json,
        state: event.failed ? ("failed" as const) : ("succeeded" as const),
        actionId: event.failed ? undefined : readActionId(event.result_json),
      };
      if (index === undefined) {
        projected.push({
          id: value.id,
          kind: "tool",
          name: event.tool_name,
          arguments: null,
          ...result,
        });
      } else {
        const tool = projected[index];
        if (tool.kind === "tool") projected[index] = { ...tool, ...result };
      }
      continue;
    }
    projected.push(mapPersistedNonToolEvent(value));
  }
  return projected;
}

function mapPersistedNonToolEvent(value: AgentEventDto): DisplayAgentEvent {
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
    case "approval":
      return {
        id: value.id,
        kind: "approval_status",
        name: event.tool_name,
        approved: event.approved,
      };
    case "tool_call":
    case "tool_result":
      throw new Error("tool events are projected as a pair");
  }
}

function projectLiveEvents(
  events: AgentTurnEventDto[],
  pendingUserMessage: string | null,
): DisplayAgentEvent[] {
  const projected: DisplayAgentEvent[] = pendingUserMessage
    ? [{ id: "live-user", kind: "user", content: pendingUserMessage }]
    : [];
  const resolvedApprovals = new Set(
    events.filter((event) => event.kind === "approval_resolved").map((event) => event.approval_id),
  );
  const pendingTools = new Map<string, number[]>();
  let assistantText = "";
  let assistantSegment = 0;

  const flushAssistant = () => {
    if (!assistantText) return;
    projected.push({
      id: `live-assistant-${assistantSegment}`,
      kind: "assistant",
      content: assistantText,
    });
    assistantSegment += 1;
    assistantText = "";
  };

  for (const [index, event] of events.entries()) {
    if (event.kind === "assistant_text_delta") {
      assistantText += event.text;
      continue;
    }
    if (event.kind === "tool_started") {
      flushAssistant();
      const projectedIndex = projected.length;
      projected.push({
        id: `live-tool-${index}`,
        kind: "tool",
        name: event.name,
        arguments: event.arguments_json,
        result: null,
        state: "running",
      });
      const pending = pendingTools.get(event.name) ?? [];
      pending.push(projectedIndex);
      pendingTools.set(event.name, pending);
      continue;
    }
    if (event.kind === "tool_finished") {
      const pending = pendingTools.get(event.name);
      const projectedIndex = pending?.shift();
      const result = {
        result: event.result_json,
        state: event.failed ? ("failed" as const) : ("succeeded" as const),
        actionId: event.failed ? undefined : readActionId(event.result_json),
      };
      if (projectedIndex === undefined) {
        flushAssistant();
        projected.push({
          id: `live-tool-${index}`,
          kind: "tool",
          name: event.name,
          arguments: null,
          ...result,
        });
      } else {
        const tool = projected[projectedIndex];
        if (tool.kind === "tool") projected[projectedIndex] = { ...tool, ...result };
      }
      continue;
    }
    if (event.kind === "approval_requested" && !resolvedApprovals.has(event.approval_id)) {
      flushAssistant();
      projected.push({
        id: `live-approval-${event.approval_id}`,
        kind: "approval",
        approvalId: event.approval_id,
        name: event.name,
        details: event.arguments_json,
      });
      continue;
    }
    if (event.kind === "generation_submitted") {
      flushAssistant();
      projected.push({ id: `live-generation-${index}`, kind: "generation" });
    }
  }
  flushAssistant();
  return projected;
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
