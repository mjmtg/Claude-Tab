/**
 * Services Module
 *
 * Provides typed interfaces and implementations for backend communication.
 * Extensions should import from this module rather than using Tauri directly.
 */

// Re-export interfaces
export type {
  IBackendService,
  IEventListener,
  ClaudeSession,
  SessionFilter,
  DirectoryPreference,
  ScanResult,
  SessionMessage,
  UnlistenFn,
  CoreEventHandler,
  PtyOutputHandler,
  PtyExitHandler,
} from "./types";

// Re-export implementations
export { TauriBackendService, tauriBackend } from "./TauriBackendService";
export { TauriEventListener, tauriEventListener } from "./TauriEventListener";
