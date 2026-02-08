export const SLOTS = {
  TAB_BAR_LEFT: "TabBarLeft",
  TAB_BAR_CENTER: "TabBarCenter",
  TAB_BAR_RIGHT: "TabBarRight",
  MAIN_CONTENT: "MainContent",
  STATUS_BAR_LEFT: "StatusBarLeft",
  STATUS_BAR_CENTER: "StatusBarCenter",
  STATUS_BAR_RIGHT: "StatusBarRight",
  TERMINAL_OVERLAY: "TerminalOverlay",
  OVERLAY: "Overlay",
  SIDE_PANEL: "SidePanel",
} as const;

export type SlotName = (typeof SLOTS)[keyof typeof SLOTS];
