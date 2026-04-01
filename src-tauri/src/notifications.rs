use crate::session::SessionStatus;

/// Send an OS notification for a session status change.
/// We use Tauri's notification plugin which is configured on the frontend side.
/// This module provides the logic for when to notify.
///
/// Notifies on transitions INTO these actionable states:
/// - NeedsInput / Stuck: session needs user attention
/// - PrReady / Done: session completed its work
pub fn should_notify(old_status: &SessionStatus, new_status: &SessionStatus) -> bool {
    if old_status == new_status {
        return false;
    }
    matches!(
        new_status,
        SessionStatus::NeedsInput | SessionStatus::Stuck | SessionStatus::PrReady | SessionStatus::Done
    )
}

/// Format notification body text
pub fn notification_body(session_name: &str, last_output: &str) -> String {
    let truncated = if last_output.chars().count() > 80 {
        let s: String = last_output.chars().take(77).collect();
        format!("{}...", s)
    } else {
        last_output.to_string()
    };
    format!("{session_name}: {truncated}")
}

pub fn notification_title(session_number: u8) -> String {
    format!("c-mux — Session {session_number}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_notify() {
        // Transitions into actionable states should notify
        assert!(should_notify(
            &SessionStatus::Working,
            &SessionStatus::NeedsInput
        ));
        assert!(should_notify(
            &SessionStatus::Empty,
            &SessionStatus::Stuck
        ));
        assert!(should_notify(
            &SessionStatus::Working,
            &SessionStatus::PrReady
        ));
        assert!(should_notify(
            &SessionStatus::Working,
            &SessionStatus::Done
        ));
        // Same status should not notify
        assert!(!should_notify(
            &SessionStatus::NeedsInput,
            &SessionStatus::NeedsInput
        ));
        // Transitions between notifiable states should still notify (status changed)
        assert!(should_notify(
            &SessionStatus::Stuck,
            &SessionStatus::NeedsInput
        ));
        // Transitions to non-actionable states should not notify
        assert!(!should_notify(
            &SessionStatus::NeedsInput,
            &SessionStatus::Working
        ));
    }

    #[test]
    fn test_notification_body_truncation() {
        let short = notification_body("test", "hello world");
        assert_eq!(short, "test: hello world");

        let long_output = "x".repeat(100);
        let truncated = notification_body("test", &long_output);
        assert!(truncated.ends_with("..."));
    }

    #[test]
    fn test_notification_body_multibyte_utf8() {
        // 30 emoji = 30 chars but 120 bytes — must not panic
        let emoji_output = "\u{1F680}".repeat(90);
        let result = notification_body("test", &emoji_output);
        assert!(result.ends_with("..."));
    }

    #[test]
    fn test_notification_title() {
        assert_eq!(notification_title(3), "c-mux — Session 3");
        assert_eq!(notification_title(9), "c-mux — Session 9");
        assert_eq!(notification_title(1), "c-mux — Session 1");
    }

    #[test]
    fn test_should_notify_all_transitions() {
        let statuses = [
            SessionStatus::Empty,
            SessionStatus::Working,
            SessionStatus::NeedsInput,
            SessionStatus::PrReady,
            SessionStatus::Stuck,
            SessionStatus::Done,
        ];

        // Exhaustively verify: transitions INTO actionable states
        // (NeedsInput, Stuck, PrReady, Done) trigger notification,
        // except same-status transitions.
        for old in &statuses {
            for new in &statuses {
                let result = should_notify(old, new);
                let expect = old != new
                    && matches!(
                        new,
                        SessionStatus::NeedsInput
                            | SessionStatus::Stuck
                            | SessionStatus::PrReady
                            | SessionStatus::Done
                    );
                assert_eq!(
                    result, expect,
                    "should_notify({old:?} → {new:?}) = {result}, expected {expect}"
                );
            }
        }
    }

    #[test]
    fn test_notification_body_exact_80_chars() {
        let output = "x".repeat(80);
        let body = notification_body("t", &output);
        // "t: " + 80 = 83 chars, but output is exactly 80 so no truncation
        assert!(!body.ends_with("..."));
    }

    #[test]
    fn test_notification_body_81_chars_truncated() {
        let output = "x".repeat(81);
        let body = notification_body("t", &output);
        assert!(body.ends_with("..."));
    }

    #[test]
    fn test_notification_body_empty_output() {
        let body = notification_body("session", "");
        assert_eq!(body, "session: ");
    }
}
