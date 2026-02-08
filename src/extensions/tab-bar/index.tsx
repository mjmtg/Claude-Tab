import React, { useState, useEffect, useCallback, useRef } from "react";
import { FrontendExtension } from "../../types/extension";
import { SessionInfo } from "../../types/session";
import { SLOTS } from "../../types/slots";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { SearchBar } from "./SearchBar";
import { HistorySection } from "./HistorySection";
import { ClaudeSession } from "./types";
import { toggleSettings } from "../settings";
import { toggleProfiles } from "../profiles";

// Module-level sidebar state (same pattern as command palette)
let sidebarVisible = localStorage.getItem("sidebarVisible") !== "false";
let sidebarWidth = parseInt(localStorage.getItem("sidebarWidth") || "200", 10);
const MIN_WIDTH = 150;
const MAX_WIDTH = 400;
let sidebarListeners: Array<() => void> = [];

function notifySidebar() {
  sidebarListeners.forEach((l) => l());
  // Apply to DOM
  const el = document.querySelector(".app-sidebar") as HTMLElement;
  if (el) {
    if (sidebarVisible) {
      el.classList.remove("collapsed");
      el.style.width = `${sidebarWidth}px`;
      el.style.minWidth = `${sidebarWidth}px`;
    } else {
      el.classList.add("collapsed");
      el.style.width = "";
      el.style.minWidth = "";
    }
  }
  localStorage.setItem("sidebarVisible", String(sidebarVisible));
}

function toggleSidebar() {
  sidebarVisible = !sidebarVisible;
  notifySidebar();
}

async function cycleTabs(direction: 1 | -1, stateFilter?: string) {
  const sessions = await invoke<SessionInfo[]>("list_sessions");
  const active = await invoke<string | null>("get_active_session");
  const filtered = stateFilter
    ? sessions.filter((s) => s.state === stateFilter)
    : sessions;
  if (filtered.length === 0) return;
  const idx = filtered.findIndex((s) => s.id === active);
  const nextIdx =
    idx === -1 ? 0 : (idx + direction + filtered.length) % filtered.length;
  invoke("set_active_session", { sessionId: filtered[nextIdx].id });
}

const STATE_COLORS: Record<string, string> = {
  active: "#4caf50",
  running: "#2196f3",
  idle: "#808080",
  your_turn: "#ff9800",
};

const STATE_LABELS: Record<string, string> = {
  active: "Active",
  running: "Running",
  idle: "Idle",
  your_turn: "Your Turn",
};

function ContextMenu({
  x,
  y,
  sessionId,
  sessionState,
  onClose,
  onRefresh,
  onRename,
}: {
  x: number;
  y: number;
  sessionId: string;
  sessionState: string;
  onClose: () => void;
  onRefresh: () => void;
  onRename: (id: string) => void;
}) {
  useEffect(() => {
    const handler = () => onClose();
    document.addEventListener("click", handler);
    return () => document.removeEventListener("click", handler);
  }, [onClose]);

  const handleRename = () => {
    onRename(sessionId);
    onClose();
  };

  const handleFork = async () => {
    try {
      await invoke("fork_active_session", { sessionId });
      onRefresh();
    } catch (err) {
      console.error("[ContextMenu] Fork failed:", err);
    }
    onClose();
  };

  const handleMarkIdle = async () => {
    try {
      await invoke("set_session_state", { sessionId, newState: "idle" });
      onRefresh();
    } catch (err) {
      console.error("[ContextMenu] Mark as idle failed:", err);
    }
    onClose();
  };

  const handleMarkActive = async () => {
    try {
      await invoke("set_session_state", { sessionId, newState: "active" });
      onRefresh();
    } catch (err) {
      console.error("[ContextMenu] Mark as active failed:", err);
    }
    onClose();
  };

  const handleClose = async () => {
    try {
      await invoke("close_session", { sessionId });
      onRefresh();
    } catch (err) {
      console.error("[ContextMenu] Close failed:", err);
    }
    onClose();
  };

  return (
    <div
      className="history-context-menu"
      style={{ top: y, left: x }}
    >
      <button type="button" className="history-context-item" onClick={handleRename}>
        Rename
      </button>
      <button type="button" className="history-context-item" onClick={handleFork}>
        Fork
      </button>
      {sessionState !== "idle" && (
        <button type="button" className="history-context-item" onClick={handleMarkIdle}>
          Mark as Idle
        </button>
      )}
      {sessionState === "idle" && (
        <button type="button" className="history-context-item" onClick={handleMarkActive}>
          Mark as Active
        </button>
      )}
      <button type="button" className="history-context-item" onClick={handleClose}>
        Close
      </button>
    </div>
  );
}

