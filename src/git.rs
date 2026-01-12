use colored::Colorize;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::error::{Error, Result};

/// Execute a git command with real-time output streaming
pub fn execute_streaming(args: &[&str], cwd: Option<&Path>) -> Result<()> {
    let mut cmd = Command::new("git");
    cmd.args(args).stdout(Stdio::inherit()).stderr(Stdio::inherit());

    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }

    let status = cmd
        .status()
        .map_err(|e| Error::git(format!("Failed to execute git command: {}", e)))?;

    if !status.success() {
        return Err(Error::git(format!(
            "Git command failed with exit code: {:?}",
            status.code()
        )));
    }

    Ok(())
}

/// Execute a git command and capture output
pub fn execute_capture(args: &[&str], cwd: Option<&Path>) -> Result<String> {
    let mut cmd = Command::new("git");
    cmd.args(args);

    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }

    let output = cmd
        .output()
        .map_err(|e| Error::git(format!("Failed to execute git command: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::git(format!("Git command failed: {}", stderr)));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Clone a repository with streaming output
pub fn clone(repo_url: &str, target_dir: &str) -> Result<()> {
    println!("{}", format!("Cloning {}...", repo_url).cyan());
    execute_streaming(&["clone", repo_url, target_dir], None)
}

/// Get the current branch name (the branch HEAD points to)
pub fn get_current_branch(repo_path: &Path) -> Result<String> {
    execute_capture(&["symbolic-ref", "--short", "HEAD"], Some(repo_path))
}

/// Get the default branch name from the remote origin
pub fn get_remote_default_branch(repo_path: &Path) -> Result<String> {
    // Try git symbolic-ref refs/remotes/origin/HEAD
    // Returns something like "refs/remotes/origin/master"
    if let Ok(ref_output) = execute_capture(
        &["symbolic-ref", "refs/remotes/origin/HEAD"],
        Some(repo_path),
    ) {
        if let Some(branch) = ref_output.strip_prefix("refs/remotes/origin/") {
            return Ok(branch.to_string());
        }
    }

    // Fallback: check which common branches exist on origin
    for branch in &["main", "master"] {
        if let Ok(result) = execute_capture(
            &["rev-parse", "--verify", &format!("origin/{}", branch)],
            Some(repo_path),
        ) {
            if !result.is_empty() {
                return Ok((*branch).to_string());
            }
        }
    }

    // Final fallback: use current branch (for repos without remote branches fetched)
    get_current_branch(repo_path)
}

/// Add a new worktree
/// List all worktrees
pub fn list_worktrees(git_dir: Option<&Path>) -> Result<Vec<Worktree>> {
    let output = execute_capture(&["worktree", "list", "--porcelain"], git_dir)?;
    parse_worktree_list(&output)
}

/// Prune worktree administrative files
///
/// Removes worktree references from .git/worktrees that are no longer valid
pub fn prune_worktrees(git_dir: &Path) -> Result<()> {
    execute_streaming(&["worktree", "prune"], Some(git_dir))
}

/// Remove a worktree
/// Delete a branch
/// Check if a branch exists
pub fn branch_exists(git_dir: &Path, branch_name: &str) -> Result<(bool, bool)> {
    let local = execute_capture(&["branch", "--list", branch_name], Some(git_dir)).unwrap_or_default();

    let remote = execute_capture(
        &["branch", "-r", "--list", &format!("origin/{}", branch_name)],
        Some(git_dir),
    )
    .unwrap_or_default();

    Ok((!local.is_empty(), !remote.is_empty()))
}

/// Get the current git root directory
pub fn get_git_root() -> Result<Option<PathBuf>> {
    match execute_capture(&["rev-parse", "--show-toplevel"], None) {
        Ok(path) => Ok(Some(PathBuf::from(path))),
        Err(_) => Ok(None),
    }
}

/// Get the remote origin URL from a git repository
pub fn get_remote_origin_url(path: &Path) -> Option<String> {
    execute_capture(&["remote", "get-url", "origin"], Some(path)).ok()
}

#[derive(Debug, Clone)]
pub struct Worktree {
    pub path: PathBuf,
    pub head: String,
    pub branch: Option<String>,
    pub bare: bool,
}

fn parse_worktree_list(output: &str) -> Result<Vec<Worktree>> {
    let mut worktrees = Vec::new();
    let mut current_worktree: Option<PartialWorktree> = None;

    #[derive(Default)]
    struct PartialWorktree {
        path: Option<PathBuf>,
        head: Option<String>,
        branch: Option<String>,
        bare: bool,
    }

    impl PartialWorktree {
        fn into_worktree(self) -> Option<Worktree> {
            match (self.path, self.head) {
                (Some(path), Some(head)) => Some(Worktree {
                    path,
                    head,
                    branch: self.branch,
                    bare: self.bare,
                }),
                _ => None,
            }
        }
    }

    for line in output.lines() {
        match parse_worktree_line(line) {
            WorktreeLine::New(path) => {
                if let Some(wt) = current_worktree.take() {
                    if let Some(worktree) = wt.into_worktree() {
                        worktrees.push(worktree);
                    }
                }
                current_worktree = Some(PartialWorktree {
                    path: Some(path),
                    ..Default::default()
                });
            }
            WorktreeLine::Head(head) => {
                if let Some(ref mut wt) = current_worktree {
                    wt.head = Some(head);
                }
            }
            WorktreeLine::Branch(branch) => {
                if let Some(ref mut wt) = current_worktree {
                    wt.branch = Some(branch);
                }
            }
            WorktreeLine::Bare => {
                if let Some(ref mut wt) = current_worktree {
                    wt.bare = true;
                }
            }
            WorktreeLine::Other => {}
        }
    }

    // Don't forget the last worktree
    if let Some(wt) = current_worktree {
        if let Some(worktree) = wt.into_worktree() {
            worktrees.push(worktree);
        }
    }

    Ok(worktrees)
}

enum WorktreeLine {
    New(PathBuf),
    Head(String),
    Branch(String),
    Bare,
    Other,
}

fn parse_worktree_line(line: &str) -> WorktreeLine {
    if let Some(path) = line.strip_prefix("worktree ") {
        WorktreeLine::New(PathBuf::from(path))
    } else if let Some(head) = line.strip_prefix("HEAD ") {
        WorktreeLine::Head(head.to_string())
    } else if let Some(branch) = line.strip_prefix("branch ") {
        WorktreeLine::Branch(branch.to_string())
    } else if line == "bare" {
        WorktreeLine::Bare
    } else {
        WorktreeLine::Other
    }
}
