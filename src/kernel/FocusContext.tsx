/**
 * FocusContext - React context for FocusManager
 *
 * Provides the useFocus() hook for extensions to access focus state
 * and operations through proper React state management.
 */

import React, { createContext, useContext, useEffect, useState, useCallback } from "react";
import type { IFocusManager, FocusState } from "../types/kernel";

interface FocusContextValue {
  /** Current focus state */
  state: FocusState;
  /** Whether the window is currently focused */
  isWindowFocused: boolean;
  /** Seconds since last user activity */
  inactivitySeconds: number;
  /** Record user activity */
  recordActivity: () => void;
  /** Focus the application window */
  focusWindow: () => Promise<void>;
  /** Request user attention */
  requestAttention: (critical?: boolean) => Promise<void>;
  /** Check if app is active */
  isAppActive: () => Promise<boolean>;
}

const FocusContext = createContext<FocusContextValue | null>(null);

interface FocusProviderProps {
  manager: IFocusManager;
  children: React.ReactNode;
}

export function FocusProvider({ manager, children }: FocusProviderProps) {
  const [state, setState] = useState<FocusState>(manager.state);

  useEffect(() => {
    return manager.subscribe((newState) => {
      setState(newState);
    });
  }, [manager]);

  const recordActivity = useCallback(() => {
    manager.recordActivity();
  }, [manager]);

  const focusWindow = useCallback(async () => {
    await manager.focusWindow();
  }, [manager]);

  const requestAttention = useCallback(
    async (critical = false) => {
      await manager.requestAttention(critical);
    },
    [manager]
  );

  const isAppActive = useCallback(async () => {
    return manager.isAppActive();
  }, [manager]);

  const value: FocusContextValue = {
    state,
    isWindowFocused: state.windowFocused,
    inactivitySeconds: (Date.now() - state.lastActivityTime) / 1000,
    recordActivity,
    focusWindow,
    requestAttention,
    isAppActive,
  };

  return <FocusContext.Provider value={value}>{children}</FocusContext.Provider>;
}

/**
 * Hook to access focus state and operations.
 *
 * @example
 * const { isWindowFocused, focusWindow, requestAttention } = useFocus();
 *
 * if (!isWindowFocused) {
 *   await focusWindow();
 *   await requestAttention(true);
 * }
 */
export function useFocus(): FocusContextValue {
  const context = useContext(FocusContext);
  if (!context) {
    throw new Error("useFocus must be used within a FocusProvider");
  }
  return context;
}

/**
 * Hook to get just the focus state without triggering re-renders on activity changes.
 * Useful when you only care about window focus, not activity tracking.
 */
export function useWindowFocused(): boolean {
  const context = useContext(FocusContext);
  if (!context) {
    throw new Error("useWindowFocused must be used within a FocusProvider");
  }
  return context.isWindowFocused;
}
