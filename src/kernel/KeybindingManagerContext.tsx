/**
 * KeybindingManagerContext - React context for KeybindingManager
 *
 * Provides the useKeybindingManager() hook for components (especially terminals)
 * to check if a keyboard event matches a registered app keybinding.
 */

import React, { createContext, useContext } from "react";
import type { IKeybindingManager } from "../types/kernel";

const KeybindingManagerContext = createContext<IKeybindingManager | null>(null);

interface KeybindingManagerProviderProps {
  manager: IKeybindingManager;
  children: React.ReactNode;
}

export function KeybindingManagerProvider({ manager, children }: KeybindingManagerProviderProps) {
  return (
    <KeybindingManagerContext.Provider value={manager}>
      {children}
    </KeybindingManagerContext.Provider>
  );
}

/**
 * Hook to access the KeybindingManager.
 *
 * Primary use case: terminals can check if a key event matches an app keybinding
 * before processing it, preventing character injection issues.
 *
 * @example
 * const keybindingManager = useKeybindingManager();
 * term.attachCustomKeyEventHandler((e: KeyboardEvent) => {
 *   const keyStr = keybindingManager.eventToKeyString(e);
 *   if (keybindingManager.hasBinding(keyStr)) {
 *     return false; // Let app handle this, don't process in terminal
 *   }
 *   return true; // Let terminal handle normally
 * });
 */
export function useKeybindingManager(): IKeybindingManager {
  const context = useContext(KeybindingManagerContext);
  if (!context) {
    throw new Error("useKeybindingManager must be used within a KeybindingManagerProvider");
  }
  return context;
}
