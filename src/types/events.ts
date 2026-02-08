export interface CoreEvent {
  topic: string;
  payload: Record<string, unknown>;
  session_id?: string;
}

export interface PtyOutputEvent {
  session_id: string;
  data: string;  // Base64 encoded
  encoding: "base64";
}

export interface PtyExitEvent {
  session_id: string;
}

export type EventHandler = (event: CoreEvent) => void;
export type UnsubscribeFn = () => void;

export const CORE_EVENTS = {
  SESSION_CREATED: "session.created",
  SESSION_CLOSED: "session.closed",
  SESSION_STATE_CHANGED: "session.state_changed",
  PTY_OUTPUT: "pty.output",
  PTY_EXIT: "pty.exit",
  DETECTION_TRIGGERED: "detection.triggered",
  CONFIG_CHANGED: "config.changed",
  EXTENSION_ACTIVATED: "extension.activated",
} as const;

/**
 * UI Events for cross-extension communication.
 * Used by frontend extensions to coordinate UI state.
 */
export const UI_EVENTS = {
  /** Toggle settings panel visibility */
  SETTINGS_TOGGLE: "ui.settings.toggle",

  /** Toggle profiles panel visibility */
  PROFILES_TOGGLE: "ui.profiles.toggle",

  /** Toggle sidebar visibility */
  SIDEBAR_TOGGLE: "ui.sidebar.toggle",

  /** Toggle command palette visibility */
  COMMAND_PALETTE_TOGGLE: "ui.command-palette.toggle",

  /** Request focus on terminal */
  TERMINAL_FOCUS: "ui.terminal.focus",

  /** Session tab clicked */
  TAB_CLICKED: "ui.tab.clicked",

  /** Session tab context menu requested */
  TAB_CONTEXT_MENU: "ui.tab.context-menu",
} as const;
