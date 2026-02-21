use regex::Regex;
use serde::Serialize;
use std::sync::LazyLock;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter};

use crate::session::SessionStatus;

// --- Constants for status detection thresholds ---
const BUFFER_MAX_CHARS: usize = 2000;
const AUTO_NAME_THRESHOLD: usize = 2000;
const SIGNIFICANT_OUTPUT_BYTES: usize = 500;
const MIN_OUTPUT_BYTES: usize = 100;
const STUCK_TIMEOUT_SECS: u64 = 180;

/// Precompiled regex for stripping ANSI escape codes (hot path).
static ANSI_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\x1b\[[0-9;]*[a-zA-Z]|\x1b\].*?\x07|\x1b\[.*?[mGKHJP]").unwrap()
});

#[derive(Debug, Clone, Serialize)]
pub struct StatusChangeEvent {
    pub session_id: String,
    pub new_status: SessionStatus,
    pub name: Option<String>,
    pub needs_attention_since: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PrDetectedEvent {
    pub session_id: String,
    pub url: String,
}

pub struct StatusDetector {
    session_id: String,
    current_status: SessionStatus,
    last_output_time: Instant,
    output_buffer: String,
    auto_name: Option<String>,
    name_locked: bool,
    pr_url: Option<String>,
    needs_attention_since: Option<u64>,
    pr_regex: Regex,
    prompt_regex: Regex,
    allow_deny_regex: Regex,
    total_output_bytes: usize,
}

impl StatusDetector {
    pub fn new(session_id: String) -> Self {
        StatusDetector {
            session_id,
            current_status: SessionStatus::Empty,
            last_output_time: Instant::now(),
            output_buffer: String::new(),
            auto_name: None,
            name_locked: false,
            pr_url: None,
            needs_attention_since: None,
            pr_regex: Regex::new(r"https://github\.com/[a-zA-Z0-9_.-]+/[a-zA-Z0-9_.-]+/pull/\d+")
                .unwrap(),
            prompt_regex: Regex::new(r"[❯>]\s*$").unwrap(),
            allow_deny_regex: Regex::new(r"(?i)(allow|deny|yes/no|y/n|\[Y/n\]|\[y/N\])").unwrap(),
            total_output_bytes: 0,
        }
    }

    pub fn feed(&mut self, data: &[u8], app_handle: &AppHandle) {
        let text = String::from_utf8_lossy(data);
        self.output_buffer.push_str(&text);
        self.total_output_bytes += data.len();

        // Trim buffer to keep memory bounded
        if self.output_buffer.len() > BUFFER_MAX_CHARS {
            let start = self.output_buffer.len() - BUFFER_MAX_CHARS;
            self.output_buffer = self.output_buffer[start..].to_string();
        }

        self.last_output_time = Instant::now();

        // Strip ANSI escape codes for pattern matching
        let clean_text = strip_ansi(&self.output_buffer);

        // PR URL detection
        if let Some(m) = self.pr_regex.find(&clean_text) {
            let url = m.as_str().to_string();
            if self.pr_url.as_deref() != Some(&url) {
                self.pr_url = Some(url.clone());
                let _ = app_handle.emit(
                    "pr-detected",
                    PrDetectedEvent {
                        session_id: self.session_id.clone(),
                        url: url.clone(),
                    },
                );
                self.transition(SessionStatus::PrReady, app_handle);
            }
        }

        // Status detection
        let new_status = self.detect_status(&clean_text);
        if new_status != self.current_status {
            self.transition(new_status, app_handle);
        }

        // Auto-naming (only on early output)
        if !self.name_locked && self.total_output_bytes < AUTO_NAME_THRESHOLD {
            self.try_auto_name(&clean_text, app_handle);
        }
    }

    fn detect_status(&mut self, clean_text: &str) -> SessionStatus {
        let last_lines: String = clean_text
            .lines()
            .rev()
            .take(5)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join("\n");

        // Check for allow/deny prompts
        if self.allow_deny_regex.is_match(&last_lines) {
            return SessionStatus::NeedsInput;
        }

        // Check for Claude Code idle prompt (❯ or >) at end of output
        if self.prompt_regex.is_match(&last_lines) {
            if self.total_output_bytes > SIGNIFICANT_OUTPUT_BYTES {
                let lower = last_lines.to_lowercase();
                if lower.contains("created pr")
                    || lower.contains("all tests pass")
                    || lower.contains("complete")
                    || lower.contains("done")
                    || lower.contains("finished")
                {
                    return SessionStatus::Done;
                }
                return SessionStatus::NeedsInput;
            }
            return SessionStatus::Empty;
        }

        // If we're receiving output, we're working
        if self.total_output_bytes > MIN_OUTPUT_BYTES {
            return SessionStatus::Working;
        }

        self.current_status.clone()
    }

