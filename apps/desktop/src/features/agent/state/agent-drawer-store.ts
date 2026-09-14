import { create } from "zustand";

import type { AgentTurnEventDto } from "@/types";

type AgentDrawerState = {
  open: boolean;
  width: number;
  activeSessionId: string | null;
  runningSessionId: string | null;
  pendingUserMessage: string | null;
  liveEvents: AgentTurnEventDto[];
  contextInputTokens: number | null;
  error: string | null;
  setOpen: (open: boolean) => void;
  toggle: () => void;
  setWidth: (width: number) => void;
  selectSession: (sessionId: string | null) => void;
  beginTurn: (sessionId: string, message: string) => void;
  pushEvent: (event: AgentTurnEventDto) => void;
  finishTurn: (error?: string) => void;
  resetWorkspace: () => void;
};

const MIN_DRAWER_WIDTH = 360;
const MAX_DRAWER_WIDTH = 760;

export const useAgentDrawerStore = create<AgentDrawerState>((set) => ({
  open: false,
  width: 460,
  activeSessionId: null,
  runningSessionId: null,
  pendingUserMessage: null,
  liveEvents: [],
  contextInputTokens: null,
  error: null,
  setOpen: (open) => set({ open }),
  toggle: () => set((state) => ({ open: !state.open })),
  setWidth: (width) =>
    set({ width: Math.min(MAX_DRAWER_WIDTH, Math.max(MIN_DRAWER_WIDTH, width)) }),
  selectSession: (activeSessionId) =>
    set({ activeSessionId, liveEvents: [], contextInputTokens: null, error: null }),
  beginTurn: (runningSessionId, pendingUserMessage) =>
    set({ runningSessionId, pendingUserMessage, liveEvents: [], error: null }),
  pushEvent: (event) =>
    set((state) => ({
      liveEvents: [...state.liveEvents, event],
      contextInputTokens: event.kind === "usage" ? event.input_tokens : state.contextInputTokens,
    })),
  finishTurn: (error) =>
    set({ runningSessionId: null, pendingUserMessage: null, liveEvents: [], error: error ?? null }),
  resetWorkspace: () =>
    set({
      activeSessionId: null,
      runningSessionId: null,
      pendingUserMessage: null,
      liveEvents: [],
      contextInputTokens: null,
      error: null,
    }),
}));

export function isAgentTurnRunning(): boolean {
  return useAgentDrawerStore.getState().runningSessionId !== null;
}
