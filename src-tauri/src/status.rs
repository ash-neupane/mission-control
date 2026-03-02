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

/// Precompiled regex for GitHub PR URL detection.
static PR_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"https://github\.com/[a-zA-Z0-9_.-]+/[a-zA-Z0-9_.-]+/pull/\d+").unwrap()
});

/// Precompiled regex for Claude Code idle prompt (❯ or >).
static PROMPT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[❯>]\s*$").unwrap());

/// Precompiled regex for allow/deny prompts.
static ALLOW_DENY_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(allow|deny|yes/no|y/n|\[Y/n\]|\[y/N\])").unwrap()
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
    session_number: u8,
    current_status: SessionStatus,
    last_output_time: Instant,
    output_buffer: String,
    auto_name: Option<String>,
    name_locked: bool,
    pr_url: Option<String>,
    needs_attention_since: Option<u64>,
    total_output_bytes: usize,
}

impl StatusDetector {
    pub fn new(session_id: String, session_number: u8) -> Self {
        StatusDetector {
            session_id,
            session_number,
            current_status: SessionStatus::Empty,
            last_output_time: Instant::now(),
            output_buffer: String::new(),
            auto_name: None,
            name_locked: false,
            pr_url: None,
            needs_attention_since: None,
            total_output_bytes: 0,
        }
    }

    pub fn feed(&mut self, data: &[u8], app_handle: &AppHandle) {
        let text = String::from_utf8_lossy(data);
        self.output_buffer.push_str(&text);
        self.total_output_bytes += data.len();

        // Trim buffer to keep memory bounded (advance to valid char boundary)
        if self.output_buffer.len() > BUFFER_MAX_CHARS {
            let mut start = self.output_buffer.len() - BUFFER_MAX_CHARS;
            while !self.output_buffer.is_char_boundary(start) && start < self.output_buffer.len() {
                start += 1;
            }
            self.output_buffer = self.output_buffer[start..].to_string();
        }

        self.last_output_time = Instant::now();

        // Strip ANSI escape codes for pattern matching
        let clean_text = strip_ansi(&self.output_buffer);

        // PR URL detection
        if let Some(m) = PR_RE.find(&clean_text) {
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
                // Don't let detect_status() override PrReady in the same feed call
                return;
            }
        }

        // Status detection (skip if already PrReady — preserve until prompt resets it)
        let new_status = self.detect_status(&clean_text);
        if new_status != self.current_status {
            self.transition(new_status, app_handle);
        }

