/**
 * SessionStateManager - State machine for session switching
 *
 * Manages the inactivity-based tab switching flow with a state machine
 * to prevent race conditions. States:
 *
 *   idle → checking_inactivity → showing_toast → switching → idle
 *                                     ↓
 *                              cooldown (2s) → idle
 *
 * Features:
 * - Atomic state transitions
 * - Declined session tracking (won't re-offer until manual switch)
 * - Cooldown period after declining to prevent immediate re-trigger
 * - Single refresh lock to prevent concurrent session list fetches
 */

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { ISessionStateManager, SwitchState, ToastState } from "../types/kernel";
import type { IFocusManager } from "../types/kernel";
import type { SessionInfo } from "../types/session";

type StateChangeListener = (state: SwitchState, toast: ToastState | null) => void;

const COOLDOWN_MS = 2000;

export class SessionStateManager implements ISessionStateManager {
  private _state: SwitchState = "idle";
  private _toastState: ToastState | null = null;
  private _declinedSessionIds: Set<string> = new Set();
  private listeners: Set<StateChangeListener> = new Set();
  private checkInterval: ReturnType<typeof setInterval> | null = null;
  private cooldownTimeout: ReturnType<typeof setTimeout> | null = null;
  private refreshLock = false;
  private unsubCoreEvent: (() => void) | null = null;
  private destroyed = false;

  // Cached session data to prevent race conditions
  private cachedSessions: SessionInfo[] = [];
  private cachedActiveSessionId: string | null = null;

  // Configuration (injected via updateConfig from ConfigProvider)
  private config = {
    enabled: true,
    inactivitySeconds: 5,
    countdownSeconds: 3,
  };

  // Optional FocusManager reference for tick-based inactivity checks
  private focusManager: IFocusManager | null = null;

  constructor() {}

  /**
   * Set the FocusManager reference so tick() can query inactivity.
   */
  setFocusManager(focusManager: IFocusManager): void {
    this.focusManager = focusManager;
  }

  get state(): SwitchState {
    return this._state;
  }

  get toastState(): ToastState | null {
    return this._toastState ? { ...this._toastState } : null;
  }

  get declinedSessionIds(): ReadonlySet<string> {
    return this._declinedSessionIds;
  }

  async init(): Promise<void> {
    // Listen for session activation events to clear declined sessions
    this.unsubCoreEvent = await listen<{ topic: string; payload: Record<string, unknown> }>(
      "core-event",
      (e) => {
        const { topic, payload } = e.payload;

        // When user manually switches to a session, clear it from declined set
        if (topic === "session.active_changed") {
          const sessionId = payload.session_id as string;
          if (sessionId && this._declinedSessionIds.has(sessionId)) {
            this._declinedSessionIds.delete(sessionId);
          }
        }

        // Update cached session states when sessions change
        if (
          topic === "session.created" ||
          topic === "session.closed" ||
          topic === "session.state_changed" ||
          topic === "session.active_changed"
        ) {
          this.refreshSessionsDebounced();
        }
      }
    );

    // Initial session fetch
    await this.refreshSessions();

    // Start checking for inactivity every second
    this.checkInterval = setInterval(() => {
      this.tick();
    }, 1000);
  }

  /**
   * Update configuration values.
   */
  updateConfig(config: Partial<typeof this.config>): void {
    this.config = { ...this.config, ...config };
  }

  /**
   * Check inactivity and potentially trigger toast.
   * Called by tick() or external code with current inactivity time.
   */
  checkInactivity(inactivitySeconds: number): void {
    if (this._state !== "idle") return;
    if (!this.config.enabled) return;

    if (inactivitySeconds < this.config.inactivitySeconds) return;

    this.evaluateSwitch();
  }

  /**
   * Decline the current toast, entering cooldown.
   */
  decline(): void {
    if (this._state !== "showing_toast" || !this._toastState) return;

    // Add to declined set
    this._declinedSessionIds.add(this._toastState.targetSessionId);

    // Clear toast and enter cooldown
    this._toastState = null;
    this.transitionTo("cooldown");

    // After cooldown, return to idle
    this.cooldownTimeout = setTimeout(() => {
      if (this._state === "cooldown") {
        this.transitionTo("idle");
      }
    }, COOLDOWN_MS);
  }

  /**
   * Complete the switch (toast countdown finished).
   */
  async completeSwitch(): Promise<void> {
    if (this._state !== "showing_toast" || !this._toastState) return;

    const targetSessionId = this._toastState.targetSessionId;
    this._toastState = null;
    this.transitionTo("switching");

    try {
      await invoke("set_active_session", { sessionId: targetSessionId });
    } catch (err) {
      console.warn("[SessionStateManager] Failed to switch session:", err);
    }

    this.transitionTo("idle");
  }