function ResizeHandle({ onResize }: { onResize: (delta: number) => void }) {
  const isDragging = useRef(false);
  const startX = useRef(0);

  useEffect(() => {
    const handleMouseMove = (e: MouseEvent) => {
      if (!isDragging.current) return;
      const delta = e.clientX - startX.current;
      startX.current = e.clientX;
      onResize(delta);
    };
    const handleMouseUp = () => {
      isDragging.current = false;
      document.body.style.cursor = '';
      document.body.style.userSelect = '';
    };
    document.addEventListener('mousemove', handleMouseMove);
    document.addEventListener('mouseup', handleMouseUp);
    return () => {
      document.removeEventListener('mousemove', handleMouseMove);
      document.removeEventListener('mouseup', handleMouseUp);
    };
  }, [onResize]);

  return (
    <div
      className="sidebar-resize-handle"
      onMouseDown={(e) => {
        isDragging.current = true;
        startX.current = e.clientX;
        document.body.style.cursor = 'col-resize';
        document.body.style.userSelect = 'none';
      }}
    />
  );
}

function SidePanel() {
  const [sessions, setSessions] = useState<SessionInfo[]>([]);
  const [activeId, setActiveId] = useState<string | null>(null);
  const [collapsed, setCollapsed] = useState<Record<string, boolean>>({});
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editValue, setEditValue] = useState("");
  const searchInputRef = useRef<HTMLInputElement | null>(null);
  const [width, setWidth] = useState(sidebarWidth);
  const [contextMenu, setContextMenu] = useState<{
    x: number;
    y: number;
    sessionId: string;
    sessionState: string;
  } | null>(null);

  const handleResize = useCallback((delta: number) => {
    setWidth((prev) => {
      const newWidth = Math.min(MAX_WIDTH, Math.max(MIN_WIDTH, prev + delta));
      sidebarWidth = newWidth;
      localStorage.setItem("sidebarWidth", String(newWidth));
      const el = document.querySelector(".app-sidebar") as HTMLElement;
      if (el && sidebarVisible) {
        el.style.width = `${newWidth}px`;
        el.style.minWidth = `${newWidth}px`;
      }
      return newWidth;
    });
  }, []);

  const refresh = useCallback(async () => {
    try {
      const list = await invoke<SessionInfo[]>("list_sessions");
      setSessions(list);
      const active = await invoke<string | null>("get_active_session");
      setActiveId(active);
    } catch (err) {
      console.error("[SidePanel] Failed to refresh:", err);
    }
  }, []);

  useEffect(() => {
    let mounted = true;
    refresh();
    const unsubs: Array<() => void> = [];

    // Listen to core events with smarter handling
    listen<{ topic: string; payload: Record<string, unknown> }>("core-event", (e) => {
      if (!mounted) return;
      const { topic, payload } = e.payload;

      // For active session changes, update immediately without full refresh
      if (topic === "session.active_changed") {
        const newActiveId = payload.session_id as string;
        if (newActiveId) {
          setActiveId(newActiveId);
        }
        return;
      }

      // For other events, do full refresh
      if (
        topic === "session.created" ||
        topic === "session.closed" ||
        topic === "session.state_changed" ||
        topic === "session.renamed"
      ) {
        refresh();
      }
    }).then((u) => {
      if (!mounted) { u(); return; }
      unsubs.push(u);
    });

    return () => { mounted = false; unsubs.forEach((u) => u()); };
  }, [refresh]);

  const handleSelect = async (id: string) => {
    await invoke("set_active_session", { sessionId: id });
    setActiveId(id);
  };

  const handleClose = async (id: string, e: React.MouseEvent) => {
    e.stopPropagation();
    await invoke("close_session", { sessionId: id });
    refresh();
  };

  const handleNewTab = async () => {
    const dir = await open({
      title: "Select Working Directory",
      directory: true,
    });
    if (!dir) return;
    const dirName = dir.split("/").filter(Boolean).pop() || dir;
    await invoke("create_session", {
      request: {
        provider_id: "claude-code",
        title: dirName,
        working_directory: dir,
      },
    });
  };

  const handleNewTerminal = async () => {
    await invoke("create_session", {
      request: { provider_id: "terminal", title: "Terminal" },
    });
  };

  // Apply initial sidebar state (collapsed + width)
  useEffect(() => {
    const el = document.querySelector(".app-sidebar") as HTMLElement;
    if (el) {
      if (!sidebarVisible) {
        el.classList.add("collapsed");
      } else {
        el.style.width = `${width}px`;
        el.style.minWidth = `${width}px`;
      }
    }
  }, [width]);

  // Listen for F2 rename trigger
  useEffect(() => {
    const handleRenameEvent = () => {
      if (activeId) {
        const s = sessions.find((s) => s.id === activeId);
        setEditingId(activeId);
        setEditValue(s?.title || "");
      }
    };
    window.addEventListener("tab-bar:rename-active", handleRenameEvent);
    return () => window.removeEventListener("tab-bar:rename-active", handleRenameEvent);
  }, [activeId, sessions]);

  const handleSearchResultClick = async (result: { session_id: string }) => {
    try {
      await invoke("resume_session", { claudeSessionId: result.session_id });
      refresh();
    } catch (err) {
      console.error("[SidePanel] Resume from search failed:", err);
    }
  };

  const handleResume = async (session: ClaudeSession) => {
    try {
      await invoke("resume_session", { claudeSessionId: session.session_id });
      refresh();
    } catch (err) {
      console.error("[SidePanel] Resume failed:", err);
    }
  };

  const handleFork = async (session: ClaudeSession) => {
    try {
      await invoke("fork_session", { claudeSessionId: session.session_id });
      refresh();
    } catch (err) {
      console.error("[SidePanel] Fork failed:", err);
    }
  };

  const handleContextMenu = (e: React.MouseEvent, session: SessionInfo) => {
    e.preventDefault();
    setContextMenu({
      x: Math.min(e.clientX, window.innerWidth - 120),
      y: Math.min(e.clientY, window.innerHeight - 120),
      sessionId: session.id,
      sessionState: session.state,
    });
  };

  const handleDoubleClick = (s: SessionInfo) => {
    setEditingId(s.id);
    setEditValue(s.title || "");
  };

  const handleRenameSubmit = async (id: string) => {
    const trimmed = editValue.trim();
    if (trimmed) {
      await invoke("rename_session", { sessionId: id, title: trimmed });
    }
    setEditingId(null);
  };

  const handleRenameKeyDown = (e: React.KeyboardEvent, id: string) => {
    if (e.key === "Enter") {
      handleRenameSubmit(id);
    } else if (e.key === "Escape") {
      setEditingId(null);
    }
  };

  const startRename = (id: string) => {
    const s = sessions.find((s) => s.id === id);
    setEditingId(id);
    setEditValue(s?.title || "");
  };

  // Separate terminal sessions from Claude sessions
  const claudeSessions = sessions.filter((s) => s.provider_id !== "terminal");
  const terminalSessions = sessions.filter((s) => s.provider_id === "terminal");

  // Group Claude sessions by state
  const groups = claudeSessions.reduce<Record<string, SessionInfo[]>>((acc, s) => {
    const key = s.state;
    if (!acc[key]) acc[key] = [];
    acc[key].push(s);
    return acc;
  }, {});

  const groupOrder = ["your_turn", "running", "active", "idle"];
  const sortedGroups = Object.entries(groups).sort(([a], [b]) => {
    const ai = groupOrder.indexOf(a);
    const bi = groupOrder.indexOf(b);
    return (ai === -1 ? 99 : ai) - (bi === -1 ? 99 : bi);
  });

  const toggleGroup = (state: string) => {
    setCollapsed((prev) => ({ ...prev, [state]: !prev[state] }));
  };

  const renderSessionItem = (s: SessionInfo) => (
    <div
      key={s.id}
      className={`side-panel-item ${s.id === activeId ? "side-panel-item-active" : ""}`}
      onClick={() => handleSelect(s.id)}
      onContextMenu={(e) => handleContextMenu(e, s)}
      title={s.summary ?? s.working_directory ?? undefined}
    >
      <div className="side-panel-item-info">
        {editingId === s.id ? (
          <input
            className="side-panel-rename-input"
            value={editValue}
            onChange={(e) => setEditValue(e.target.value)}
            onBlur={() => handleRenameSubmit(s.id)}
            onKeyDown={(e) => handleRenameKeyDown(e, s.id)}
            autoFocus
            onClick={(e) => e.stopPropagation()}
          />
        ) : (
          <span className="side-panel-item-title" onDoubleClick={() => handleDoubleClick(s)}>
            {s.title || `Session ${s.id.slice(0, 6)}`}
          </span>
        )}
        {s.subtitle && (
          <span className="side-panel-item-subtitle">{s.subtitle}</span>
        )}
        {s.tags && s.tags.length > 0 && (
          <div className="side-panel-item-tags">
            {s.tags.map((tag) => (
              <span key={tag} className="side-panel-tag">{tag}</span>
            ))}
          </div>
        )}
        {s.working_directory && (
          <span className="side-panel-item-dir">
            {s.working_directory.split("/").filter(Boolean).pop()}
          </span>
        )}
      </div>
      <button
        className="side-panel-item-close"
        onClick={(e) => handleClose(s.id, e)}
        aria-label={`Close ${s.title || 'session'}`}
      >
        &times;
      </button>
    </div>
  );

  return (
    <div className="side-panel">
      <div className="side-panel-toolbar">
        <SearchBar onResultClick={handleSearchResultClick} inputRef={searchInputRef} />
        <button className="side-panel-new-btn" onClick={handleNewTab} title="New Session (Cmd+T)" aria-label="New Session">
          +
        </button>
        <button className="side-panel-new-btn" onClick={handleNewTerminal} title="New Terminal (Cmd+Shift+T)" aria-label="New Terminal">
          $
        </button>
        <button className="side-panel-new-btn" onClick={toggleProfiles} title="Profiles (Cmd+Shift+P)" aria-label="Profiles">
          ▶
        </button>
        <button className="side-panel-new-btn" onClick={toggleSettings} title="Settings (Cmd+,)" aria-label="Settings">
          ⚙
        </button>
      </div>
      <div className="side-panel-groups">
        {sortedGroups.map(([state, items]) => (
          <div key={state} className="side-panel-group">
            <div
              className="side-panel-group-header"
              onClick={() => toggleGroup(state)}
            >
              <span
                className="side-panel-group-indicator"
                style={{ backgroundColor: STATE_COLORS[state] ?? "#808080" }}
              />
              <span className="side-panel-group-label">
                {STATE_LABELS[state] ?? state}
              </span>
              <span className="side-panel-group-count">{items.length}</span>
              <span className={`side-panel-chevron ${collapsed[state] ? "collapsed" : ""}`}>
                &#9662;
              </span>
            </div>
            {!collapsed[state] && (
              <div className="side-panel-group-items">
                {items.map(renderSessionItem)}
              </div>
            )}
          </div>
        ))}
        {terminalSessions.length > 0 && (
          <div className="side-panel-group">
            <div
              className="side-panel-group-header"
              onClick={() => toggleGroup("_terminals")}
            >
              <span
                className="side-panel-group-indicator"
                style={{ backgroundColor: "#a0a0a0" }}
              />
              <span className="side-panel-group-label">Terminals</span>
              <span className="side-panel-group-count">{terminalSessions.length}</span>
              <span className={`side-panel-chevron ${collapsed["_terminals"] ? "collapsed" : ""}`}>
                &#9662;
              </span>
            </div>
            {!collapsed["_terminals"] && (
              <div className="side-panel-group-items">
                {terminalSessions.map(renderSessionItem)}
              </div>
            )}
          </div>
        )}
        {sessions.length === 0 && (
          <div className="side-panel-empty">
            <span>No active sessions</span>
            <span className="side-panel-empty-hint">
              Press <kbd className="kbd">⌘T</kbd> to start a new session
            </span>
          </div>
        )}
        <HistorySection onResume={handleResume} onFork={handleFork} />
      </div>
      {contextMenu && (
        <ContextMenu
          x={contextMenu.x}
          y={contextMenu.y}
          sessionId={contextMenu.sessionId}
          sessionState={contextMenu.sessionState}
          onClose={() => setContextMenu(null)}
          onRefresh={refresh}
          onRename={startRename}
        />
      )}
      <ResizeHandle onResize={handleResize} />
    </div>
  );
}

