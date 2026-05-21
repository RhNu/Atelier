import { create } from "zustand";

export type RouteRestoreState = {
  lastRoute: string;
  setLastRoute: (route: string) => void;
};

export type ShellPreferencesState = {
  compactNav: boolean;
  rightInspectorCollapsed: boolean;
  historyPanelCollapsed: boolean;
  setCompactNav: (compactNav: boolean) => void;
  setRightInspectorCollapsed: (collapsed: boolean) => void;
  setHistoryPanelCollapsed: (collapsed: boolean) => void;
};

export const useRouteRestoreStore = create<RouteRestoreState>((set) => ({
  lastRoute: "/generate",
  setLastRoute: (lastRoute) => set({ lastRoute }),
}));

export const useShellPreferencesStore = create<ShellPreferencesState>((set) => ({
  compactNav: true,
  rightInspectorCollapsed: false,
  historyPanelCollapsed: false,
  setCompactNav: (compactNav) => set({ compactNav }),
  setRightInspectorCollapsed: (rightInspectorCollapsed) => set({ rightInspectorCollapsed }),
  setHistoryPanelCollapsed: (historyPanelCollapsed) => set({ historyPanelCollapsed }),
}));
