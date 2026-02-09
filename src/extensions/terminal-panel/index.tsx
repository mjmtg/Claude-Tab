import React, { useEffect, useState, useCallback, useRef } from "react";
import { FrontendExtension } from "../../types/extension";
import { SLOTS } from "../../types/slots";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { UnifiedTerminal } from "./UnifiedTerminal";

// Memoized UnifiedTerminal to prevent unnecessary re-renders
const MemoizedUnifiedTerminal = React.memo(UnifiedTerminal, (prev, next) =>
  prev.sessionId === next.sessionId && prev.isActive === next.isActive
);

function TerminalPanel() {
  const [activeId, setActiveId] = useState<string | null>(null);
  const [sessions, setSessions] = useState<Set<string>>(new Set());
  const mountedRef = useRef(true);

  // Listen for session events - event-driven, no polling
  useEffect(() => {
    mountedRef.current = true;
    const unsubs: Array<() => void> = [];

    const setup = async () => {
      const u1 = await listen<{ topic: string; payload: Record<string, unknown> }>("core-event", (e) => {
        if (!mountedRef.current) return;
        const { topic, payload } = e.payload;

        if (topic === "session.created") {
          const sid = payload.session_id as string;
          setSessions(prev => new Set(prev).add(sid));
          setActiveId(sid);
        } else if (topic === "session.closed") {
          const sid = payload.session_id as string;
          setSessions(prev => {
            const next = new Set(prev);
            next.delete(sid);
            return next;
          });
          setActiveId(current => current === sid ? null : current);
        } else if (topic === "session.active_changed") {
          const sid = payload.session_id as string;
          if (sid) {
            setActiveId(sid);
            setSessions(prev => {
              if (!prev.has(sid)) {
                return new Set(prev).add(sid);
              }
              return prev;
            });
          }
        }
      });
      if (!mountedRef.current) { u1(); return; }
      unsubs.push(u1);
    };

    setup();

    // Check active session on mount only (not polling)
    const checkActive = async () => {
      if (!mountedRef.current) return;
      const active = await invoke<string | null>("get_active_session");
      if (mountedRef.current && active) {
        setActiveId(active);
        setSessions(prev => {
          if (!prev.has(active)) {
            return new Set(prev).add(active);
          }
          return prev;
        });
      }
    };
    checkActive();

    return () => {
      mountedRef.current = false;
      unsubs.forEach((u) => u());
    };
  }, []);

  // Emit activity event for inactivity tracking when user interacts
  const handleActivity = useCallback(() => {
    window.dispatchEvent(new CustomEvent("terminal:activity"));
  }, []);

  if (!activeId) {
    return (
      <div
        style={{
          width: "100%",
          height: "100%",
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          backgroundColor: "var(--terminal-bg, #1e1e1e)",
          color: "var(--text-tertiary, #666)",
        }}
      >
        No active session
      </div>
    );
  }

  return (
    <div
      style={{ width: "100%", height: "100%", position: "relative" }}
      onKeyDown={handleActivity}
      onClick={handleActivity}
    >
      {Array.from(sessions).map(sessionId => (
        <div
          key={sessionId}
          style={{
            width: "100%",
            height: "100%",
            position: "absolute",
            top: 0,
            left: 0,
            display: sessionId === activeId ? "block" : "none",
          }}
        >
          <MemoizedUnifiedTerminal sessionId={sessionId} isActive={sessionId === activeId} />
        </div>
      ))}
    </div>
  );
}

export function createTerminalPanelExtension(): FrontendExtension {
  return {
    manifest: {
      id: "terminal-panel",
      name: "Terminal Panel",
      version: "0.3.0",
      description: "Unified terminal with mouse gesture support",
      dependencies: ["tab-bar"],
    },
    activate(ctx) {
      ctx.componentRegistry.register(SLOTS.MAIN_CONTENT, {
        id: "terminal-panel-main",
        component: TerminalPanel,
        priority: 10,
        extensionId: "terminal-panel",
      });
    },
  };
}
