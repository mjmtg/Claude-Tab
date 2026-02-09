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
  // Session lifecycle
  SESSION_CREATED: "session.created",
  SESSION_CLOSED: "session.closed",
  SESSION_STATE_CHANGED: "session.state_changed",
  SESSION_RENAMED: "session.renamed",
  SESSION_METADATA_CHANGED: "session.metadata_changed",
  SESSION_ACTIVE_CHANGED: "session.active_changed",
  // PTY
  PTY_OUTPUT: "pty.output",
  PTY_EXIT: "pty.exit",
  PTY_RESIZED: "pty.resized",
  // Config
  CONFIG_CHANGED: "config.changed",
  CONFIG_RELOADED: "config.reloaded",
  // Extension
  EXTENSION_ACTIVATED: "extension.activated",
  EXTENSION_DEACTIVATED: "extension.deactivated",
  EXTENSION_ACTIVATION_FAILED: "extension.activation_failed",
  // Hooks
  HOOK_RECEIVED: "hook.received",
  HOOK_PERMISSION_NEEDED: "hook.permission_needed",
  // Profile
  PROFILE_SAVED: "profile.saved",
  PROFILE_DELETED: "profile.deleted",
  PROFILE_LAUNCHED: "profile.launched",
  // System
  SYSTEM_STARTUP: "system.startup",
  SYSTEM_SHUTDOWN: "system.shutdown",
  SYSTEM_ERROR: "system.error",
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
