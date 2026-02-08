import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { SessionInfo } from "../types/session";

export function useSession() {
  const [sessions, setSessions] = useState<SessionInfo[]>([]);
  const [activeId, setActiveId] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    const list = await invoke<SessionInfo[]>("list_sessions");
    setSessions(list);
    const active = await invoke<string | null>("get_active_session");
    setActiveId(active);
  }, []);

  useEffect(() => {
    refresh();

    let unlisten: (() => void) | null = null;
    listen("core-event", () => refresh()).then((u) => {
      unlisten = u;
    });

    return () => {
      if (unlisten) unlisten();
    };
  }, [refresh]);

  return { sessions, activeId, refresh };
}