        // Auto-naming (only on early output)
        if !self.name_locked && self.total_output_bytes < AUTO_NAME_THRESHOLD {
            self.try_auto_name(&clean_text, app_handle);
        }
    }

    fn detect_status(&self, clean_text: &str) -> SessionStatus {
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
        if ALLOW_DENY_RE.is_match(&last_lines) {
            return SessionStatus::NeedsInput;
        }

        // Check for Claude Code idle prompt (❯ or >) at end of output
        if PROMPT_RE.is_match(&last_lines) {
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

    /// Mark session as Done (called by reader thread on PTY EOF).
    pub fn mark_done(&mut self, app_handle: &AppHandle) {
        if self.current_status != SessionStatus::Done {
            self.transition(SessionStatus::Done, app_handle);
        }
    }

    fn transition(&mut self, new_status: SessionStatus, app_handle: &AppHandle) {
        use crate::notifications;

        // Check if this transition should trigger an OS notification
        let should_notify = notifications::should_notify(&self.current_status, &new_status);

        log::info!(
            "Session {} status: {:?} -> {:?}",
            self.session_id,
            self.current_status,
            new_status
        );

        let old_status = self.current_status.clone();
        self.current_status = new_status.clone();

        // Emit notification event so the frontend can show an OS notification
        if should_notify {
            let _ = app_handle.emit(
                "session-notification",
                serde_json::json!({
                    "session_id": self.session_id,
                    "title": notifications::notification_title(self.session_number),
                    "body": notifications::notification_body(
                        self.auto_name.as_deref().unwrap_or(&self.session_id),
                        &format!("{old_status:?} → {new_status:?}"),
                    ),
                }),
            );
        }

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

    fn make_detector() -> StatusDetector {
        StatusDetector::new("test-session".to_string(), 1)
    }

    // --- Helper function tests ---

    #[test]
    fn test_strip_ansi_plain() {
        assert_eq!(strip_ansi("hello"), "hello");
    }

    #[test]
    fn test_strip_ansi_color_codes() {
        assert_eq!(strip_ansi("\x1b[32mgreen\x1b[0m"), "green");
        assert_eq!(strip_ansi("\x1b[1;34mbold blue\x1b[0m"), "bold blue");
    }

    #[test]
    fn test_strip_ansi_nested() {
        assert_eq!(
            strip_ansi("\x1b[1m\x1b[31merror\x1b[0m\x1b[0m"),
            "error"
        );
    }

    #[test]
    fn test_extract_slug_standard() {
        assert_eq!(
            extract_slug("I'll help you add a Stripe webhook handler"),
            Some("add-stripe-webhook-handler".to_string())
        );
        assert_eq!(
            extract_slug("Let me fix the authentication bug"),
            Some("fix-the-authentication-bug".to_string())
        );
    }

    #[test]
    fn test_extract_slug_too_short() {
        assert_eq!(extract_slug("hi"), None);
        assert_eq!(extract_slug("I'll do a"), None); // "a" is filtered (len <= 2)
    }

    #[test]
    fn test_extract_slug_truncation() {
        let long_text =
            "I'll help you implement a very long feature name that exceeds thirty characters";
        let slug = extract_slug(long_text).unwrap();
        assert!(slug.len() <= 30);
    }

    #[test]
    fn test_extract_slug_special_chars_stripped() {
        let slug = extract_slug("I'll help you add @special #chars!").unwrap();
        assert!(slug.chars().all(|c| c.is_alphanumeric() || c == '-'));
    }

    #[test]
    fn test_unix_timestamp() {
        let ts = unix_timestamp();
        assert!(ts > 1700000000);
    }

    // --- Static regex tests ---

    #[test]
    fn test_pr_url_regex_matches() {
        assert!(PR_RE.is_match("https://github.com/owner/repo/pull/123"));
        assert!(PR_RE.is_match("https://github.com/my-org/my-repo/pull/42"));
        assert!(PR_RE.is_match("https://github.com/a.b/c-d/pull/1"));
    }

    #[test]
    fn test_pr_url_regex_rejects() {
        assert!(!PR_RE.is_match("https://gitlab.com/owner/repo/pull/1"));
        assert!(!PR_RE.is_match("https://github.com/owner/repo/issues/1"));
        assert!(!PR_RE.is_match("not a url at all"));
    }

    #[test]
    fn test_prompt_regex() {
        assert!(PROMPT_RE.is_match("some output ❯ "));
        assert!(PROMPT_RE.is_match("some output > "));
        assert!(PROMPT_RE.is_match("❯"));
        assert!(!PROMPT_RE.is_match("some output without prompt"));
    }

    #[test]
    fn test_allow_deny_regex() {
        assert!(ALLOW_DENY_RE.is_match("Do you want to allow this?"));
        assert!(ALLOW_DENY_RE.is_match("Allow or deny?"));
        assert!(ALLOW_DENY_RE.is_match("Continue? [Y/n]"));
        assert!(ALLOW_DENY_RE.is_match("Proceed? [y/N]"));
        assert!(ALLOW_DENY_RE.is_match("yes/no"));
        assert!(!ALLOW_DENY_RE.is_match("just some normal output"));
    }

    // --- Status detection state machine tests ---

    #[test]
    fn test_detect_status_empty_initial() {
        let det = make_detector();
        assert_eq!(*det.current_status(), SessionStatus::Empty);
    }

    #[test]
    fn test_detect_status_stays_empty_with_little_output() {
        let mut det = make_detector();
        det.total_output_bytes = 50; // below MIN_OUTPUT_BYTES
        let status = det.detect_status("some text");
        assert_eq!(status, SessionStatus::Empty);
    }

    #[test]
    fn test_detect_status_working_with_enough_output() {
        let mut det = make_detector();
        det.total_output_bytes = MIN_OUTPUT_BYTES + 1;
        let status = det.detect_status("actively processing something");
        assert_eq!(status, SessionStatus::Working);
    }

    #[test]
    fn test_detect_status_needs_input_on_prompt() {
        let mut det = make_detector();
        det.total_output_bytes = SIGNIFICANT_OUTPUT_BYTES + 1;
        let status = det.detect_status("here is some output\n❯ ");
        assert_eq!(status, SessionStatus::NeedsInput);
    }

    #[test]
    fn test_detect_status_empty_on_prompt_with_little_output() {
        let mut det = make_detector();
        det.total_output_bytes = 50; // below SIGNIFICANT_OUTPUT_BYTES
        let status = det.detect_status("❯ ");
        assert_eq!(status, SessionStatus::Empty);
    }

    #[test]
    fn test_detect_status_done_on_completion_keywords() {
        let mut det = make_detector();
        det.total_output_bytes = SIGNIFICANT_OUTPUT_BYTES + 1;

        assert_eq!(
            det.detect_status("All tests pass. Done.\n❯ "),
            SessionStatus::Done
        );
        assert_eq!(
            det.detect_status("Task complete\n> "),
            SessionStatus::Done
        );
        assert_eq!(
            det.detect_status("I've finished the implementation\n❯ "),
            SessionStatus::Done
        );
        assert_eq!(
            det.detect_status("Created PR #42\n❯ "),
            SessionStatus::Done
        );
    }

    #[test]
    fn test_detect_status_allow_deny_takes_priority() {
        let mut det = make_detector();
        det.total_output_bytes = SIGNIFICANT_OUTPUT_BYTES + 1;
        // allow/deny should match even without a prompt character
        let status = det.detect_status("Do you want to allow this change? [Y/n]");
        assert_eq!(status, SessionStatus::NeedsInput);
    }

    #[test]
    fn test_detect_status_preserves_current_when_no_match() {
        let mut det = make_detector();
        det.current_status = SessionStatus::PrReady;
        det.total_output_bytes = 50; // low
        let status = det.detect_status("some text");
        // Should preserve current (PrReady) since no condition matches
        assert_eq!(status, SessionStatus::PrReady);
    }

    // --- needs_attention_since lifecycle tests ---

    #[test]
    fn test_needs_attention_since_starts_none() {
        let det = make_detector();
        assert!(det.needs_attention_since.is_none());
    }

    // --- Buffer management tests ---

    #[test]
    fn test_buffer_bounded_by_max_chars() {
        let mut det = make_detector();
        // Directly manipulate the buffer to exceed the limit
        det.output_buffer = "x".repeat(BUFFER_MAX_CHARS + 500);
        // Simulate what feed() does for trimming
        if det.output_buffer.len() > BUFFER_MAX_CHARS {
            let mut start = det.output_buffer.len() - BUFFER_MAX_CHARS;
            while !det.output_buffer.is_char_boundary(start) && start < det.output_buffer.len() {
                start += 1;
            }
            det.output_buffer = det.output_buffer[start..].to_string();
        }
        assert!(det.output_buffer.len() <= BUFFER_MAX_CHARS);
    }

    #[test]
    fn test_buffer_trim_respects_char_boundaries() {
        let mut det = make_detector();
        // Fill with multi-byte chars (emoji = 4 bytes each)
        let emoji = "\u{1F680}".repeat(600); // 2400 bytes, 600 chars
        det.output_buffer = emoji;
        if det.output_buffer.len() > BUFFER_MAX_CHARS {
            let mut start = det.output_buffer.len() - BUFFER_MAX_CHARS;
            while !det.output_buffer.is_char_boundary(start) && start < det.output_buffer.len() {
                start += 1;
            }
            det.output_buffer = det.output_buffer[start..].to_string();
        }
        // Should not panic and should be valid UTF-8
        assert!(det.output_buffer.len() <= BUFFER_MAX_CHARS + 4); // +4 for boundary rounding
    }

    // --- Auto-naming tests ---

    #[test]
    fn test_auto_name_not_locked_initially() {
        let det = make_detector();
        assert!(!det.name_locked);
        assert!(det.auto_name.is_none());
    }

    // --- Stuck detection tests ---

    #[test]
    fn test_check_stuck_no_op_when_not_working() {
        let mut det = make_detector();
        det.current_status = SessionStatus::NeedsInput;
        // Artificially set old last_output_time
        det.last_output_time = Instant::now() - Duration::from_secs(STUCK_TIMEOUT_SECS + 10);
        // check_stuck needs AppHandle which we can't create in tests,
        // but we can verify the condition check directly
        let is_stuck_condition = det.current_status == SessionStatus::Working
            && det.last_output_time.elapsed() > Duration::from_secs(STUCK_TIMEOUT_SECS);
        assert!(!is_stuck_condition); // NeedsInput != Working, so not stuck
    }

    #[test]
    fn test_stuck_condition_true_when_working_and_timed_out() {
        let mut det = make_detector();
        det.current_status = SessionStatus::Working;
        det.last_output_time = Instant::now() - Duration::from_secs(STUCK_TIMEOUT_SECS + 10);
        let is_stuck_condition = det.current_status == SessionStatus::Working
            && det.last_output_time.elapsed() > Duration::from_secs(STUCK_TIMEOUT_SECS);
        assert!(is_stuck_condition);
    }

    #[test]
    fn test_stuck_condition_false_when_working_recently() {
        let mut det = make_detector();
        det.current_status = SessionStatus::Working;
        det.last_output_time = Instant::now(); // just now
        let is_stuck_condition = det.current_status == SessionStatus::Working
            && det.last_output_time.elapsed() > Duration::from_secs(STUCK_TIMEOUT_SECS);
        assert!(!is_stuck_condition);
    }

    // --- Notification integration tests ---

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

    // --- Session number in detector ---

    #[test]
    fn test_detector_stores_session_number() {
        let det = StatusDetector::new("id-1".to_string(), 5);
        assert_eq!(det.session_number, 5);
    }
}
