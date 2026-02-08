/**
 * Tauri Backend Service Implementation
 *
 * Implements IBackendService using Tauri's invoke API.
 * This is the production implementation used in the actual app.
 */

import { invoke } from "@tauri-apps/api/core";
import {
  IBackendService,
  ClaudeSession,
  SessionFilter,
  DirectoryPreference,
  ScanResult,
  SessionMessage,
} from "./types";
import { SessionInfo, CreateSessionRequest } from "../types/session";
import { Profile, ProfileLaunchRequest } from "../types/profile";

export class TauriBackendService implements IBackendService {
  // -------------------------------------------------------------------------
  // Session Management
  // -------------------------------------------------------------------------

  async listSessions(): Promise<SessionInfo[]> {
    return invoke<SessionInfo[]>("list_sessions");
  }

  async getActiveSession(): Promise<string | null> {
    return invoke<string | null>("get_active_session");
  }

  async setActiveSession(sessionId: string): Promise<void> {
    await invoke("set_active_session", { sessionId });
  }

  async createSession(request: CreateSessionRequest): Promise<SessionInfo> {
    return invoke<SessionInfo>("create_session", { request });
  }

  async closeSession(sessionId: string): Promise<void> {
    await invoke("close_session", { sessionId });
  }

  async renameSession(sessionId: string, title: string): Promise<void> {
    await invoke("rename_session", { sessionId, title });
  }

  async forkActiveSession(sessionId: string): Promise<SessionInfo> {
    return invoke<SessionInfo>("fork_active_session", { sessionId });
  }

  async resumeSession(claudeSessionId: string): Promise<SessionInfo> {
    return invoke<SessionInfo>("resume_session", { claudeSessionId });
  }

  async forkSession(claudeSessionId: string): Promise<SessionInfo> {
    return invoke<SessionInfo>("fork_session", { claudeSessionId });
  }

  // -------------------------------------------------------------------------
  // PTY Operations
  // -------------------------------------------------------------------------

  async writeToPty(sessionId: string, data: number[]): Promise<void> {
    await invoke("write_to_pty", { sessionId, data });
  }

  async resizePty(sessionId: string, rows: number, cols: number): Promise<void> {
    await invoke("resize_pty", { sessionId, rows, cols });
  }

  // -------------------------------------------------------------------------
  // Configuration
  // -------------------------------------------------------------------------

  async getConfigValue(key: string): Promise<unknown> {
    return invoke("get_config_value", { key });
  }

  async setConfigValue(key: string, value: unknown): Promise<void> {
    await invoke("set_config_value", { key, value });
  }

  // -------------------------------------------------------------------------
  // Claude Code Session History
  // -------------------------------------------------------------------------

  async scanClaudeSessions(): Promise<ScanResult> {
    return invoke<ScanResult>("scan_claude_sessions");
  }

  async listClaudeSessions(filter?: SessionFilter): Promise<ClaudeSession[]> {
    return invoke<ClaudeSession[]>("list_claude_sessions", { filter });
  }

  async getClaudeSession(sessionId: string): Promise<ClaudeSession | null> {
    return invoke<ClaudeSession | null>("get_claude_session", { sessionId });
  }

  async getSessionContent(sessionId: string): Promise<SessionMessage[]> {
    return invoke<SessionMessage[]>("get_session_content", { sessionId });
  }

  async setDirectoryPreference(
    projectPath: string,
    pinned?: boolean,
    hidden?: boolean,
    displayName?: string
  ): Promise<void> {
    await invoke("set_directory_preference", { projectPath, pinned, hidden, displayName });
  }

  async getDirectoryPreferences(): Promise<DirectoryPreference[]> {
    return invoke<DirectoryPreference[]>("get_directory_preferences");
  }

  async removeDirectoryPreference(projectPath: string): Promise<void> {
    await invoke("remove_directory_preference", { projectPath });
  }

  // -------------------------------------------------------------------------
  // Profiles
  // -------------------------------------------------------------------------

  async listProfiles(): Promise<Profile[]> {
    return invoke<Profile[]>("list_profiles");
  }

  async saveProfile(profile: Profile): Promise<void> {
    await invoke("save_profile", { profile });
  }

  async deleteProfile(profileId: string): Promise<void> {
    await invoke("delete_profile", { profileId });
  }

  async launchProfile(request: ProfileLaunchRequest): Promise<SessionInfo> {
    return invoke<SessionInfo>("launch_profile", { request });
  }
}

/**
 * Default singleton instance for convenience.
 * Extensions can import this directly if they don't need DI.
 */
export const tauriBackend = new TauriBackendService();
