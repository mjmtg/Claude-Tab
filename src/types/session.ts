export interface SessionInfo {
  id: string;
  provider_id: string;
  state: string;
  title: string;
  working_directory: string | null;
  subtitle: string | null;
  tags: string[] | null;
  summary: string | null;
}

export interface CreateSessionRequest {
  provider_id: string;
  working_directory?: string;
  title?: string;
  resume_claude_session_id?: string;
  fork?: boolean;
  initial_prompt?: string;
  mcp_config_path?: string;
  allowed_tools?: string[];
  model?: string;
  system_prompt?: string;
}
