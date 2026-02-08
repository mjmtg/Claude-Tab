import { useEffect, useRef } from "react";
import { CoreEvent, EventHandler } from "../types/events";
import { useEventBusOptional } from "../kernel/EventBusContext";
import { listen } from "@tauri-apps/api/event";

/**
 * Hook for subscribing to backend events.
 *
 * Uses the EventBus from context if available (preferred),
 * otherwise falls back to direct Tauri listener (legacy support).
 *
 * @param pattern - Event pattern to match ("*", "session.*", "session.**", or exact topic)
 * @param handler - Callback invoked when matching events occur
 *
 * @example
 * ```tsx
 * function MyComponent() {
 *   useEvent("session.created", (event) => {
 *     console.log("New session:", event.payload.session_id);
 *   });
 *
 *   useEvent("session.*", (event) => {
 *     // Matches session.created, session.closed, etc.
 *   });
 *
 *   useEvent("session.**", (event) => {
 *     // Matches session.created, session.state.changed, etc.
 *   });
 * }
 * ```
 */
export function useEvent(pattern: string, handler: EventHandler): void {
  const eventBus = useEventBusOptional();
  // Use ref to avoid re-subscribing on handler changes while keeping latest handler
  const handlerRef = useRef(handler);
  handlerRef.current = handler;

  useEffect(() => {
    // If EventBus is available via context, use it (preferred path)
    if (eventBus) {
      const unsub = eventBus.subscribe(pattern, (event) => {
        handlerRef.current(event);
      });
      return unsub;
    }

    // Fallback: Direct Tauri listener (for components outside EventBusProvider)
    let unlisten: (() => void) | null = null;
    let mounted = true;

    listen<CoreEvent>("core-event", (e) => {
      if (mounted && matchesPattern(pattern, e.payload.topic)) {
        handlerRef.current(e.payload);
      }
    }).then((u) => {
      if (mounted) {
        unlisten = u;
      } else {
        u();
      }
    });

    return () => {
      mounted = false;
      if (unlisten) unlisten();
    };
  }, [pattern, eventBus]);
}

function matchesPattern(pattern: string, topic: string): boolean {
  if (pattern === "*") return true;
  if (pattern.endsWith(".**")) {
    return topic.startsWith(pattern.slice(0, -3));
  }
  if (pattern.endsWith(".*")) {
    const prefix = pattern.slice(0, -2);
    return topic.startsWith(prefix + ".") && !topic.slice(prefix.length + 1).includes(".");
  }
  return pattern === topic;
}
