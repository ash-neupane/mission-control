//! Auto-naming logic for sessions.
//!
//! Priority:
//! 1. Git branch name (if not generic like cmux/session-N)
//! 2. Agent's first substantive output
//! 3. Fallback: {project-name}-{session-number}

/// Derive a session name from a branch name.
///
/// `branch_prefix` is the configured prefix (e.g. "cmux/") so we can strip it
/// even if the user changes it from the default.
/// Returns None if the branch name is generic (e.g., cmux/session-3).
pub fn name_from_branch(branch: &str, branch_prefix: &str) -> Option<String> {
    // Build the "session-" variant of the configured prefix
    let session_prefix = format!("{}session-", branch_prefix);

    // Strip configured prefix first, then common git prefixes
    let stripped = branch
        .strip_prefix(&session_prefix)
        .or_else(|| branch.strip_prefix(branch_prefix))
        .or_else(|| branch.strip_prefix("feature/"))
        .or_else(|| branch.strip_prefix("fix/"))
        .or_else(|| branch.strip_prefix("bugfix/"))
        .or_else(|| branch.strip_prefix("hotfix/"))
        .unwrap_or(branch);

    // If it's just a number (from prefix/session-N), it's not useful
    if stripped.parse::<u32>().is_ok() {
        return None;
    }

    if stripped.len() < 3 {
        return None;
    }

    let name = stripped
        .replace(['/', '_'], "-")
        .to_lowercase();

    Some(name)
}

/// Generate a fallback name from project name and session number.
pub fn fallback_name(project_name: &str, session_number: u8) -> String {
    format!("{}-{}", project_name, session_number)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_name_from_branch_default_prefix() {
        assert_eq!(name_from_branch("cmux/session-3", "cmux/"), None);
        assert_eq!(
            name_from_branch("fix/token-expiry", "cmux/"),
            Some("token-expiry".to_string())
        );
        assert_eq!(
            name_from_branch("feature/add-auth", "cmux/"),
            Some("add-auth".to_string())
        );
        assert_eq!(
            name_from_branch("my-cool-branch", "cmux/"),
            Some("my-cool-branch".to_string())
        );
    }

    #[test]
    fn test_name_from_branch_custom_prefix() {
        assert_eq!(name_from_branch("dev/session-5", "dev/"), None);
        assert_eq!(
            name_from_branch("dev/fix-auth", "dev/"),
            Some("fix-auth".to_string())
        );
    }

    #[test]
    fn test_fallback_name() {
        assert_eq!(fallback_name("payments-api", 3), "payments-api-3");
    }
}
