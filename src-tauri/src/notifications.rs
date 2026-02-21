use crate::session::SessionStatus;

/// Send an OS notification for a session status change.
/// We use Tauri's notification plugin which is configured on the frontend side.
/// This module provides the logic for when to notify.
pub fn should_notify(old_status: &SessionStatus, new_status: &SessionStatus) -> bool {
    matches!(
        new_status,
        SessionStatus::NeedsInput | SessionStatus::Stuck
    ) && !matches!(
        old_status,
        SessionStatus::NeedsInput | SessionStatus::Stuck
    )
}

/// Format notification body text
pub fn notification_body(session_name: &str, last_output: &str) -> String {
    let truncated = if last_output.len() > 80 {
        format!("{}...", &last_output[..77])
    } else {
        last_output.to_string()
    };
    format!("{}: {}", session_name, truncated)
}

pub fn notification_title(session_number: u8) -> String {
    format!("c-mux — Session {}", session_number)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_notify() {
        assert!(should_notify(
            &SessionStatus::Working,
            &SessionStatus::NeedsInput
        ));
        assert!(should_notify(
            &SessionStatus::Empty,
            &SessionStatus::Stuck
        ));
        assert!(!should_notify(
            &SessionStatus::NeedsInput,
            &SessionStatus::Working
        ));
        assert!(!should_notify(
            &SessionStatus::Stuck,
            &SessionStatus::NeedsInput
        ));
    }

    #[test]
    fn test_notification_body_truncation() {
        let short = notification_body("test", "hello world");
        assert_eq!(short, "test: hello world");

        let long_output = "x".repeat(100);
        let truncated = notification_body("test", &long_output);
        assert!(truncated.len() < 100);
        assert!(truncated.ends_with("..."));
    }

    #[test]
    fn test_notification_title() {
        assert_eq!(notification_title(3), "c-mux — Session 3");
    }
}
