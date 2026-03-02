use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AgentType {
    Claude,
    Codex,
    Shell,
}

impl std::fmt::Display for AgentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentType::Claude => write!(f, "Claude"),
            AgentType::Codex => write!(f, "Codex"),
            AgentType::Shell => write!(f, "Shell"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SessionStatus {
    Empty,
    Working,
    NeedsInput,
    PrReady,
    Stuck,
    Done,
}

impl SessionStatus {
    pub fn sort_priority(&self) -> u8 {
        match self {
            SessionStatus::NeedsInput => 0,
            SessionStatus::Stuck => 1,
            SessionStatus::PrReady => 2,
            SessionStatus::Working => 3,
            SessionStatus::Done => 4,
            SessionStatus::Empty => 5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub number: u8,
    pub name: String,
    pub project_path: String,
    pub project_name: String,
    pub working_dir: String,
    pub agent: AgentType,
    pub status: SessionStatus,
    pub branch: Option<String>,
    pub pr_url: Option<String>,
    pub started_at: u64,
    pub tokens_used: Option<u64>,
    pub last_output_preview: String,
    pub needs_attention_since: Option<u64>,
}

pub struct SessionManager {
    pub sessions: HashMap<String, Session>,
    // Track which session numbers (1-9) are in use
    pub used_numbers: [bool; 9],
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionManager {
    pub fn new() -> Self {
        SessionManager {
            sessions: HashMap::new(),
            used_numbers: [false; 9],
        }
    }

    pub fn next_available_number(&self) -> Option<u8> {
        for i in 0..9 {
            if !self.used_numbers[i] {
                return Some((i + 1) as u8);
            }
        }
        None
    }

    pub fn add_session(&mut self, session: Session) {
        let num = session.number;
        assert!((1..=9).contains(&num), "session number must be 1-9, got {num}");
        self.used_numbers[(num - 1) as usize] = true;
        self.sessions.insert(session.id.clone(), session);
    }

    pub fn remove_session(&mut self, id: &str) -> Option<Session> {
        if let Some(session) = self.sessions.remove(id) {
            self.used_numbers[(session.number - 1) as usize] = false;
            Some(session)
        } else {
            None
        }
    }

    pub fn get_session(&self, id: &str) -> Option<&Session> {
        self.sessions.get(id)
    }

    pub fn get_session_mut(&mut self, id: &str) -> Option<&mut Session> {
        self.sessions.get_mut(id)
    }

    pub fn list_sessions(&self) -> Vec<Session> {
        let mut sessions: Vec<Session> = self.sessions.values().cloned().collect();
        sessions.sort_by(|a, b| {
            a.status
                .sort_priority()
                .cmp(&b.status.sort_priority())
                .then_with(|| {
                    // Within same priority, sort NeedsInput by oldest first
                    if a.status == SessionStatus::NeedsInput {
                        a.needs_attention_since.cmp(&b.needs_attention_since)
                    } else {
                        a.number.cmp(&b.number)
                    }
                })
        });
        sessions
    }

    pub fn get_session_by_number(&self, number: u8) -> Option<&Session> {
        self.sessions.values().find(|s| s.number == number)
    }
}

pub struct SessionManagerState(pub Mutex<SessionManager>);

impl Default for SessionManagerState {
    fn default() -> Self {
        SessionManagerState(Mutex::new(SessionManager::new()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_session(id: &str, number: u8, status: SessionStatus) -> Session {
        Session {
            id: id.to_string(),
            number,
            name: format!("session-{}", number),
            project_path: "/tmp/test".to_string(),
            project_name: "test".to_string(),
            working_dir: "/tmp/test".to_string(),
            agent: AgentType::Claude,
            status,
            branch: None,
            pr_url: None,
            started_at: 1000,
            tokens_used: None,
            last_output_preview: String::new(),
            needs_attention_since: None,
        }
    }

    #[test]
    fn test_next_available_number() {
        let mut manager = SessionManager::new();
        assert_eq!(manager.next_available_number(), Some(1));

        manager.add_session(make_session("a", 1, SessionStatus::Working));
        assert_eq!(manager.next_available_number(), Some(2));

        manager.add_session(make_session("b", 2, SessionStatus::Working));
        assert_eq!(manager.next_available_number(), Some(3));

        // Fill all 9 slots
        for i in 3..=9 {
            manager.add_session(make_session(
                &format!("s{}", i),
                i,
                SessionStatus::Working,
            ));
        }
        assert_eq!(manager.next_available_number(), None);
    }

    #[test]
    fn test_remove_session_frees_number() {
        let mut manager = SessionManager::new();
        manager.add_session(make_session("a", 1, SessionStatus::Working));
        manager.add_session(make_session("b", 2, SessionStatus::Working));

        manager.remove_session("a");
        assert_eq!(manager.next_available_number(), Some(1));
        assert!(manager.get_session("a").is_none());
    }

    #[test]
    fn test_list_sessions_sorted_by_priority() {
        let mut manager = SessionManager::new();
        manager.add_session(make_session("a", 1, SessionStatus::Working));
        manager.add_session(make_session("b", 2, SessionStatus::NeedsInput));
        manager.add_session(make_session("c", 3, SessionStatus::Done));

        let sessions = manager.list_sessions();
        assert_eq!(sessions[0].status, SessionStatus::NeedsInput);
        assert_eq!(sessions[1].status, SessionStatus::Working);
        assert_eq!(sessions[2].status, SessionStatus::Done);
    }

    #[test]
    fn test_get_session_by_number() {
        let mut manager = SessionManager::new();
        manager.add_session(make_session("a", 3, SessionStatus::Working));

        assert!(manager.get_session_by_number(3).is_some());
        assert!(manager.get_session_by_number(1).is_none());
    }

    #[test]
    fn test_status_sort_priority() {
        assert!(SessionStatus::NeedsInput.sort_priority() < SessionStatus::Working.sort_priority());
        assert!(SessionStatus::Stuck.sort_priority() < SessionStatus::PrReady.sort_priority());
        assert!(SessionStatus::Working.sort_priority() < SessionStatus::Done.sort_priority());
    }

    #[test]
    fn test_agent_type_display() {
        assert_eq!(format!("{}", AgentType::Claude), "Claude");
        assert_eq!(format!("{}", AgentType::Codex), "Codex");
        assert_eq!(format!("{}", AgentType::Shell), "Shell");
    }

    #[test]
    fn test_list_sessions_needs_input_sorted_by_attention_time() {
        let mut manager = SessionManager::new();

        let mut s1 = make_session("a", 1, SessionStatus::NeedsInput);
        s1.needs_attention_since = Some(1000); // oldest

        let mut s2 = make_session("b", 2, SessionStatus::NeedsInput);
        s2.needs_attention_since = Some(3000); // newest

        let mut s3 = make_session("c", 3, SessionStatus::NeedsInput);
        s3.needs_attention_since = Some(2000); // middle

        manager.add_session(s2);
        manager.add_session(s3);
        manager.add_session(s1);

        let sessions = manager.list_sessions();
        // All NeedsInput, should be sorted oldest attention first
        assert_eq!(sessions[0].id, "a"); // 1000
        assert_eq!(sessions[1].id, "c"); // 2000
        assert_eq!(sessions[2].id, "b"); // 3000
    }

    #[test]
    fn test_list_sessions_mixed_status_priority() {
        let mut manager = SessionManager::new();

        manager.add_session(make_session("done", 1, SessionStatus::Done));
        manager.add_session(make_session("stuck", 2, SessionStatus::Stuck));
        manager.add_session(make_session("working", 3, SessionStatus::Working));
        manager.add_session(make_session("needs", 4, SessionStatus::NeedsInput));
        manager.add_session(make_session("pr", 5, SessionStatus::PrReady));
        manager.add_session(make_session("empty", 6, SessionStatus::Empty));

        let sessions = manager.list_sessions();
        assert_eq!(sessions[0].status, SessionStatus::NeedsInput);
        assert_eq!(sessions[1].status, SessionStatus::Stuck);
        assert_eq!(sessions[2].status, SessionStatus::PrReady);
        assert_eq!(sessions[3].status, SessionStatus::Working);
        assert_eq!(sessions[4].status, SessionStatus::Done);
        assert_eq!(sessions[5].status, SessionStatus::Empty);
    }

    #[test]
    fn test_list_sessions_same_status_sorted_by_number() {
        let mut manager = SessionManager::new();
        manager.add_session(make_session("c", 5, SessionStatus::Working));
        manager.add_session(make_session("a", 2, SessionStatus::Working));
        manager.add_session(make_session("b", 8, SessionStatus::Working));

        let sessions = manager.list_sessions();
        assert_eq!(sessions[0].number, 2);
        assert_eq!(sessions[1].number, 5);
        assert_eq!(sessions[2].number, 8);
    }

    #[test]
    fn test_add_session_panics_on_invalid_number() {
        let mut manager = SessionManager::new();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            manager.add_session(make_session("bad", 0, SessionStatus::Empty));
        }));
        assert!(result.is_err());
    }

    #[test]
    fn test_remove_nonexistent_session_returns_none() {
        let mut manager = SessionManager::new();
        assert!(manager.remove_session("nonexistent").is_none());
    }

    #[test]
    fn test_fill_and_empty_all_slots() {
        let mut manager = SessionManager::new();
        for i in 1..=9 {
            manager.add_session(make_session(
                &format!("s{}", i),
                i,
                SessionStatus::Working,
            ));
        }
        assert_eq!(manager.next_available_number(), None);

        // Remove middle slot
        manager.remove_session("s5");
        assert_eq!(manager.next_available_number(), Some(5));

        // Remove all
        for i in 1..=9 {
            manager.remove_session(&format!("s{}", i));
        }
        assert_eq!(manager.next_available_number(), Some(1));
    }
}
