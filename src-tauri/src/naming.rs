/// Auto-naming logic for sessions.
///
/// Priority:
/// 1. Git branch name (if not generic like cmux/session-N)
/// 2. Agent's first substantive output
/// 3. Fallback: {project-name}-{session-number}

/// Derive a session name from a branch name.
/// Returns None if the branch name is generic (e.g., cmux/session-3).
pub fn name_from_branch(branch: &str) -> Option<String> {
    // Strip common prefixes
    let stripped = branch
        .strip_prefix("cmux/session-")
        .or_else(|| branch.strip_prefix("cmux/"))
        .or_else(|| branch.strip_prefix("feature/"))
        .or_else(|| branch.strip_prefix("fix/"))
        .or_else(|| branch.strip_prefix("bugfix/"))
        .or_else(|| branch.strip_prefix("hotfix/"))
        .unwrap_or(branch);

    // If it's just a number (from cmux/session-N), it's not useful
    if stripped.parse::<u32>().is_ok() {
        return None;
    }

    // If the name is too short, skip
    if stripped.len() < 3 {
        return None;
    }

    // Convert to kebab-case friendly name
    let name = stripped
        .replace('/', "-")
        .replace('_', "-")
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
    fn test_name_from_branch() {
        assert_eq!(name_from_branch("cmux/session-3"), None);
        assert_eq!(
            name_from_branch("fix/token-expiry"),
            Some("token-expiry".to_string())
        );
        assert_eq!(
            name_from_branch("feature/add-auth"),
            Some("add-auth".to_string())
        );
        assert_eq!(
            name_from_branch("my-cool-branch"),
            Some("my-cool-branch".to_string())
        );
    }

    #[test]
    fn test_fallback_name() {
        assert_eq!(fallback_name("payments-api", 3), "payments-api-3");
    }
}
