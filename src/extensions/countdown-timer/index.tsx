import { useState, useEffect } from "react";
import { FrontendExtension } from "../../types/extension";
import { SLOTS } from "../../types/slots";

interface CountdownState {
  visible: boolean;
  sessionId: string | null;
  remaining: number;
  total: number;
}

let countdownState: CountdownState = {
  visible: false,
  sessionId: null,
  remaining: 0,
  total: 5,
};

let listeners: Array<() => void> = [];

function notifyListeners() {
  listeners.forEach((l) => l());
}

function CountdownOverlay() {
  const [state, setState] = useState(countdownState);

  useEffect(() => {
    const update = () => setState({ ...countdownState });
    listeners.push(update);
    return () => {
      listeners = listeners.filter((l) => l !== update);
    };
  }, []);

  if (!state.visible) return null;

  const progress = state.remaining / state.total;

  return (
    <div className="countdown-overlay">
      <div className="countdown-content">
        <div className="countdown-label">Switching in</div>
        <div className="countdown-number">{Math.ceil(state.remaining)}s</div>
        <div className="countdown-bar">
          <div
            className="countdown-bar-fill"
            style={{ width: `${progress * 100}%` }}
          />
        </div>
        <div className="countdown-hint">Press Cmd+Shift+P to switch now</div>
      </div>
    </div>
  );
}

export function createCountdownTimerExtension(): FrontendExtension {
  return {
    manifest: {
      id: "countdown-timer",
      name: "Countdown Timer",
      version: "0.1.0",
      description: "Auto-switch countdown overlay",
    },
    activate(ctx) {
      ctx.componentRegistry.register(SLOTS.TERMINAL_OVERLAY, {
        id: "countdown-timer-overlay",
        component: CountdownOverlay,
        priority: 100,
        extensionId: "countdown-timer",
      });

      ctx.eventBus.subscribe("auto-switch.countdown.*", (event) => {
        if (event.topic === "auto-switch.countdown.started") {
          countdownState = {
            visible: true,
            sessionId: event.payload.session_id as string,
            remaining: event.payload.duration as number,
            total: event.payload.duration as number,
          };
          notifyListeners();
        } else if (event.topic === "auto-switch.countdown.tick") {
          countdownState.remaining = event.payload.remaining as number;
          notifyListeners();
        } else if (
          event.topic === "auto-switch.countdown.cancelled" ||
          event.topic === "auto-switch.countdown.completed"
        ) {
          countdownState.visible = false;
          notifyListeners();
        }
      });
    },
  };
}
