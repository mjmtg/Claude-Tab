import { listen, emit } from "@tauri-apps/api/event";
import { CoreEvent, EventHandler, UnsubscribeFn } from "../types/events";
import type { IEventBus } from "../types/kernel";

type Middleware = (event: CoreEvent) => boolean;

/**
 * Topic index for O(1) event dispatch
 */
class TopicIndex {
  // Exact topic matches
  private exact = new Map<string, Set<number>>();
  // Wildcard patterns: prefix.* (single level)
  private wildcardSingle = new Map<string, Set<number>>();
  // Wildcard patterns: prefix.** (deep)
  private wildcardDeep = new Map<string, Set<number>>();
  // Global wildcard (*) handlers
  private global = new Set<number>();

  add(id: number, pattern: string): void {
    if (pattern === "*") {
      this.global.add(id);
    } else if (pattern.endsWith(".**")) {
      const prefix = pattern.slice(0, -3);
      if (!this.wildcardDeep.has(prefix)) {
        this.wildcardDeep.set(prefix, new Set());
      }
      this.wildcardDeep.get(prefix)!.add(id);
    } else if (pattern.endsWith(".*")) {
      const prefix = pattern.slice(0, -2);
      if (!this.wildcardSingle.has(prefix)) {
        this.wildcardSingle.set(prefix, new Set());
      }
      this.wildcardSingle.get(prefix)!.add(id);
    } else {
      if (!this.exact.has(pattern)) {
        this.exact.set(pattern, new Set());
      }
      this.exact.get(pattern)!.add(id);
    }
  }

  remove(id: number, pattern: string): void {
    if (pattern === "*") {
      this.global.delete(id);
    } else if (pattern.endsWith(".**")) {
      const prefix = pattern.slice(0, -3);
      this.wildcardDeep.get(prefix)?.delete(id);
    } else if (pattern.endsWith(".*")) {
      const prefix = pattern.slice(0, -2);
      this.wildcardSingle.get(prefix)?.delete(id);
    } else {
      this.exact.get(pattern)?.delete(id);
    }
  }

  getMatchingHandlers(topic: string): Set<number> {
    const result = new Set<number>();

    // Global handlers
    for (const id of this.global) {
      result.add(id);
    }

    // Exact match
    const exactHandlers = this.exact.get(topic);
    if (exactHandlers) {
      for (const id of exactHandlers) {
        result.add(id);
      }
    }

    // Check wildcard patterns
    const dotPos = topic.lastIndexOf(".");
    if (dotPos !== -1) {
      const prefix = topic.slice(0, dotPos);

      // Single wildcard: prefix.* matches prefix.X but not prefix.X.Y
      if (!topic.slice(dotPos + 1).includes(".")) {
        const singleHandlers = this.wildcardSingle.get(prefix);
        if (singleHandlers) {
          for (const id of singleHandlers) {
            result.add(id);
          }
        }
      }

      // Deep wildcard: check all prefixes
      let current = topic;
      while (true) {
        const pos = current.lastIndexOf(".");
        if (pos === -1) break;
        const checkPrefix = current.slice(0, pos);
        const deepHandlers = this.wildcardDeep.get(checkPrefix);
        if (deepHandlers) {
          for (const id of deepHandlers) {
            result.add(id);
          }
        }
        current = checkPrefix;
      }
    }

    return result;
  }
}

export class EventBus implements IEventBus {
  private handlers = new Map<number, { pattern: string; handler: EventHandler }>();
  private middleware: Middleware[] = [];
  private nextId = 1;
  private tauriUnlisteners: Array<() => void> = [];
  private topicIndex = new TopicIndex();

  async init(): Promise<void> {
    console.log("[EventBus] Initializing...");
    const unlisten1 = await listen<CoreEvent>("core-event", (e) => {
      this.dispatch(e.payload);
    });
    this.tauriUnlisteners.push(unlisten1);
    console.log("[EventBus] Initialized successfully");
  }

  subscribe(pattern: string, handler: EventHandler): UnsubscribeFn {
    const id = this.nextId++;
    this.handlers.set(id, { pattern, handler });
    this.topicIndex.add(id, pattern);
    return () => {
      this.handlers.delete(id);
      this.topicIndex.remove(id, pattern);
    };
  }

  publish(event: CoreEvent): void {
    for (const mw of this.middleware) {
      if (!mw(event)) return;
    }
    this.dispatch(event);
  }

  addMiddleware(mw: Middleware): void {
    this.middleware.push(mw);
  }

  async emitToBackend(topic: string, payload: Record<string, unknown>): Promise<void> {
    await emit("frontend-event", { topic, payload });
  }

  private dispatch(event: CoreEvent): void {
    // Use topic index for O(1) handler lookup
    const matchingIds = this.topicIndex.getMatchingHandlers(event.topic);
    for (const id of matchingIds) {
      const entry = this.handlers.get(id);
      if (entry) {
        try {
          entry.handler(event);
        } catch (err) {
          console.error(`[EventBus] Handler error for ${event.topic}:`, err);
        }
      }
    }
  }

  destroy(): void {
    for (const unlisten of this.tauriUnlisteners) {
      unlisten();
    }
    this.tauriUnlisteners = [];
    this.handlers.clear();
    this.middleware = [];
    this.topicIndex = new TopicIndex();
  }
}
