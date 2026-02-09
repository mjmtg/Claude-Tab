import type { IKeybindingManager, KeybindingDefinition } from "../types/kernel";

const STORAGE_KEY = "claude-tabs-keybindings";

export class KeybindingManager implements IKeybindingManager {
  private bindings = new Map<string, KeybindingDefinition>();
  /** Reverse index: key string -> binding ID for O(1) lookup */
  private keyIndex = new Map<string, string>();
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

  register(binding: KeybindingDefinition): () => void {
    const defaultKeys = binding.keys;
    const customKey = this.customKeys[binding.id];
    const stored: KeybindingDefinition = {
      ...binding,
      defaultKeys,
      keys: customKey ?? defaultKeys,
    };
    this.bindings.set(binding.id, stored);
    this.keyIndex.set(stored.keys, binding.id);
    return () => {
      const current = this.bindings.get(binding.id);
      if (current) {
        this.keyIndex.delete(current.keys);
      }
      this.bindings.delete(binding.id);
    };
  }

  unregister(id: string): void {
    const binding = this.bindings.get(id);
    if (binding) {
      this.keyIndex.delete(binding.keys);
    }
    this.bindings.delete(id);
  }

  updateKeys(id: string, newKeys: string): void {
    const binding = this.bindings.get(id);
    if (!binding) return;
    this.keyIndex.delete(binding.keys);
    binding.keys = newKeys;
    this.keyIndex.set(newKeys, id);
    this.customKeys[id] = newKeys;
    this.saveCustomKeys();
  }

  resetKeys(id: string): void {
    const binding = this.bindings.get(id);
    if (!binding || !binding.defaultKeys) return;
    this.keyIndex.delete(binding.keys);
    binding.keys = binding.defaultKeys;
    this.keyIndex.set(binding.defaultKeys, id);
    delete this.customKeys[id];
    this.saveCustomKeys();
  }

  setActive(active: boolean): void {
    this.active = active;
  }

  getAll(): KeybindingDefinition[] {
    return Array.from(this.bindings.values());
  }

  hasBinding(keyStr: string): boolean {
    return this.keyIndex.has(keyStr);
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

    // Let Escape pass through to the terminal when no binding uses it
    if (keyStr === "Escape" && !this.keyIndex.has("Escape")) {
      return;
    }

    const bindingId = this.keyIndex.get(keyStr);
    if (bindingId) {
      const binding = this.bindings.get(bindingId);
      if (binding) {
        e.preventDefault();
        e.stopPropagation();
        binding.handler();
      }
    }
  }

  destroy(): void {
    window.removeEventListener("keydown", this.handleKeyDown);
    this.bindings.clear();
    this.keyIndex.clear();
  }
}
