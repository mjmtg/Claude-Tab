/**
 * UnifiedTerminal - Simple xterm.js terminal connected to PTY
 *
 * Standard terminal behavior - all input goes directly to PTY,
 * all output displayed as-is. Scrollback preserved while running.
 */

import { useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { WebglAddon } from "@xterm/addon-webgl";
import { PtyOutputEvent } from "../../types/events";
import { useKeybindingManager } from "../../kernel/KeybindingManagerContext";

interface UnifiedTerminalProps {
  sessionId: string;
  isActive?: boolean;
}

export function UnifiedTerminal({ sessionId, isActive = false }: UnifiedTerminalProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const terminalRef = useRef<Terminal | null>(null);
  const fitAddonRef = useRef<FitAddon | null>(null);
  const keybindingManager = useKeybindingManager();

  // Send data to PTY
  const sendToPty = useCallback(async (data: string) => {
    try {
      const bytes = Array.from(new TextEncoder().encode(data));
      await invoke("write_to_pty", { sessionId, data: bytes });
    } catch (err) {
      console.error("Failed to write to PTY:", err);
    }
  }, [sessionId]);

  // Initialize terminal
  useEffect(() => {
    if (!containerRef.current || terminalRef.current) return;

    const term = new Terminal({
      cursorBlink: true,
      cursorStyle: "bar",
      disableStdin: false,
      fontSize: 14,
      fontFamily: "JetBrains Mono, Menlo, Monaco, Consolas, monospace",
      theme: {
        background: "#1a1a2e",
        foreground: "#e0e0e0",
        cursor: "#e0e0e0",
        selectionBackground: "#3a3a5e",
      },
      allowProposedApi: true,
      scrollback: 10000,
    });

    const fitAddon = new FitAddon();
    term.loadAddon(fitAddon);
    fitAddonRef.current = fitAddon;

    term.open(containerRef.current);

    // Try WebGL addon for better performance
    try {
      const webglAddon = new WebglAddon();
      webglAddon.onContextLoss(() => webglAddon.dispose());
      term.loadAddon(webglAddon);
    } catch {
      // Canvas fallback - no action needed
    }

    terminalRef.current = term;

    // Intercept app keybindings before xterm processes them
    // This prevents character injection (e.g., Alt+C producing 'ç')
    // when pressing app shortcuts like Cmd+Alt+C
    term.attachCustomKeyEventHandler((e: KeyboardEvent) => {
      const keyStr = keybindingManager.eventToKeyString(e);
      if (keybindingManager.hasBinding(keyStr)) {
        // Let the app's KeybindingManager handle this key
        // Return false to prevent xterm from processing it
        return false;
      }
      // Let xterm handle normally
      return true;
    });

    // Handle keyboard input - send directly to PTY
    term.onData((data) => {
      sendToPty(data);
    });

    // Initial fit
    requestAnimationFrame(() => {
      fitAddon.fit();
      invoke("resize_pty", {
        sessionId,
        rows: term.rows,
        cols: term.cols,
      }).catch(console.error);
    });

    return () => {
      term.dispose();
      terminalRef.current = null;
      fitAddonRef.current = null;
    };
  }, [sessionId, sendToPty, keybindingManager]);

  // Listen to PTY output
  useEffect(() => {
    let mounted = true;
    const unsubs: Array<() => void> = [];

    // Decode Base64 string to Uint8Array
    const decodeBase64 = (base64: string): Uint8Array => {
      const binaryString = atob(base64);
      const bytes = new Uint8Array(binaryString.length);
      for (let i = 0; i < binaryString.length; i++) {
        bytes[i] = binaryString.charCodeAt(i);
      }
      return bytes;
    };

    const setup = async () => {
      // Listen for PTY output (Base64 encoded)
      const u1 = await listen<PtyOutputEvent>("pty-output", (e) => {
        if (!mounted) return;
        const { session_id, data } = e.payload;
        if (session_id === sessionId && terminalRef.current) {
          const bytes = decodeBase64(data);
          terminalRef.current.write(bytes);
        }
      });
      if (!mounted) { u1(); return; }
      unsubs.push(u1);

      // Listen for PTY exit
      const u2 = await listen<{ session_id: string }>("pty-exit", (e) => {
        if (!mounted) return;
        const { session_id } = e.payload;
        if (session_id === sessionId && terminalRef.current) {
          terminalRef.current.writeln("\r\n[Process exited]");
        }
      });
      if (!mounted) { u2(); return; }
      unsubs.push(u2);
    };

    setup();

    return () => {
      mounted = false;
      unsubs.forEach(u => u());
    };
  }, [sessionId]);

  // Resize observer
  useEffect(() => {
    if (!containerRef.current) return;

    const observer = new ResizeObserver(() => {
      if (fitAddonRef.current && terminalRef.current) {
        fitAddonRef.current.fit();
        invoke("resize_pty", {
          sessionId,
          rows: terminalRef.current.rows,
          cols: terminalRef.current.cols,
        }).catch(console.error);
      }
    });

    observer.observe(containerRef.current);
    return () => observer.disconnect();
  }, [sessionId]);

  // Handle activation - fit and focus when becoming visible
  useEffect(() => {
    if (isActive && terminalRef.current && fitAddonRef.current) {
      requestAnimationFrame(() => {
        if (fitAddonRef.current && terminalRef.current) {
          fitAddonRef.current.fit();
          terminalRef.current.focus();
        }
      });
    }
  }, [isActive]);

  return (
    <div
      ref={containerRef}
      style={{
        width: "100%",
        height: "100%",
        overflow: "hidden",
        backgroundColor: "#1a1a2e",
      }}
    />
  );
}
