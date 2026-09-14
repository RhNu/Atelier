import { useAgentDrawerStore } from "../features/agent/state/agent-drawer-store";

describe("Agent drawer state", () => {
  beforeEach(() => {
    useAgentDrawerStore.setState({
      open: false,
      width: 460,
      activeSessionId: null,
      runningSessionId: null,
      pendingUserMessage: null,
      liveEvents: [],
      contextInputTokens: null,
      error: null,
    });
  });

  it("keeps an active turn running when the drawer is closed", () => {
    const state = useAgentDrawerStore.getState();
    state.beginTurn("session-1", "Improve the lighting");
    state.setOpen(false);

    expect(useAgentDrawerStore.getState()).toMatchObject({
      open: false,
      runningSessionId: "session-1",
      pendingUserMessage: "Improve the lighting",
    });
  });

  it("clamps the resizable drawer to supported bounds", () => {
    useAgentDrawerStore.getState().setWidth(100);
    expect(useAgentDrawerStore.getState().width).toBe(360);
    useAgentDrawerStore.getState().setWidth(1_000);
    expect(useAgentDrawerStore.getState().width).toBe(760);
  });

  it("keeps the latest streamed context usage after a turn finishes", () => {
    const state = useAgentDrawerStore.getState();
    state.beginTurn("session-1", "Improve the lighting");
    state.pushEvent({ kind: "usage", input_tokens: 1_240, output_tokens: 80 });
    state.finishTurn();

    expect(useAgentDrawerStore.getState().contextInputTokens).toBe(1_240);
  });
});
