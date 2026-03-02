use std::process::Command;

/// Get the current branch of a git repo.
pub fn current_branch(project_path: &str) -> Result<String, String> {
    let output = Command::new("git")
        .args(["branch", "--show-current"])
        .current_dir(project_path)
        .output()
        .map_err(|e| format!("Failed to run git: {}", e))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Create a new branch from the default branch.
pub fn create_new_branch(project_path: &str, branch_name: &str) -> Result<String, String> {
    // Check for dirty working directory
    let status_output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(project_path)
        .output()
        .map_err(|e| format!("Failed to run git status: {}", e))?;

    let is_dirty = !String::from_utf8_lossy(&status_output.stdout)
        .trim()
        .is_empty();

    if is_dirty {
        log::warn!(
            "Working directory {} has uncommitted changes",
            project_path
        );
    }

    // Create branch from current HEAD
    let output = Command::new("git")
        .args(["checkout", "-b", branch_name])
        .current_dir(project_path)
        .output()
        .map_err(|e| format!("Failed to create branch: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // If branch already exists, just check it out
        if stderr.contains("already exists") {
            let output = Command::new("git")
                .args(["checkout", branch_name])
                .current_dir(project_path)
                .output()
                .map_err(|e| format!("Failed to checkout branch: {}", e))?;

            if !output.status.success() {
                return Err(String::from_utf8_lossy(&output.stderr).to_string());
            }
        } else {
            return Err(stderr.to_string());
        }
    }

    Ok(branch_name.to_string())
}

