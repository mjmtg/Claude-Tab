/**
 * SessionStateContext - React context for SessionStateManager
 *
 * Provides the useSessionState() hook for extensions to access the
 * session switching state machine through proper React state management.
 */

import React, { createContext, useContext, useEffect, useState, useCallback } from "react";
import type { ISessionStateManager, SwitchState, ToastState } from "../types/kernel";

interface SessionStateContextValue {
  /** Current state machine state */
  switchState: SwitchState;
  /** Current toast state (if showing) */
  toastState: ToastState | null;
  /** Whether a toast is currently showing */
  isToastShowing: boolean;
  /** Session IDs that user has declined */
  declinedSessionIds: ReadonlySet<string>;
  /** Check inactivity and potentially trigger toast */
  checkInactivity: (inactivitySeconds: number) => void;
  /** Decline the current toast */
  decline: () => void;
  /** Complete the switch (toast countdown finished) */
  completeSwitch: () => Promise<void>;
  /** Cancel the toast without declining */
  cancelToast: () => void;
}

const SessionStateContext = createContext<SessionStateContextValue | null>(null);

interface SessionStateProviderProps {
  manager: ISessionStateManager;
  children: React.ReactNode;
}

export function SessionStateProvider({ manager, children }: SessionStateProviderProps) {
  const [switchState, setSwitchState] = useState<SwitchState>(manager.state);
  const [toastState, setToastState] = useState<ToastState | null>(manager.toastState);
  const [declinedSessionIds, setDeclinedSessionIds] = useState<ReadonlySet<string>>(
    manager.declinedSessionIds
  );

  useEffect(() => {
    return manager.subscribe((newState, newToast) => {
      setSwitchState(newState);
      setToastState(newToast);
      setDeclinedSessionIds(new Set(manager.declinedSessionIds));
    });
  }, [manager]);

  const checkInactivity = useCallback(
    (inactivitySeconds: number) => {
      manager.checkInactivity(inactivitySeconds);
    },
    [manager]
  );

  const decline = useCallback(() => {
    manager.decline();
  }, [manager]);

  const completeSwitch = useCallback(async () => {
    await manager.completeSwitch();
  }, [manager]);

  const cancelToast = useCallback(() => {
    manager.cancelToast();
  }, [manager]);

  const value: SessionStateContextValue = {
    switchState,
    toastState,
    isToastShowing: switchState === "showing_toast" && toastState !== null,
    declinedSessionIds,
    checkInactivity,
    decline,
    completeSwitch,
    cancelToast,
  };

  return (
    <SessionStateContext.Provider value={value}>{children}</SessionStateContext.Provider>
  );
}

/**
 * Hook to access session state machine and operations.
 *
 * @example
 * const { isToastShowing, toastState, decline, completeSwitch } = useSessionState();
 *
 * if (isToastShowing && toastState) {
 *   // Render toast with toastState.targetSessionName
 *   // Call decline() on cancel, completeSwitch() when countdown ends
 * }
 */
export function useSessionState(): SessionStateContextValue {
  const context = useContext(SessionStateContext);
  if (!context) {
    throw new Error("useSessionState must be used within a SessionStateProvider");
  }
  return context;
}

/**
 * Hook to get just the toast state.
 * Returns null when no toast is showing.
 */
export function useToastState(): ToastState | null {
  const context = useContext(SessionStateContext);
  if (!context) {
    throw new Error("useToastState must be used within a SessionStateProvider");
  }
  return context.toastState;
}
