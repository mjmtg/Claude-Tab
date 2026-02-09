/**
 * Backend Service Interfaces
 *
 * These interfaces define the contract for all IPC communication
 * with the Tauri backend. By programming to these interfaces,
 * the frontend can be tested with mock implementations.
 */

import { SessionInfo, CreateSessionRequest } from "../types/session";
import { Profile, ProfileLaunchRequest } from "../types/profile";
import { CoreEvent, PtyOutputEvent, PtyExitEvent } from "../types/events";

// ============================================================================
// Claude Code Session Types
// ============================================================================

export interface ClaudeSession {
  session_id: string;
  project_path: string;
  encoded_path: string;
  jsonl_path: string;
  first_prompt: string | null;
  summary: string | null;
  message_count: number;
  created_at: string;
  modified_at: string;
  git_branch: string | null;
  file_mtime: number | null;
  indexed_at: string;
}

export interface SessionFilter {
  project_path?: string;
  pinned_only?: boolean;
  include_hidden?: boolean;
  search_query?: string;
  limit?: number;
  offset?: number;
}

export interface DirectoryPreference {
  project_path: string;
  pinned: boolean;
  hidden: boolean;
  display_name: string | null;
}

export interface ScanResult {
  sessions_found: number;
  sessions_added: number;
  sessions_updated: number;
  projects_scanned: number;
}

export interface MessagePayload {
  role?: string;
  content?: unknown;
  model?: string;
}

export interface SessionMessage {
  type: string;
  message?: MessagePayload;
  timestamp?: string;
  cwd?: string;
  gitBranch?: string;
  sessionId?: string;
  uuid?: string;
  parentUuid?: string;
  summary?: string;
}

// ============================================================================
// Backend Service Interface
// ============================================================================

/**
 * IBackendService provides typed methods for all Tauri commands.
 * This abstraction allows for mock implementations during testing.
 */
export interface IBackendService {
  // -------------------------------------------------------------------------
  // Session Management
  // -------------------------------------------------------------------------

  /** List all active sessions */
  listSessions(): Promise<SessionInfo[]>;

  /** Get the currently active session ID */
  getActiveSession(): Promise<string | null>;

  /** Set the active session */
  setActiveSession(sessionId: string): Promise<void>;

  /** Create a new session */
  createSession(request: CreateSessionRequest): Promise<SessionInfo>;

  /** Close a session */
  closeSession(sessionId: string): Promise<void>;

  /** Rename a session */
  renameSession(sessionId: string, title: string): Promise<void>;

  /** Fork the currently active session */
  forkActiveSession(sessionId: string): Promise<SessionInfo>;

  /** Resume a Claude Code session */
  resumeSession(claudeSessionId: string): Promise<SessionInfo>;

  /** Fork a Claude Code session (new session with same context) */
  forkSession(claudeSessionId: string): Promise<SessionInfo>;

  // -------------------------------------------------------------------------
  // PTY Operations
  // -------------------------------------------------------------------------

  /** Write data to a session's PTY */
  writeToPty(sessionId: string, data: number[]): Promise<void>;

  /** Resize a session's PTY */
  resizePty(sessionId: string, rows: number, cols: number): Promise<void>;

  // -------------------------------------------------------------------------
  // Configuration
  // -------------------------------------------------------------------------

  /** Get a configuration value */
  getConfigValue(key: string): Promise<unknown>;

  /** Set a configuration value */
  setConfigValue(key: string, value: unknown): Promise<void>;

  // -------------------------------------------------------------------------
  // Claude Code Session History
  // -------------------------------------------------------------------------

  /** Trigger a scan of ~/.claude/projects/ */
  scanClaudeSessions(): Promise<ScanResult>;

  /** List indexed Claude Code sessions */
  listClaudeSessions(filter?: SessionFilter): Promise<ClaudeSession[]>;

  /** Get a specific Claude Code session by ID */
  getClaudeSession(sessionId: string): Promise<ClaudeSession | null>;

  /** Get full conversation content from a session */
  getSessionContent(sessionId: string): Promise<SessionMessage[]>;

  /** Set directory preferences (pin/hide/rename) */
  setDirectoryPreference(
    projectPath: string,
    pinned?: boolean,
    hidden?: boolean,
    displayName?: string
  ): Promise<void>;

  /** Get all directory preferences */
  getDirectoryPreferences(): Promise<DirectoryPreference[]>;

  /** Remove directory preferences */
  removeDirectoryPreference(projectPath: string): Promise<void>;

  // -------------------------------------------------------------------------
  // Profiles
  // -------------------------------------------------------------------------

  /** List all profiles */
  listProfiles(): Promise<Profile[]>;

  /** Save (create or update) a profile */
  saveProfile(profile: Profile): Promise<void>;

  /** Delete a profile */
  deleteProfile(profileId: string): Promise<void>;

  /** Launch a profile as a new session */
  launchProfile(request: ProfileLaunchRequest): Promise<SessionInfo>;

  // -------------------------------------------------------------------------
  // Session Metadata
  // -------------------------------------------------------------------------

  /** Set session hidden state */
  setSessionHidden(sessionId: string, hidden: boolean): Promise<void>;

  /** Get the chain of sessions (previous_session_id links) */
  getSessionChain(sessionId: string): Promise<SessionInfo[]>;

  /** Trigger title generation for a session */
  triggerTitleGeneration(sessionId: string): Promise<void>;
}

// ============================================================================
// Event Listener Interface
// ============================================================================

export type UnlistenFn = () => void;

export type CoreEventHandler = (event: CoreEvent) => void;
export type PtyOutputHandler = (event: PtyOutputEvent) => void;
export type PtyExitHandler = (event: PtyExitEvent) => void;

/**
 * IEventListener provides typed subscriptions for backend events.
 * This abstraction decouples event subscription from Tauri specifics.
 */
export interface IEventListener {
  /** Subscribe to core events from the backend */
  onCoreEvent(handler: CoreEventHandler): Promise<UnlistenFn>;

  /** Subscribe to PTY output events */
  onPtyOutput(handler: PtyOutputHandler): Promise<UnlistenFn>;

  /** Subscribe to PTY exit events */
  onPtyExit(handler: PtyExitHandler): Promise<UnlistenFn>;

  /** Clean up all listeners */
  destroy(): void;
}
