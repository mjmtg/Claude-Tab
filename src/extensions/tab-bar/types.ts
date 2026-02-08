// Types for Claude Code session history

export interface ClaudeSession {
  session_id: string;
  project_path: string;
  jsonl_path: string;
  first_prompt: string | null;
  summary: string | null;  // Claude's auto-generated title
  message_count: number;
  created_at: string;
  modified_at: string;
  git_branch: string | null;
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
