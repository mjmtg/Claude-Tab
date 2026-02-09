import React, { useState, useEffect, useRef, useCallback } from "react";
import { FrontendExtension } from "../../types/extension";
import { SessionInfo } from "../../types/session";
import { SLOTS } from "../../types/slots";
import type { KeybindingDefinition } from "../../types/kernel";
import { invoke } from "@tauri-apps/api/core";

type PaletteItem =
  | { type: "tab"; id: string; label: string; detail?: string; provider_id: string; isActive: boolean }
  | { type: "command"; id: string; label: string; keys: string; handler: () => void };

let paletteCommands: KeybindingDefinition[] = [];
let showPalette = false;
let paletteListeners: Array<() => void> = [];

function notifyPalette() {
  paletteListeners.forEach((l) => l());
}

function fuzzyMatch(query: string, text: string): boolean {
  const lower = text.toLowerCase();
  let qi = 0;
  for (let i = 0; i < lower.length && qi < query.length; i++) {
    if (lower[i] === query[qi]) qi++;
  }
  return qi === query.length;
}

function CommandPalette() {
  const [visible, setVisible] = useState(false);
  const [query, setQuery] = useState("");
  const [selected, setSelected] = useState(0);
  const [sessions, setSessions] = useState<SessionInfo[]>([]);
  const [activeId, setActiveId] = useState<string | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    const update = () => {
      const wasVisible = visible;
      setVisible(showPalette);
      if (showPalette) {
        setQuery("");
        setSelected(0);
        setTimeout(() => inputRef.current?.focus(), 0);
      } else if (wasVisible) {
        // Return focus to terminal when palette closes
        const terminal = document.querySelector(".xterm-helper-textarea") as HTMLTextAreaElement;
        if (terminal) terminal.focus();
      }
    };
    paletteListeners.push(update);
    return () => {
      paletteListeners = paletteListeners.filter((l) => l !== update);
    };
  }, [visible]);

  useEffect(() => {
    if (visible) {
      invoke<SessionInfo[]>("list_sessions").then(setSessions);
      invoke<string | null>("get_active_session").then(setActiveId);
    }
  }, [visible]);

  const isCommandMode = query.startsWith(">");
  const searchQuery = isCommandMode ? query.slice(1).trimStart().toLowerCase() : query.toLowerCase();

  const items: PaletteItem[] = [];

  if (!isCommandMode) {
    // Show matching tabs first
    const matchingTabs = sessions.filter((s) =>
      searchQuery ? fuzzyMatch(searchQuery, s.title || s.id) : true
    );
    for (const s of matchingTabs) {
      items.push({
        type: "tab",
        id: s.id,
        label: s.title || `Session ${s.id.slice(0, 6)}`,
        detail: s.working_directory?.split("/").filter(Boolean).pop(),
        provider_id: s.provider_id,
        isActive: s.id === activeId,
      });
    }
  }

  // Show matching commands
  const matchingCommands = paletteCommands.filter((cmd) =>
    searchQuery
      ? cmd.label.toLowerCase().includes(searchQuery) ||
        cmd.keys.toLowerCase().includes(searchQuery)
      : true
  );
  for (const cmd of matchingCommands) {
    items.push({
      type: "command",
      id: cmd.id,
      label: cmd.label,
      keys: cmd.keys,
      handler: cmd.handler,
    });
  }

  const execute = useCallback(
    (item: PaletteItem) => {
      showPalette = false;
      notifyPalette();
      if (item.type === "tab") {
        invoke("set_active_session", { sessionId: item.id });
      } else {
        item.handler();
      }
    },
    []
  );

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Escape") {
      showPalette = false;
      notifyPalette();
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      setSelected((s) => Math.min(s + 1, items.length - 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setSelected((s) => Math.max(s - 1, 0));
    } else if (e.key === "Enter" && items[selected]) {
      execute(items[selected]);
    }
  };

  if (!visible) return null;

  return (
    <div
      className="command-palette-backdrop"
      onClick={() => { showPalette = false; notifyPalette(); }}
      role="presentation"
    >
      <div
        className="command-palette"
        onClick={(e) => e.stopPropagation()}
        role="dialog"
        aria-label="Command palette"
        aria-modal="true"
      >
        <input
          ref={inputRef}
          className="command-palette-input"
          placeholder="Search tabs or type > for commands..."
          value={query}
          onChange={(e) => { setQuery(e.target.value); setSelected(0); }}
          onKeyDown={handleKeyDown}
          role="combobox"
          aria-expanded={items.length > 0}
          aria-controls="command-palette-listbox"
          aria-activedescendant={items[selected] ? `palette-item-${items[selected].id}` : undefined}
          aria-autocomplete="list"
        />
        <div className="command-palette-list" id="command-palette-listbox" role="listbox">
          {items.map((item, i) => (
            <div
              key={item.id}
              id={`palette-item-${item.id}`}
              className={`command-palette-item ${i === selected ? "selected" : ""}`}
              onClick={() => execute(item)}
              role="option"
              aria-selected={i === selected}
            >
              {item.type === "tab" ? (
                <>
                  <span className="command-palette-tab-row">
                    <span className={`tab-provider-icon ${item.provider_id === "terminal" ? "tab-provider-terminal" : "tab-provider-claude"}`}>
                      {item.provider_id === "terminal" ? "$" : "C"}
                    </span>
                    <span className="command-label">{item.label}</span>
                    {item.detail && <span className="command-palette-detail">{item.detail}</span>}
                  </span>
                  {item.isActive && <span className="command-palette-active-badge">active</span>}
                </>
              ) : (
                <>
                  <span className="command-label">{item.label}</span>
                  <span className="command-keys">{item.keys}</span>
                </>
              )}
            </div>
          ))}
          {items.length === 0 && (
            <div className="command-palette-empty" role="status">No results found</div>
          )}
        </div>
      </div>
    </div>
  );
}

export function createCommandPaletteExtension(): FrontendExtension {
  return {
    manifest: {
      id: "command-palette",
      name: "Command Palette",
      version: "0.1.0",
      description: "Cmd+K command palette and tab switcher",
    },
    activate(ctx) {
      ctx.componentRegistry.register(SLOTS.OVERLAY, {
        id: "command-palette-overlay",
        component: CommandPalette,
        priority: 100,
        extensionId: "command-palette",
      });

      ctx.keybindingManager.register({
        id: "command-palette.toggle",
        keys: "Cmd+K",
        label: "Command Palette",
        extensionId: "command-palette",
        handler: () => {
          paletteCommands = ctx.keybindingManager.getAll();
          showPalette = !showPalette;
          notifyPalette();
        },
      });
    },
  };
}
