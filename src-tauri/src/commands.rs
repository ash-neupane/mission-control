use std::sync::{Mutex, MutexGuard};
use tauri::{AppHandle, State};

use crate::git;
use crate::naming;
use crate::projects::{Config, ConfigState, ProjectRegistryState, RegisteredProject};
use crate::pty::PtyPoolState;
use crate::session::{AgentType, Session, SessionManagerState, SessionStatus};
use crate::status::unix_timestamp;

/// Acquire a mutex lock, returning a user-facing error on poison.
fn lock_or_err<T>(mutex: &Mutex<T>) -> Result<MutexGuard<'_, T>, String> {
    mutex
        .lock()
        .map_err(|_| "Internal error: lock poisoned".to_string())
}

#[tauri::command]
pub fn create_session(
    app_handle: AppHandle,
    session_manager: State<SessionManagerState>,
    pty_pool: State<PtyPoolState>,
    project_registry: State<ProjectRegistryState>,
    config: State<ConfigState>,
    project_path: String,
    agent: String,
    branch_name: Option<String>,
) -> Result<Session, String> {
    let mut manager = lock_or_err(&session_manager.0)?;

    let number = manager
        .next_available_number()
        .ok_or("Maximum sessions (9) reached")?;

    let cfg = lock_or_err(&config.0)?;

    let agent_type = match agent.to_lowercase().as_str() {
        "claude" => AgentType::Claude,
        "codex" => AgentType::Codex,
        _ => AgentType::Shell,
    };

    let project_name = std::path::Path::new(&project_path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| project_path.clone());

    // Handle branch creation if auto_branch is enabled
    let branch = if cfg.auto_branch && agent_type != AgentType::Shell {
        let branch = branch_name
            .unwrap_or_else(|| format!("{}session-{}", cfg.branch_prefix, number));
        match git::create_new_branch(&project_path, &branch) {
            Ok(b) => Some(b),
            Err(e) => {
                log::warn!("Failed to create branch: {}", e);
                None
            }
        }
    } else {
        branch_name
    };

    // Determine session name — pass branch_prefix so naming can strip it
    let name = branch
        .as_deref()
        .and_then(|b| naming::name_from_branch(b, &cfg.branch_prefix))
        .unwrap_or_else(|| naming::fallback_name(&project_name, number));

    let session_id = uuid::Uuid::new_v4().to_string();

    let session = Session {
        id: session_id.clone(),
        number,
        name,
        project_path: project_path.clone(),
        project_name: project_name.clone(),
        working_dir: project_path.clone(),
        agent: agent_type.clone(),
        status: SessionStatus::Empty,
        branch: branch.clone(),
        pr_url: None,
        started_at: unix_timestamp(),
        tokens_used: None,
        last_output_preview: String::new(),
        needs_attention_since: None,
    };

    // Spawn PTY
    let (command, args) = cfg.agent_command(&agent_type);
    let command = command.to_string();
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();

    let mut pool = lock_or_err(&pty_pool.0)?;

    let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    pool.spawn(
        &session_id,
        &project_path,
        &command,
        &args_refs,
        app_handle.clone(),
    )?;

    manager.add_session(session.clone());

    // Update project registry (best-effort)
    if let Ok(mut registry) = project_registry.0.lock() {
        registry.touch_project(&project_path);
    }

    Ok(session)
}

#[tauri::command]
pub fn kill_session(
    session_manager: State<SessionManagerState>,
    pty_pool: State<PtyPoolState>,
    session_id: String,
) -> Result<(), String> {
    let mut pool = lock_or_err(&pty_pool.0)?;
    pool.kill(&session_id)?;

    let mut manager = lock_or_err(&session_manager.0)?;
    manager.remove_session(&session_id);

    Ok(())
}

#[tauri::command]
pub fn list_sessions(
    session_manager: State<SessionManagerState>,
) -> Result<Vec<Session>, String> {
    let manager = lock_or_err(&session_manager.0)?;
    Ok(manager.list_sessions())
}

#[tauri::command]
pub fn get_session(
    session_manager: State<SessionManagerState>,
    session_id: String,
) -> Result<Option<Session>, String> {
    let manager = lock_or_err(&session_manager.0)?;
    Ok(manager.get_session(&session_id).cloned())
}

#[tauri::command]
pub fn write_to_pty(
    pty_pool: State<PtyPoolState>,
    session_id: String,
    data: Vec<u8>,
) -> Result<(), String> {
    let pool = lock_or_err(&pty_pool.0)?;
    pool.write(&session_id, &data)
}

#[tauri::command]
pub fn resize_pty(
    pty_pool: State<PtyPoolState>,
    session_id: String,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    let pool = lock_or_err(&pty_pool.0)?;
    pool.resize(&session_id, cols, rows)
}

#[tauri::command]
pub fn list_projects(
    project_registry: State<ProjectRegistryState>,
) -> Result<Vec<RegisteredProject>, String> {
    let registry = lock_or_err(&project_registry.0)?;
    Ok(registry.sorted_projects())
}

#[tauri::command]
pub fn add_project(
    project_registry: State<ProjectRegistryState>,
    path: String,
) -> Result<RegisteredProject, String> {
    let mut registry = lock_or_err(&project_registry.0)?;
    registry.add_project(&path)
}

#[tauri::command]
pub fn remove_project(
    project_registry: State<ProjectRegistryState>,
    path: String,
) -> Result<(), String> {
    let mut registry = lock_or_err(&project_registry.0)?;
    registry.remove_project(&path)
}

#[tauri::command]
pub fn create_branch(project_path: String, branch_name: String) -> Result<String, String> {
    git::create_new_branch(&project_path, &branch_name)
}

#[tauri::command]
pub fn get_current_branch(project_path: String) -> Result<String, String> {
    git::current_branch(&project_path)
}

#[tauri::command]
pub fn get_config(config: State<ConfigState>) -> Result<Config, String> {
    let cfg = lock_or_err(&config.0)?;
    Ok(cfg.clone())
}

#[tauri::command]
pub fn update_config(config: State<ConfigState>, new_config: Config) -> Result<(), String> {
    let mut cfg = lock_or_err(&config.0)?;
    new_config.save()?;
    *cfg = new_config;
    Ok(())
}

#[tauri::command]
pub fn open_url(url: String) -> Result<(), String> {
    open::that(&url).map_err(|e| format!("Failed to open URL: {}", e))
}
