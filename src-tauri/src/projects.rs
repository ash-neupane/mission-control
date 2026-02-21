use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use crate::session::AgentType;
use crate::status::unix_timestamp;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisteredProject {
    pub path: String,
    pub name: String,
    pub last_used: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectRegistry {
    pub projects: Vec<RegisteredProject>,
}

impl Default for ProjectRegistry {
    fn default() -> Self {
        ProjectRegistry {
            projects: Vec::new(),
        }
    }
}

impl ProjectRegistry {
    fn config_path() -> PathBuf {
        let mut path = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push(".cmux");
        fs::create_dir_all(&path).ok();
        path.push("projects.json");
        path
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            if let Ok(data) = fs::read_to_string(&path) {
                if let Ok(registry) = serde_json::from_str(&data) {
                    return registry;
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::config_path();
        let data = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize: {}", e))?;
        fs::write(path, data).map_err(|e| format!("Failed to write: {}", e))?;
        Ok(())
    }

    pub fn add_project(&mut self, path: &str) -> Result<RegisteredProject, String> {
        // Verify directory exists
        if !PathBuf::from(path).is_dir() {
            return Err(format!("Directory does not exist: {}", path));
        }

        // Check for duplicates
        if self.projects.iter().any(|p| p.path == path) {
            // Update last_used
            if let Some(p) = self.projects.iter_mut().find(|p| p.path == path) {
                p.last_used = unix_timestamp();
                let project = p.clone();
                self.save()?;
                return Ok(project);
            }
        }

        let name = PathBuf::from(path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string());

        let project = RegisteredProject {
            path: path.to_string(),
            name,
            last_used: unix_timestamp(),
        };

        self.projects.push(project.clone());
        self.save()?;
        Ok(project)
    }

    pub fn remove_project(&mut self, path: &str) -> Result<(), String> {
        self.projects.retain(|p| p.path != path);
        self.save()
    }

    pub fn touch_project(&mut self, path: &str) {
        if let Some(p) = self.projects.iter_mut().find(|p| p.path == path) {
            p.last_used = unix_timestamp();
            let _ = self.save();
        }
    }

    pub fn sorted_projects(&self) -> Vec<RegisteredProject> {
        let mut projects = self.projects.clone();
        projects.sort_by(|a, b| b.last_used.cmp(&a.last_used));
        projects
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub default_agent: AgentType,
    pub claude_command: String,
    pub codex_command: String,
    pub shell: String,
    pub notifications_enabled: bool,
    pub auto_branch: bool,
    pub branch_prefix: String,
    pub max_sessions: u8,
}

impl Default for Config {
    fn default() -> Self {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
        Config {
            default_agent: AgentType::Claude,
            claude_command: "claude".to_string(),
            codex_command: "codex".to_string(),
            shell,
            notifications_enabled: true,
            auto_branch: true,
            branch_prefix: "cmux/".to_string(),
            max_sessions: 9,
        }
    }
}

impl Config {
    fn config_path() -> PathBuf {
        let mut path = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push(".cmux");
        fs::create_dir_all(&path).ok();
        path.push("config.json");
        path
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            if let Ok(data) = fs::read_to_string(&path) {
                if let Ok(config) = serde_json::from_str(&data) {
                    return config;
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::config_path();
        let data = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize: {}", e))?;
        fs::write(path, data).map_err(|e| format!("Failed to write: {}", e))?;
        Ok(())
    }

    pub fn agent_command(&self, agent: &AgentType) -> (&str, Vec<&str>) {
        match agent {
            AgentType::Claude => (&self.claude_command, vec![]),
            AgentType::Codex => (&self.codex_command, vec![]),
            AgentType::Shell => (&self.shell, vec![]),
        }
    }
}

pub struct ProjectRegistryState(pub Mutex<ProjectRegistry>);

impl Default for ProjectRegistryState {
    fn default() -> Self {
        ProjectRegistryState(Mutex::new(ProjectRegistry::load()))
    }
}

pub struct ConfigState(pub Mutex<Config>);

impl Default for ConfigState {
    fn default() -> Self {
        ConfigState(Mutex::new(Config::load()))
    }
}
