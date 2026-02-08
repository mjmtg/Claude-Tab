import type { IKeybindingManager, KeybindingDefinition } from "../types/kernel";

const STORAGE_KEY = "claude-tabs-keybindings";

// Re-export KeybindingDefinition as Keybinding for backwards compatibility
export type Keybinding = KeybindingDefinition;

export class KeybindingManager implements IKeybindingManager {
  private bindings = new Map<string, Keybinding>();
  private active = true;
  private customKeys: Record<string, string> = {};

  constructor() {
    this.handleKeyDown = this.handleKeyDown.bind(this);
    window.addEventListener("keydown", this.handleKeyDown);
    this.loadCustomKeys();
  }

  private loadCustomKeys(): void {
    try {
      const stored = localStorage.getItem(STORAGE_KEY);
      if (stored) {
        this.customKeys = JSON.parse(stored);
      }
    } catch {
      this.customKeys = {};
    }
  }

  private saveCustomKeys(): void {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(this.customKeys));
  }

  register(binding: Keybinding): () => void {
    const defaultKeys = binding.keys;
    const customKey = this.customKeys[binding.id];
    const stored: Keybinding = {
      ...binding,
      defaultKeys,
      keys: customKey ?? defaultKeys,
    };
    this.bindings.set(binding.id, stored);
    return () => {
      this.bindings.delete(binding.id);
    };
  }

  unregister(id: string): void {
    this.bindings.delete(id);
  }

  updateKeys(id: string, newKeys: string): void {
    const binding = this.bindings.get(id);
    if (!binding) return;
    binding.keys = newKeys;
    this.customKeys[id] = newKeys;
    this.saveCustomKeys();
  }

  resetKeys(id: string): void {
    const binding = this.bindings.get(id);
    if (!binding || !binding.defaultKeys) return;
    binding.keys = binding.defaultKeys;
    delete this.customKeys[id];
    this.saveCustomKeys();
  }

  setActive(active: boolean): void {
    this.active = active;
  }

  getAll(): Keybinding[] {
    return Array.from(this.bindings.values());
  }

  hasBinding(keyStr: string): boolean {
    for (const [, binding] of this.bindings) {
      if (binding.keys === keyStr) {
        return true;
      }
    }
    return false;
  }

  eventToKeyString(e: KeyboardEvent): string {
    const parts: string[] = [];
    if (e.metaKey) parts.push("Cmd");
    if (e.ctrlKey) parts.push("Ctrl");
    if (e.altKey) parts.push("Alt");
    if (e.shiftKey) parts.push("Shift");

    // On macOS, Alt+key produces special characters (e.g., Alt+T = †)
    // Use e.code to get the physical key when Alt is pressed
    let key: string;
    if (e.altKey && e.code.startsWith("Key")) {
      // e.code is like "KeyT" -> extract "T"
      key = e.code.slice(3);
    } else if (e.altKey && e.code.startsWith("Digit")) {
      // e.code is like "Digit1" -> extract "1"
      key = e.code.slice(5);
    } else {
      key = e.key.length === 1 ? e.key.toUpperCase() : e.key;
    }

    if (!["Meta", "Control", "Alt", "Shift"].includes(e.key)) {
      parts.push(key);
    }

    return parts.join("+");
  }

  private handleKeyDown(e: KeyboardEvent): void {
    if (!this.active) return;

    const keyStr = this.eventToKeyString(e);

    for (const [, binding] of this.bindings) {
      if (binding.keys === keyStr) {
        e.preventDefault();
        e.stopPropagation();
        binding.handler();
        return;
      }
    }
  }

  destroy(): void {
    window.removeEventListener("keydown", this.handleKeyDown);
    this.bindings.clear();
  }
}