    fn transition(&mut self, new_status: SessionStatus, app_handle: &AppHandle) {
        log::info!(
            "Session {} status: {:?} -> {:?}",
            self.session_id,
            self.current_status,
            new_status
        );

        self.current_status = new_status.clone();

        // Track when the session started needing attention (backend is source of truth)
        if new_status == SessionStatus::NeedsInput || new_status == SessionStatus::Stuck {
            if self.needs_attention_since.is_none() {
                self.needs_attention_since = Some(unix_timestamp());
            }
        } else {
            self.needs_attention_since = None;
        }

        let _ = app_handle.emit(
            "session-status-changed",
            StatusChangeEvent {
                session_id: self.session_id.clone(),
                new_status,
                name: self.auto_name.clone(),
                needs_attention_since: self.needs_attention_since,
            },
        );
    }

    fn try_auto_name(&mut self, clean_text: &str, app_handle: &AppHandle) {
        for line in clean_text.lines() {
            let line = line.trim();
            if line.len() < 10 || line.starts_with('$') || line.starts_with("claude") {
                continue;
            }

            if line.contains("I'll help you") || line.contains("I'll ") || line.contains("Let me ")
            {
                if let Some(name) = extract_slug(line) {
                    self.auto_name = Some(name.clone());
                    self.name_locked = true;
                    let _ = app_handle.emit(
                        "session-status-changed",
                        StatusChangeEvent {
                            session_id: self.session_id.clone(),
                            new_status: self.current_status.clone(),
                            name: Some(name),
                            needs_attention_since: self.needs_attention_since,
                        },
                    );
                    return;
                }
            }
        }
    }

    /// Check for stuck status (called periodically from background thread)
    pub fn check_stuck(&mut self, app_handle: &AppHandle) {
        if self.current_status == SessionStatus::Working
            && self.last_output_time.elapsed() > Duration::from_secs(STUCK_TIMEOUT_SECS)
        {
            self.transition(SessionStatus::Stuck, app_handle);
        }
    }

    pub fn current_status(&self) -> &SessionStatus {
        &self.current_status
    }

    pub fn get_name(&self) -> Option<&str> {
        self.auto_name.as_deref()
    }

    pub fn get_pr_url(&self) -> Option<&str> {
        self.pr_url.as_deref()
    }
}

/// Strip ANSI escape codes from text (uses cached regex).
fn strip_ansi(text: &str) -> String {
    ANSI_RE.replace_all(text, "").to_string()
}

/// Extract a short slug from a description.
fn extract_slug(text: &str) -> Option<String> {
    let text = text
        .replace("I'll help you ", "")
        .replace("I'll ", "")
        .replace("Let me ", "");

    let words: Vec<&str> = text
        .split_whitespace()
        .filter(|w| w.len() > 2)
        .take(4)
        .collect();

    if words.is_empty() {
        return None;
    }

    let slug: String = words
        .join("-")
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-')
        .collect();

    if slug.len() < 3 {
        return None;
    }

    Some(slug.chars().take(30).collect())
}

pub fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_ansi() {
        assert_eq!(strip_ansi("hello"), "hello");
        assert_eq!(strip_ansi("\x1b[32mgreen\x1b[0m"), "green");
        assert_eq!(strip_ansi("\x1b[1;34mbold blue\x1b[0m"), "bold blue");
    }

    #[test]
    fn test_extract_slug() {
        assert_eq!(
            extract_slug("I'll help you add a Stripe webhook handler"),
            Some("add-stripe-webhook-handler".to_string())
        );
        assert_eq!(
            extract_slug("Let me fix the authentication bug"),
            Some("fix-the-authentication-bug".to_string())
        );
        assert_eq!(extract_slug("hi"), None);
    }

    #[test]
    fn test_extract_slug_truncation() {
        let long_text =
            "I'll help you implement a very long feature name that exceeds thirty characters";
        let slug = extract_slug(long_text).unwrap();
        assert!(slug.len() <= 30);
    }

    #[test]
    fn test_unix_timestamp() {
        let ts = unix_timestamp();
        assert!(ts > 1700000000);
    }

    #[test]
    fn test_pr_url_regex() {
        let re =
            Regex::new(r"https://github\.com/[a-zA-Z0-9_.-]+/[a-zA-Z0-9_.-]+/pull/\d+").unwrap();
        assert!(re.is_match("https://github.com/owner/repo/pull/123"));
        assert!(re.is_match("https://github.com/my-org/my-repo/pull/42"));
        assert!(!re.is_match("https://gitlab.com/owner/repo/pull/1"));
    }

    #[test]
    fn test_should_notify() {
        use crate::notifications::should_notify;

        assert!(should_notify(
            &SessionStatus::Working,
            &SessionStatus::NeedsInput
        ));
        assert!(should_notify(
            &SessionStatus::Working,
            &SessionStatus::Stuck
        ));
        assert!(!should_notify(
            &SessionStatus::NeedsInput,
            &SessionStatus::NeedsInput
        ));
        assert!(!should_notify(
            &SessionStatus::Working,
            &SessionStatus::Done
        ));
    }
}
