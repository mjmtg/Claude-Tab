import { FrontendExtension, ExtensionContext } from "../../types/extension";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import type { IFocusManager } from "../../types/kernel";

/**
 * Window Focus Extension
 *
 * Brings the app window to the front and requests user attention
 * when a session transitions to "your_turn" state.
 *
 * Uses the centralized FocusManager for all focus operations,
 * which provides native platform-specific focus (bypassing Tauri's buggy APIs on macOS).
 */

let focusManager: IFocusManager | null = null;
let unsubCoreEvent: (() => void) | null = null;

export function createWindowFocusExtension(): FrontendExtension {
  return {
    manifest: {
      id: "window-focus",
      name: "Window Focus",
      version: "0.1.0",
      description: "Auto-focus window when session needs attention",
    },

    async activate(ctx: ExtensionContext) {
      focusManager = ctx.focusManager;

      // Listen for session state changes
      unsubCoreEvent = await listen<{ topic: string; payload: Record<string, unknown> }>(
        "core-event",
        async (e) => {
          const { topic, payload } = e.payload;

          if (topic === "session.state_changed") {
            const toState = payload.to as string;

            // Only act on transitions TO your_turn state
            if (toState === "your_turn") {
              await handleYourTurn();
            }
          }
        }
      );

      // Sync config to localStorage for immediate access
      syncConfigToLocalStorage();
    },

    deactivate() {
      if (unsubCoreEvent) {
        unsubCoreEvent();
        unsubCoreEvent = null;
      }
      focusManager = null;
    },
  };
}

async function handleYourTurn() {
  if (!focusManager) return;

  // Check if window auto-focus is enabled
  const enabled = getConfigValue("autoFocus.windowBringToFront", true);
  if (!enabled) return;

  // Only act if window is not focused
  if (focusManager.isWindowFocused) return;

  try {
    // Request user attention (bounces dock icon on macOS, flashes taskbar on Windows)
    await focusManager.requestAttention(true);

    // Bring window to front using native platform APIs
    await focusManager.focusWindow();

    // Aggressive mode: temporarily set always-on-top to force focus
    // This is a fallback in case native focus doesn't work
    const aggressiveMode = getConfigValue("autoFocus.aggressiveMode", false);
    if (aggressiveMode) {
      try {
        const { Window } = await import("@tauri-apps/api/window");
        const appWindow = Window.getCurrent();
        await appWindow.setAlwaysOnTop(true);
        // Small delay then disable to avoid staying on top
        setTimeout(async () => {
          try {
            await appWindow.setAlwaysOnTop(false);
          } catch {
            // Ignore
          }
        }, 100);
      } catch {
        // Ignore aggressive mode failures
      }
    }
  } catch (err) {
    console.warn("[window-focus] Failed to focus window:", err);
  }
}

function getConfigValue<T>(key: string, defaultValue: T): T {
  const stored = localStorage.getItem(`config.${key}`);
  if (stored !== null) {
    try {
      return JSON.parse(stored) as T;
    } catch {
      // fall through
    }
  }
  return defaultValue;
}

async function syncConfigToLocalStorage() {
  const keys = [
    "autoFocus.windowBringToFront",
    "autoFocus.aggressiveMode",
    "autoFocus.tabAutoSwitch",
    "autoFocus.inactivitySeconds",
    "autoFocus.countdownSeconds",
  ];

  for (const key of keys) {
    try {
      const value = await invoke<unknown | null>("get_config_value", { key });
      if (value !== null) {
        localStorage.setItem(`config.${key}`, JSON.stringify(value));
      }
    } catch {
      // Ignore errors
    }
  }

  // Listen for config changes and update localStorage
  window.addEventListener("config-changed", ((e: CustomEvent<{ key: string; value: unknown }>) => {
    if (e.detail && e.detail.key && keys.includes(e.detail.key)) {
      localStorage.setItem(`config.${e.detail.key}`, JSON.stringify(e.detail.value));
    }
  }) as EventListener);
}
