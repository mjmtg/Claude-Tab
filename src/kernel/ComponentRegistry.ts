import { SlotComponent } from "../types/extension";
import { SlotName } from "../types/slots";
import type { IComponentRegistry } from "../types/kernel";

type RegistryListener = () => void;

export class ComponentRegistry implements IComponentRegistry {
  private slots = new Map<string, SlotComponent[]>();
  private listeners: RegistryListener[] = [];

  register(slot: SlotName, component: SlotComponent): void {
    const existing = this.slots.get(slot) ?? [];
    existing.push(component);
    existing.sort((a, b) => (b.priority ?? 0) - (a.priority ?? 0));
    this.slots.set(slot, existing);
    this.notify();
  }

  unregister(slot: SlotName, componentId: string): void {
    const existing = this.slots.get(slot) ?? [];
    this.slots.set(
      slot,
      existing.filter((c) => c.id !== componentId)
    );
    this.notify();
  }

  getComponents(slot: SlotName): SlotComponent[] {
    return this.slots.get(slot) ?? [];
  }

  subscribe(listener: RegistryListener): () => void {
    this.listeners.push(listener);
    return () => {
      this.listeners = this.listeners.filter((l) => l !== listener);
    };
  }

  private notify(): void {
    for (const listener of this.listeners) {
      listener();
    }
  }
}
