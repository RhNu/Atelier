import { buildDisplayAgentEvents } from "../features/agent/model/agent-event-model";
import type { AgentEventDto, AgentTurnEventDto } from "../types";

describe("Agent event presentation", () => {
  it("keeps a streamed assistant response together and hides resolved approvals", () => {
    const liveEvents: AgentTurnEventDto[] = [
      { kind: "assistant_text_delta", text: "First " },
      { kind: "assistant_text_delta", text: "draft" },
      {
        kind: "approval_requested",
        approval_id: "approval-1",
        name: "submit_generation",
        arguments_json: "{}",
      },
      { kind: "approval_resolved", approval_id: "approval-1", approved: true },
    ];

    expect(
      buildDisplayAgentEvents({
        events: [],
        liveEvents,
        pendingUserMessage: "Create a portrait",
        running: true,
      }),
    ).toEqual([
      { id: "live-user", kind: "user", content: "Create a portrait" },
      { id: "live-assistant-0", kind: "assistant", content: "First draft" },
    ]);
  });

  it("extracts an undo action only from a successful tool result", () => {
    const events: AgentEventDto[] = [
      event("success", {
        kind: "tool_result",
        tool_name: "edit_generation_draft",
        result_json: '{"action_id":"action-1"}',
        failed: false,
      }),
      event("failure", {
        kind: "tool_result",
        tool_name: "edit_generation_draft",
        result_json: '{"action_id":"action-2"}',
        failed: true,
      }),
    ];

    const displayed = buildDisplayAgentEvents({
      events,
      liveEvents: [],
      pendingUserMessage: null,
      running: false,
    });

    expect(displayed[0]).toMatchObject({ actionId: "action-1", state: "succeeded" });
    expect(displayed[1]).toMatchObject({ actionId: undefined, state: "failed" });
  });

  it("renders a tool start and finish as one updated item", () => {
    const displayed = buildDisplayAgentEvents({
      events: [],
      liveEvents: [
        {
          kind: "tool_started",
          name: "edit_generation_draft",
          arguments_json: '{"operations":[]}',
        },
        {
          kind: "tool_finished",
          name: "edit_generation_draft",
          result_json: '{"action_id":"action-1"}',
          failed: false,
        },
      ],
      pendingUserMessage: null,
      running: true,
    });

    expect(displayed).toEqual([
      {
        id: "live-tool-0",
        kind: "tool",
        name: "edit_generation_draft",
        arguments: '{"operations":[]}',
        result: '{"action_id":"action-1"}',
        state: "succeeded",
        actionId: "action-1",
      },
    ]);
  });

  it("merges persisted tool calls with their results", () => {
    const displayed = buildDisplayAgentEvents({
      events: [
        event("call", {
          kind: "tool_call",
          tool_name: "get_generation_context",
          arguments_json: "{}",
        }),
        event("result", {
          kind: "tool_result",
          tool_name: "get_generation_context",
          result_json: '{"draft":{}}',
          failed: false,
        }),
      ],
      liveEvents: [],
      pendingUserMessage: null,
      running: false,
    });

    expect(displayed).toHaveLength(1);
    expect(displayed[0]).toMatchObject({
      id: "call",
      arguments: "{}",
      result: '{"draft":{}}',
      state: "succeeded",
    });
  });
});

function event(id: string, kind: AgentEventDto["event"]): AgentEventDto {
  return { id, session_id: "session-1", sequence: 1, created_at_ms: 1, event: kind };
}