  /**
   * Cancel the toast without declining (e.g., user activity detected).
   */
  cancelToast(): void {
    if (this._state !== "showing_toast") return;

    this._toastState = null;
    this.transitionTo("idle");
  }

  /**
   * Subscribe to state changes.
   */
  subscribe(listener: StateChangeListener): () => void {
    this.listeners.add(listener);
    // Immediately notify with current state
    listener(this._state, this.toastState);
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

    if (this.checkInterval) {
      clearInterval(this.checkInterval);
      this.checkInterval = null;
    }

    if (this.cooldownTimeout) {
      clearTimeout(this.cooldownTimeout);
      this.cooldownTimeout = null;
    }

    if (this.unsubCoreEvent) {
      this.unsubCoreEvent();
      this.unsubCoreEvent = null;
    }

    if (this.refreshDebounceTimeout) {
      clearTimeout(this.refreshDebounceTimeout);
      this.refreshDebounceTimeout = null;
    }

    this.listeners.clear();
    this._declinedSessionIds.clear();
  }

  // ============================================================================
  // Internal methods
  // ============================================================================

  private tick(): void {
    if (this._state !== "idle") return;
    if (!this.focusManager) return;

    this.checkInactivity(this.focusManager.inactivitySeconds);
  }

  private async evaluateSwitch(): Promise<void> {
    // Transition to checking while we refresh data
    this.transitionTo("checking_inactivity");

    // Ensure fresh data
    await this.refreshSessions();

    const sessions = this.cachedSessions;
    const activeSessionId = this.cachedActiveSessionId;

    // Need at least 2 sessions
    if (sessions.length < 2 || !activeSessionId) {
      this.transitionTo("idle");
      return;
    }

    // Find active session
    const activeSession = sessions.find((s) => s.id === activeSessionId);
    if (!activeSession) {
      this.transitionTo("idle");
      return;
    }

    // If current session is already your_turn, no need to switch
    if (activeSession.state === "your_turn") {
      this.transitionTo("idle");
      return;
    }

    // Find a your_turn session that isn't declined
    const yourTurnSession = sessions.find(
      (s) =>
        s.id !== activeSessionId &&
        s.state === "your_turn" &&
        !this._declinedSessionIds.has(s.id)
    );

    if (!yourTurnSession) {
      this.transitionTo("idle");
      return;
    }

    // Show toast
    this._toastState = {
      targetSessionId: yourTurnSession.id,
      targetSessionName: yourTurnSession.title || `Session ${yourTurnSession.id.slice(0, 8)}`,
      countdownSeconds: this.config.countdownSeconds,
    };
    this.transitionTo("showing_toast");
  }

  private transitionTo(newState: SwitchState): void {
    const prevState = this._state;
    this._state = newState;

    // Clear cooldown timeout if transitioning away from cooldown
    if (prevState === "cooldown" && newState !== "cooldown" && this.cooldownTimeout) {
      clearTimeout(this.cooldownTimeout);
      this.cooldownTimeout = null;
    }

    this.notifyListeners();
  }

  private notifyListeners(): void {
    const state = this._state;
    const toast = this.toastState;

    for (const listener of this.listeners) {
      try {
        listener(state, toast);
      } catch (err) {
        console.error("[SessionStateManager] Listener error:", err);
      }
    }
  }

  private refreshDebounceTimeout: ReturnType<typeof setTimeout> | null = null;

  private refreshSessionsDebounced(): void {
    if (this.refreshDebounceTimeout) {
      clearTimeout(this.refreshDebounceTimeout);
    }
    this.refreshDebounceTimeout = setTimeout(() => {
      this.refreshSessions();
    }, 100);
  }

  private async refreshSessions(): Promise<void> {
    // Single refresh lock to prevent concurrent fetches
    if (this.refreshLock) return;
    this.refreshLock = true;

    try {
      // Fetch both at once to prevent desync
      const [sessions, activeSessionId] = await Promise.all([
        invoke<SessionInfo[]>("list_sessions"),
        invoke<string | null>("get_active_session"),
      ]);

      this.cachedSessions = sessions;
      this.cachedActiveSessionId = activeSessionId;
    } catch (err) {
      console.warn("[SessionStateManager] Failed to refresh sessions:", err);
    } finally {
      this.refreshLock = false;
    }
  }

}
