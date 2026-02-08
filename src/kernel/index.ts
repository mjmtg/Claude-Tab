/**
 * Kernel Module
 *
 * Central export point for all kernel components.
 * This module provides both interfaces (for DI) and implementations.
 *
 * Extensions should import from this module:
 * ```ts
 * import { EventBus, type IEventBus } from "../kernel";
 * ```
 */

// ============================================================================
// Re-export Interfaces
// ============================================================================

export type {
  IEventBus,
  IComponentRegistry,
  IKeybindingManager,
  IConfigProvider,
  KeybindingDefinition,
} from "../types/kernel";

// ============================================================================
// Re-export Implementations
// ============================================================================

export { EventBus } from "./EventBus";
export { ComponentRegistry } from "./ComponentRegistry";
export { KeybindingManager, type Keybinding } from "./KeybindingManager";
export { ConfigProvider, useConfig } from "./ConfigProvider";
export { ExtensionHost, type ExtensionHostConfig } from "./ExtensionHost";
export { EventBusProvider, useEventBus, useEventBusOptional } from "./EventBusContext";
export {
  createStateStore,
  useStateStore,
  useStoreSelector,
  type StateStore,
} from "./StateStore";