function SidebarToggleButton() {
  const [visible, setVisible] = useState(sidebarVisible);

  useEffect(() => {
    const listener = () => setVisible(sidebarVisible);
    sidebarListeners.push(listener);
    return () => {
      sidebarListeners = sidebarListeners.filter((l) => l !== listener);
    };
  }, []);

  return (
    <button
      className="sidebar-toggle"
      onClick={toggleSidebar}
      title={visible ? "Hide Sidebar (Cmd+B)" : "Show Sidebar (Cmd+B)"}
    >
      {visible ? "\u2039" : "\u203A"}
    </button>
  );
}

export function createTabBarExtension(): FrontendExtension {
  return {
    manifest: {
      id: "tab-bar",
      name: "Tab Bar",
      version: "0.1.0",
      description: "Tab management UI with grouping",
    },
    activate(ctx) {
      ctx.componentRegistry.register(SLOTS.TAB_BAR_LEFT, {
        id: "tab-bar-sidebar-toggle",
        component: SidebarToggleButton,
        priority: 10,
        extensionId: "tab-bar",
      });

      ctx.componentRegistry.register(SLOTS.SIDE_PANEL, {
        id: "tab-bar-side-panel",
        component: SidePanel,
        priority: 10,
        extensionId: "tab-bar",
      });

      ctx.keybindingManager.register({
        id: "tab-bar.toggle-sidebar",
        keys: "Cmd+B",
        label: "Toggle Sidebar",
        extensionId: "tab-bar",
        handler: toggleSidebar,
      });

      ctx.keybindingManager.register({
        id: "tab-bar.new-tab",
        keys: "Cmd+T",
        label: "New Session",
        extensionId: "tab-bar",
        handler: async () => {
          const dir = await open({
            title: "Select Working Directory",
            directory: true,
          });
          if (!dir) return;
          const dirName = dir.split("/").filter(Boolean).pop() || dir;
          await invoke("create_session", {
            request: {
              provider_id: "claude-code",
              title: dirName,
              working_directory: dir,
            },
          });
        },
      });

      ctx.keybindingManager.register({
        id: "tab-bar.new-terminal",
        keys: "Cmd+Shift+T",
        label: "New Terminal",
        extensionId: "tab-bar",
        handler: async () => {
          await invoke("create_session", {
            request: { provider_id: "terminal", title: "Terminal" },
          });
        },
      });

      ctx.keybindingManager.register({
        id: "tab-bar.rename-tab",
        keys: "F2",
        label: "Rename Tab",
        extensionId: "tab-bar",
        handler: () => {
          window.dispatchEvent(new Event("tab-bar:rename-active"));
        },
      });

      ctx.keybindingManager.register({
        id: "tab-bar.close-tab",
        keys: "Cmd+W",
        label: "Close Tab",
        extensionId: "tab-bar",
        handler: async () => {
          const active = await invoke<string | null>("get_active_session");
          if (active) {
            invoke("close_session", { sessionId: active });
          }
        },
      });

      ctx.keybindingManager.register({
        id: "tab-bar.next-tab",
        keys: "Ctrl+Tab",
        label: "Next Tab",
        extensionId: "tab-bar",
        handler: () => cycleTabs(1),
      });

      ctx.keybindingManager.register({
        id: "tab-bar.prev-tab",
        keys: "Ctrl+Shift+Tab",
        label: "Previous Tab",
        extensionId: "tab-bar",
        handler: () => cycleTabs(-1),
      });

      ctx.keybindingManager.register({
        id: "tab-bar.next-active-tab",
        keys: "Cmd+Alt+ArrowRight",
        label: "Next Active Tab",
        extensionId: "tab-bar",
        handler: () => cycleTabs(1, "active"),
      });

      ctx.keybindingManager.register({
        id: "tab-bar.prev-active-tab",
        keys: "Cmd+Alt+ArrowLeft",
        label: "Previous Active Tab",
        extensionId: "tab-bar",
        handler: () => cycleTabs(-1, "active"),
      });

      ctx.keybindingManager.register({
        id: "tab-bar.next-your-turn-tab",
        keys: "Cmd+Shift+ArrowRight",
        label: "Next Your Turn Tab",
        extensionId: "tab-bar",
        handler: () => cycleTabs(1, "your_turn"),
      });

      ctx.keybindingManager.register({
        id: "tab-bar.prev-your-turn-tab",
        keys: "Cmd+Shift+ArrowLeft",
        label: "Previous Your Turn Tab",
        extensionId: "tab-bar",
        handler: () => cycleTabs(-1, "your_turn"),
      });

      ctx.keybindingManager.register({
        id: "tab-bar.fork-session",
        keys: "Cmd+Shift+K",
        label: "Fork Current Session",
        extensionId: "tab-bar",
        handler: async () => {
          const active = await invoke<string | null>("get_active_session");
          if (active) {
            try {
              await invoke("fork_active_session", { sessionId: active });
            } catch (err) {
              console.error("[tab-bar] Fork failed:", err);
            }
          }
        },
      });

      ctx.keybindingManager.register({
        id: "tab-bar.new-terminal-in-cwd",
        keys: "Cmd+Alt+T",
        label: "New Terminal in Current Directory",
        extensionId: "tab-bar",
        handler: async () => {
          const activeId = await invoke<string | null>("get_active_session");
          let workingDir: string | null = null;

          if (activeId) {
            const sessions = await invoke<SessionInfo[]>("list_sessions");
            const activeSession = sessions.find((s) => s.id === activeId);
            workingDir = activeSession?.working_directory ?? null;
          }

          if (workingDir) {
            const dirName = workingDir.split("/").filter(Boolean).pop() || workingDir;
            await invoke("create_session", {
              request: {
                provider_id: "terminal",
                title: dirName,
                working_directory: workingDir,
              },
            });
          } else {
            // Fallback: create terminal without specific directory
            await invoke("create_session", {
              request: { provider_id: "terminal", title: "Terminal" },
            });
          }
        },
      });

      ctx.keybindingManager.register({
        id: "tab-bar.new-claude-in-cwd",
        keys: "Cmd+Alt+C",
        label: "New Claude in Current Directory",
        extensionId: "tab-bar",
        handler: async () => {
          const activeId = await invoke<string | null>("get_active_session");
          let workingDir: string | null = null;

          if (activeId) {
            const sessions = await invoke<SessionInfo[]>("list_sessions");
            const activeSession = sessions.find((s) => s.id === activeId);
            workingDir = activeSession?.working_directory ?? null;
          }

          if (workingDir) {
            const dirName = workingDir.split("/").filter(Boolean).pop() || workingDir;
            await invoke("create_session", {
              request: {
                provider_id: "claude-code",
                title: dirName,
                working_directory: workingDir,
              },
            });
          } else {
            // Fallback: open directory picker
            const dir = await open({
              title: "Select Working Directory",
              directory: true,
            });
            if (!dir) return;
            const dirName = dir.split("/").filter(Boolean).pop() || dir;
            await invoke("create_session", {
              request: {
                provider_id: "claude-code",
                title: dirName,
                working_directory: dir,
              },
            });
          }
        },
      });

      for (let i = 1; i <= 9; i++) {
        ctx.keybindingManager.register({
          id: `tab-bar.switch-${i}`,
          keys: `Cmd+${i}`,
          label: `Switch to Tab ${i}`,
          extensionId: "tab-bar",
          handler: async () => {
            const sessions = await invoke<SessionInfo[]>("list_sessions");
            if (sessions[i - 1]) {
              invoke("set_active_session", { sessionId: sessions[i - 1].id });
            }
          },
        });
      }

      ctx.keybindingManager.register({
        id: "tab-bar.focus-search",
        keys: "Cmd+Shift+F",
        label: "Search History",
        extensionId: "tab-bar",
        handler: async () => {
          // Focus the search input in the side panel
          const input = document.querySelector(".search-bar-input") as HTMLInputElement;
          if (input) input.focus();
        },
      });
    },
  };
}
