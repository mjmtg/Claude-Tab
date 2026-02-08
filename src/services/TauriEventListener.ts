/**
 * Tauri Event Listener Implementation
 *
 * Implements IEventListener using Tauri's event API.
 * This is the production implementation used in the actual app.
 */

import { listen } from "@tauri-apps/api/event";
import {
  IEventListener,
  UnlistenFn,
  CoreEventHandler,
  PtyOutputHandler,
  PtyExitHandler,
} from "./types";
import { CoreEvent, PtyOutputEvent, PtyExitEvent } from "../types/events";

export class TauriEventListener implements IEventListener {
  private unlisteners: UnlistenFn[] = [];

  async onCoreEvent(handler: CoreEventHandler): Promise<UnlistenFn> {
    const unlisten = await listen<CoreEvent>("core-event", (event) => {
      handler(event.payload);
    });
    this.unlisteners.push(unlisten);
    return unlisten;
  }

  async onPtyOutput(handler: PtyOutputHandler): Promise<UnlistenFn> {
    const unlisten = await listen<PtyOutputEvent>("pty-output", (event) => {
      handler(event.payload);
    });
    this.unlisteners.push(unlisten);
    return unlisten;
  }

  async onPtyExit(handler: PtyExitHandler): Promise<UnlistenFn> {
    const unlisten = await listen<PtyExitEvent>("pty-exit", (event) => {
      handler(event.payload);
    });
    this.unlisteners.push(unlisten);
    return unlisten;
  }

  destroy(): void {
    for (const unlisten of this.unlisteners) {
      unlisten();
    }
    this.unlisteners = [];
  }
}

/**
 * Default singleton instance for convenience.
 * Extensions can import this directly if they don't need DI.
 */
export const tauriEventListener = new TauriEventListener();
