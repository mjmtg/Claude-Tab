import { useState, useEffect } from "react";
import { FrontendExtension } from "../../types/extension";
import { SessionInfo } from "../../types/session";
import { SLOTS } from "../../types/slots";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

function SessionCount() {
  const [count, setCount] = useState(0);

  useEffect(() => {
    let mounted = true;

    const refresh = async () => {
      if (!mounted) return;
      const sessions = await invoke<SessionInfo[]>("list_sessions");
      if (mounted) setCount(sessions.length);
    };
    refresh();

    let unsub: (() => void) | null = null;
    // Only refresh on session created/closed, not every event
    listen<{ topic: string }>("core-event", (e) => {
      const topic = e.payload.topic;
      if (topic === "session.created" || topic === "session.closed") {
        refresh();
      }
    }).then((u) => { unsub = u; });

    return () => {
      mounted = false;
      if (unsub) unsub();
    };
  }, []);

  return <span className="status-item">Sessions: {count}</span>;
}

function ActiveSessionInfo() {
  const [session, setSession] = useState<SessionInfo | null>(null);

  useEffect(() => {
    let mounted = true;

    const refresh = async () => {
      if (!mounted) return;
      const activeId = await invoke<string | null>("get_active_session");
      if (!mounted) return;
      if (activeId) {
        const sessions = await invoke<SessionInfo[]>("list_sessions");
        if (!mounted) return;
        const active = sessions.find((s) => s.id === activeId) ?? null;
        setSession(active);
      } else {
        setSession(null);
      }
    };
    refresh();

    let unsub: (() => void) | null = null;
    // Filter to only session-related events, not high-frequency PTY events
    listen<{ topic: string }>("core-event", (e) => {
      const topic = e.payload.topic;
      if (topic.startsWith("session.")) {
        refresh();
      }
    }).then((u) => { unsub = u; });

    return () => {
      mounted = false;
      if (unsub) unsub();
    };
  }, []);

  if (!session) return null;

  const title = session.title || `Session ${session.id.slice(0, 6)}`;

  return (
    <span className="status-item" style={{ opacity: 0.8 }}>
      {title} ({session.state.replace("core.", "")})
    </span>
  );
}

export function createStatusBarExtension(): FrontendExtension {
  return {
    manifest: {
      id: "status-bar",
      name: "Status Bar",
      version: "0.1.0",
      description: "Session count and active session info",
    },
    activate(ctx) {
      ctx.componentRegistry.register(SLOTS.STATUS_BAR_LEFT, {
        id: "status-bar-count",
        component: SessionCount,
        priority: 10,
        extensionId: "status-bar",
      });

      ctx.componentRegistry.register(SLOTS.STATUS_BAR_RIGHT, {
        id: "status-bar-active",
        component: ActiveSessionInfo,
        priority: 10,
        extensionId: "status-bar",
      });
    },
  };
}
