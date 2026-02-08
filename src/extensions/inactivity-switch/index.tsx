import { useEffect, useRef } from "react";
import { FrontendExtension, ExtensionContext } from "../../types/extension";
import { SLOTS } from "../../types/slots";
import { InactivityToast } from "./InactivityToast";
import { useSessionState } from "../../kernel/SessionStateContext";
import { useFocus } from "../../kernel/FocusContext";
import type { IFocusManager, ISessionStateManager } from "../../types/kernel";

/**
 * Inactivity Switch Extension
 *
 * When user is inactive on a non-your_turn session while another
 * session is in "your_turn" state, shows a countdown toast and
 * auto-switches to the your_turn session.
 *
 * Uses the SessionStateManager state machine to prevent race conditions
 * and the FocusManager for activity tracking.
 */

// Managers accessed from extension activation
let focusManager: IFocusManager | null = null;
let sessionStateManager: ISessionStateManager | null = null;
let checkInterval: ReturnType<typeof setInterval> | null = null;

function InactivityOverlay() {
  const { isToastShowing, toastState, completeSwitch, cancelToast } = useSessionState();
  const { recordActivity } = useFocus();
  const prevToastRef = useRef<boolean>(false);

  // Cancel toast on user activity
  useEffect(() => {
    const handleActivity = () => {
      if (isToastShowing) {
        // User activity while toast showing - cancel without declining
        cancelToast();
      }
      recordActivity();
    };

    window.addEventListener("terminal:activity", handleActivity);
    return () => {
      window.removeEventListener("terminal:activity", handleActivity);
    };
  }, [isToastShowing, cancelToast, recordActivity]);

  // Track toast state for activity reset
  useEffect(() => {
    if (!isToastShowing && prevToastRef.current) {
      // Toast just closed - reset activity to prevent immediate re-trigger
      recordActivity();
    }
    prevToastRef.current = isToastShowing;
  }, [isToastShowing, recordActivity]);

  if (!isToastShowing || !toastState) return null;

  // Use cancelToast for X button/Escape - just dismiss, don't prevent re-offer
  // The declined set is only populated when user manually switches away
  return (
    <InactivityToast
      targetSessionName={toastState.targetSessionName}
      countdownSeconds={toastState.countdownSeconds}
      onComplete={completeSwitch}
      onCancel={cancelToast}
    />
  );
}

export function createInactivitySwitchExtension(): FrontendExtension {
  return {
    manifest: {
      id: "inactivity-switch",
      name: "Inactivity Switch",
      version: "0.1.0",
      description: "Auto-switch to your_turn sessions after inactivity",
    },

    async activate(ctx: ExtensionContext) {
      focusManager = ctx.focusManager;
      sessionStateManager = ctx.sessionStateManager;

      // Check inactivity every second
      checkInterval = setInterval(() => {
        if (!focusManager || !sessionStateManager) return;

        // Only check if feature is enabled
        const enabled = getConfigValue("autoFocus.tabAutoSwitch", true);
        if (!enabled) return;

        const inactivitySeconds = focusManager.inactivitySeconds;
        sessionStateManager.checkInactivity(inactivitySeconds);
      }, 1000);

      // Register the overlay component
      ctx.componentRegistry.register(SLOTS.OVERLAY, {
        id: "inactivity-overlay",
        component: InactivityOverlay,
        priority: 95,
        extensionId: "inactivity-switch",
      });
    },

    deactivate() {
      if (checkInterval) {
        clearInterval(checkInterval);
        checkInterval = null;
      }
      focusManager = null;
      sessionStateManager = null;
    },
  };
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
