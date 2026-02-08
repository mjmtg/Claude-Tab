import { ComponentType } from "react";
import type {
  IEventBus,
  IComponentRegistry,
  IKeybindingManager,
  IFocusManager,
  ISessionStateManager,
} from "./kernel";
import type { IBackendService, IEventListener } from "../services/types";

export interface ExtensionManifest {
  id: string;
  name: string;
  version: string;
  description?: string;
  dependencies?: string[];
}

/**
 * Context provided to extensions during activation.
 * Uses interfaces to allow for dependency injection and testing.
 */
export interface ExtensionContext {
  /** Event bus for pub/sub communication */
  eventBus: IEventBus;

  /** Component registry for slot-based UI */
  componentRegistry: IComponentRegistry;

  /** Keybinding manager for keyboard shortcuts */
  keybindingManager: IKeybindingManager;

  /** Backend service for IPC with Tauri */
  backend: IBackendService;

  /** Event listener for backend events */
  eventListener: IEventListener;

  /** Focus manager for window focus state and operations */
  focusManager: IFocusManager;

  /** Session state manager for tab switching state machine */
  sessionStateManager: ISessionStateManager;
}

export interface FrontendExtension {
  manifest: ExtensionManifest;
  activate(ctx: ExtensionContext): void | Promise<void>;
  deactivate?(): void | Promise<void>;
}

export interface SlotComponent {
  id: string;
  component: ComponentType<Record<string, unknown>>;
  priority?: number;
  extensionId: string;
}
