// Extension creation
export { createExtension } from "./createExtension";

// React hooks
export { useEvent } from "./useEvent";
export { useSession } from "./useSession";
export { useConfig } from "../kernel/ConfigProvider";

// Re-export kernel interfaces for extension use
export type {
  IEventBus,
  IComponentRegistry,
  IKeybindingManager,
  IConfigProvider,
  IFocusManager,
  ISessionStateManager,
  KeybindingDefinition,
  FocusState,
  SwitchState,
  ToastState,
} from "../types/kernel";

// Re-export service interfaces and singletons
export type {
  IBackendService,
  IEventListener,
  ClaudeSession,
  SessionFilter,
  DirectoryPreference,
} from "../services/types";
export { tauriBackend, tauriEventListener } from "../services";
