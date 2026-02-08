/**
 * FocusManager - Centralized window focus state management
 *
 * Provides a single source of truth for:
 * - Window focus state (whether the app is in the foreground)
 * - Activity tracking (last user activity timestamp)
 * - Focus operations using native platform APIs
 */

import { invoke } from "@tauri-apps/api/core";
import { Window } from "@tauri-apps/api/window";
import { IFocusManager, FocusState } from "../types/kernel";

type FocusListener = (state: FocusState) => void;

export class FocusManager implements IFocusManager {
  private _state: FocusState = {
    windowFocused: true,
    lastActivityTime: Date.now(),
  };

  private listeners: Set<FocusListener> = new Set();
  private unsubFocusChanged: (() => void) | null = null;
  private activityHandler: (() => void) | null = null;
  private keydownHandler: ((e: KeyboardEvent) => void) | null = null;
  private destroyed = false;

  get state(): FocusState {
    return { ...this._state };
  }

  get isWindowFocused(): boolean {
    return this._state.windowFocused;
  }

  get lastActivityTime(): number {
    return this._state.lastActivityTime;
  }

  get inactivitySeconds(): number {
    return (Date.now() - this._state.lastActivityTime) / 1000;
  }

  async init(): Promise<void> {
    // Get initial focus state
    try {
      const appWindow = Window.getCurrent();
      this._state.windowFocused = await appWindow.isFocused();
    } catch {
      this._state.windowFocused = true;
    }

    // Listen for focus changes
    const appWindow = Window.getCurrent();
    this.unsubFocusChanged = await appWindow.onFocusChanged(({ payload }) => {
      this.updateState({ windowFocused: payload });
    });

    // Track activity from terminal and keyboard
    this.activityHandler = () => this.recordActivity();
    this.keydownHandler = () => this.recordActivity();

    window.addEventListener("terminal:activity", this.activityHandler);
    window.addEventListener("keydown", this.keydownHandler);
  }

  /**
   * Record user activity, resetting the inactivity timer.
   */
  recordActivity(): void {
    this.updateState({ lastActivityTime: Date.now() });
  }

  /**
   * Focus the application window using native platform APIs.
   * This bypasses Tauri's buggy setFocus() on macOS.
   */
  async focusWindow(): Promise<void> {
    try {
      // Use our native platform-specific implementation
      await invoke("focus_window");
    } catch (err) {
      console.warn("[FocusManager] Native focus failed, trying Tauri fallback:", err);
      // Fallback to Tauri APIs
      try {
        const appWindow = Window.getCurrent();
        await appWindow.unminimize();
        await appWindow.show();
        await appWindow.setFocus();
      } catch (fallbackErr) {
        console.warn("[FocusManager] Tauri fallback also failed:", fallbackErr);
      }
    }
  }

  /**
   * Request user attention (dock bounce on macOS, taskbar flash on Windows).
   */
  async requestAttention(critical = false): Promise<void> {
    try {
      await invoke("request_attention", { critical });
    } catch (err) {
      console.warn("[FocusManager] Native attention request failed, trying Tauri fallback:", err);
      // Fallback to Tauri
      try {
        const appWindow = Window.getCurrent();
        await appWindow.requestUserAttention(critical ? 2 : 1);
      } catch {
        // Ignore fallback failures
      }
    }
  }

  /**
   * Check if the app is currently active/frontmost.
   */
  async isAppActive(): Promise<boolean> {
    try {
      return await invoke<boolean>("is_app_active");
    } catch {
      // Fallback to our tracked state
      return this._state.windowFocused;
    }
  }

  /**
   * Subscribe to focus state changes.
   */
  subscribe(listener: FocusListener): () => void {
    this.listeners.add(listener);
    // Immediately notify with current state
    listener(this.state);
    return () => {
      this.listeners.delete(listener);
    };
  }

  /**
   * Clean up resources.
   */
  destroy(): void {
    if (this.destroyed) return;
    this.destroyed = true;

    if (this.unsubFocusChanged) {
      this.unsubFocusChanged();
      this.unsubFocusChanged = null;
    }

    if (this.activityHandler) {
      window.removeEventListener("terminal:activity", this.activityHandler);
      this.activityHandler = null;
    }

    if (this.keydownHandler) {
      window.removeEventListener("keydown", this.keydownHandler);
      this.keydownHandler = null;
    }

    this.listeners.clear();
  }

  private updateState(partial: Partial<FocusState>): void {
    const prev = this._state;
    this._state = { ...prev, ...partial };

    // Only notify if something actually changed
    if (
      prev.windowFocused !== this._state.windowFocused ||
      prev.lastActivityTime !== this._state.lastActivityTime
    ) {
      this.notifyListeners();
    }
  }

  private notifyListeners(): void {
    const state = this.state;
    for (const listener of this.listeners) {
      try {
        listener(state);
      } catch (err) {
        console.error("[FocusManager] Listener error:", err);
      }
    }
  }
}
