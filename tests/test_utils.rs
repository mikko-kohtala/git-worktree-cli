use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Set up a temporary test environment
pub fn setup_test_env() -> TempDir {
    let temp_dir = tempfile::Builder::new()
        .prefix("gwt_test_")
        .tempdir()
        .expect("Failed to create temp directory");

    // Ensure the temp directory is clean
    let temp_path = temp_dir.path();
    if temp_path.exists() {
        fs::remove_dir_all(temp_path).ok();
        fs::create_dir_all(temp_path).expect("Failed to create temp directory");
    }

    temp_dir
}

/// Clean up test environment
pub fn cleanup_test_env(temp_dir: TempDir) {
    // The TempDir will automatically clean up when dropped,
    // but we can also do explicit cleanup if needed
    drop(temp_dir);
}

/// Create a test configuration file
#[allow(dead_code)]
pub fn create_test_config(dir: &std::path::Path, repo_url: &str, main_branch: &str) -> PathBuf {
    let config_content = format!(
        r#"{{
  "repositoryUrl": "{}",
  "mainBranch": "{}",
  "createdAt": "2025-06-25T17:25:28.766876Z",
  "sourceControl": "github",
  "hooks": {{
    "postAdd": [],
    "postRemove": []
  }}
}}"#,
        repo_url, main_branch
    );

    let config_path = dir.join("git-worktree-config.jsonc");
    fs::write(&config_path, config_content).expect("Failed to write test config");
    config_path
}

/// Verify that a directory contains a git repository
#[allow(dead_code)]
pub fn is_git_repo(dir: &std::path::Path) -> bool {
    dir.join(".git").exists()
}

/// Get the current git branch name from a repository
#[allow(dead_code)]
pub fn get_current_branch(repo_dir: &std::path::Path) -> Result<String, Box<dyn std::error::Error>> {
    use std::process::Command;

    let output = Command::new("git")
        .args(["symbolic-ref", "--short", "HEAD"])
        .current_dir(repo_dir)
        .output()?;

    if output.status.success() {
        Ok(String::from_utf8(output.stdout)?.trim().to_string())
    } else {
        Err("Failed to get current branch".into())
    }
}

/// Check if git is available on the system
#[allow(dead_code)]
pub fn is_git_available() -> bool {
    std::process::Command::new("git")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Skip test if git is not available
#[macro_export]
macro_rules! require_git {
    () => {
        if !$crate::test_utils::is_git_available() {
            eprintln!("Skipping test: git not available");
            return;
        }
    };
}
