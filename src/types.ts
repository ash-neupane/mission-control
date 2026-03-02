export type AgentType = "Claude" | "Codex" | "Shell";

export type SessionStatus =
  | "Empty"
  | "Working"
  | "NeedsInput"
  | "PrReady"
  | "Stuck"
  | "Done";

export interface Session {
  id: string;
  number: number;
  name: string;
  project_path: string;
  project_name: string;
  working_dir: string;
  agent: AgentType;
  status: SessionStatus;
  branch: string | null;
  pr_url: string | null;
  started_at: number;
  needs_attention_since: number | null;
}

export interface RegisteredProject {
  path: string;
  name: string;
  last_used: number;
}

export interface Config {
  default_agent: AgentType;
  claude_command: string;
  codex_command: string;
  shell: string;
  notifications_enabled: boolean;
  auto_branch: boolean;
  branch_prefix: string;
  max_sessions: number;
}

export type ViewMode = "overview" | "focus";

export interface StatusChangeEvent {
  session_id: string;
  new_status: SessionStatus;
  name: string | null;
  needs_attention_since: number | null;
}

export interface PrDetectedEvent {
  session_id: string;
  url: string;
}
