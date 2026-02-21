import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  Session,
  RegisteredProject,
  Config,
  StatusChangeEvent,
  PrDetectedEvent,
} from "../types";

// Session management
export async function createSession(
  projectPath: string,
  agent: string,
  branchName?: string
): Promise<Session> {
  return invoke("create_session", {
    projectPath,
    agent,
    branchName: branchName || null,
  });
}

export async function killSession(sessionId: string): Promise<void> {
  return invoke("kill_session", { sessionId });
}

export async function listSessions(): Promise<Session[]> {
  return invoke("list_sessions");
}

export async function getSession(
  sessionId: string
): Promise<Session | null> {
  return invoke("get_session", { sessionId });
}

// PTY interaction
export async function writeToPty(
  sessionId: string,
  data: number[]
): Promise<void> {
  return invoke("write_to_pty", { sessionId, data });
}

export async function resizePty(
  sessionId: string,
  cols: number,
  rows: number
): Promise<void> {
  return invoke("resize_pty", { sessionId, cols, rows });
}

// Project management
export async function listProjects(): Promise<RegisteredProject[]> {
  return invoke("list_projects");
}

export async function addProject(
  path: string
): Promise<RegisteredProject> {
  return invoke("add_project", { path });
}

export async function removeProject(path: string): Promise<void> {
  return invoke("remove_project", { path });
}

// Git
export async function createBranch(
  projectPath: string,
  branchName: string
): Promise<string> {
  return invoke("create_branch", { projectPath, branchName });
}

export async function getCurrentBranch(
  projectPath: string
): Promise<string> {
  return invoke("get_current_branch", { projectPath });
}

// Config
export async function getConfig(): Promise<Config> {
  return invoke("get_config");
}

export async function updateConfig(config: Config): Promise<void> {
  return invoke("update_config", { newConfig: config });
}

// URL
export async function openUrl(url: string): Promise<void> {
  return invoke("open_url", { url });
}

// Event listeners
export function onPtyOutput(
  sessionId: string,
  callback: (data: number[]) => void
): Promise<UnlistenFn> {
  return listen<number[]>(`pty-output-${sessionId}`, (event) => {
    callback(event.payload);
  });
}

export function onStatusChanged(
  callback: (event: StatusChangeEvent) => void
): Promise<UnlistenFn> {
  return listen<StatusChangeEvent>("session-status-changed", (e) => {
    callback(e.payload);
  });
}

export function onPrDetected(
  callback: (event: PrDetectedEvent) => void
): Promise<UnlistenFn> {
  return listen<PrDetectedEvent>("pr-detected", (e) => {
    callback(e.payload);
  });
}
