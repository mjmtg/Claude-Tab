import React, { useEffect, useState, useCallback, useRef } from "react";
import { FrontendExtension } from "../../types/extension";
import { SLOTS } from "../../types/slots";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { UnifiedTerminal } from "./UnifiedTerminal";

// Maximum number of "hot" terminals to keep in memory
const MAX_HOT_TERMINALS = 3;

// LRU cache for terminal access order
class TerminalLRU {
  private order: string[] = [];

  touch(sessionId: string): void {
    // Remove if exists, then add to end (most recent)
    const idx = this.order.indexOf(sessionId);
    if (idx !== -1) {
      this.order.splice(idx, 1);
    }
    this.order.push(sessionId);
  }

  remove(sessionId: string): void {
    const idx = this.order.indexOf(sessionId);
    if (idx !== -1) {
      this.order.splice(idx, 1);
    }
  }

  // Get sessions that should be "cold" (disposed)
  getColdSessions(allSessions: Set<string>, maxHot: number): string[] {
    // Filter to only include sessions that still exist
    const validOrder = this.order.filter(id => allSessions.has(id));
    this.order = validOrder;

    // Sessions beyond maxHot from the end are cold
    if (validOrder.length <= maxHot) return [];
    return validOrder.slice(0, validOrder.length - maxHot);
  }

  // Get the hot sessions (most recently accessed)
  getHotSessions(allSessions: Set<string>, maxHot: number): Set<string> {
    const validOrder = this.order.filter(id => allSessions.has(id));
    const hotIds = validOrder.slice(-maxHot);
    return new Set(hotIds);
  }
}

// Memoized UnifiedTerminal to prevent unnecessary re-renders
const MemoizedUnifiedTerminal = React.memo(UnifiedTerminal, (prev, next) =>
  prev.sessionId === next.sessionId && prev.isActive === next.isActive
);

function TerminalPanel() {
  const [activeId, setActiveId] = useState<string | null>(null);
  const [sessions, setSessions] = useState<Set<string>>(new Set());
  const [hotSessions, setHotSessions] = useState<Set<string>>(new Set());
  const mountedRef = useRef(true);
  const lruRef = useRef(new TerminalLRU());

  // Update hot sessions based on LRU
  const updateHotSessions = useCallback((allSessions: Set<string>, currentActive: string | null) => {
    // Always ensure active session is hot
    if (currentActive) {
      lruRef.current.touch(currentActive);
    }
    const hot = lruRef.current.getHotSessions(allSessions, MAX_HOT_TERMINALS);
    // Always include active session in hot set
    if (currentActive && allSessions.has(currentActive)) {
      hot.add(currentActive);
    }
    setHotSessions(hot);
  }, []);

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
          setSessions(prev => {
            const next = new Set(prev).add(sid);
            lruRef.current.touch(sid);
            // Defer hot session update to avoid state update in callback
            setTimeout(() => updateHotSessions(next, sid), 0);
            return next;
          });
          setActiveId(sid);
        } else if (topic === "session.closed") {
          const sid = payload.session_id as string;
          setSessions(prev => {
            const next = new Set(prev);
            next.delete(sid);
            lruRef.current.remove(sid);
            return next;
          });
          setActiveId(current => current === sid ? null : current);
        } else if (topic === "session.active_changed") {
          const sid = payload.session_id as string;
          if (sid) {
            lruRef.current.touch(sid);
            setActiveId(sid);
            setSessions(prev => {
              if (!prev.has(sid)) {
                const next = new Set(prev).add(sid);
                setTimeout(() => updateHotSessions(next, sid), 0);
                return next;
              }
              setTimeout(() => updateHotSessions(prev, sid), 0);
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
        lruRef.current.touch(active);
        setActiveId(active);
        setSessions(prev => {
          if (!prev.has(active)) {
            const next = new Set(prev).add(active);
            updateHotSessions(next, active);
            return next;
          }
          updateHotSessions(prev, active);
          return prev;
        });
      }
    };
    checkActive();

    return () => {
      mountedRef.current = false;
      unsubs.forEach((u) => u());
    };
  }, [updateHotSessions]);

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
          backgroundColor: "#1a1a2e",
          color: "#666",
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
      {/* Only render hot terminals to save memory */}
      {Array.from(sessions).filter(id => hotSessions.has(id)).map(sessionId => (
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
