//! Project discovery and management utilities
//!
//! This module handles finding project roots, git directories, and managing
//! project-related operations.

use crate::config::GitWorktreeConfig;
use crate::error::{Error, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// Represents a git worktree project with its root and git directory
#[derive(Debug, Clone)]
pub struct Project {
    /// The root directory containing git-worktree-config.jsonc
    pub root: PathBuf,
    /// The .git directory path
    pub git_dir: PathBuf,
}

impl Project {
    /// Find a project starting from the current directory
    pub fn find() -> Result<Self> {
        let root = find_project_root()?;
        let git_dir = find_git_directory_from(&root)?;
        Ok(Self { root, git_dir })
    }

    /// Find a project starting from a specific path
    pub fn find_from(start_path: &Path) -> Result<Self> {
        let root = find_project_root_from(start_path)?;
        let git_dir = find_git_directory_from(&root)?;
        Ok(Self { root, git_dir })
    }

    /// Get the bare repository directory (usually named after the main branch)
    pub fn bare_repo_dir(&self) -> Result<PathBuf> {
        find_existing_worktree(&self.root)
    }
}

/// Find the project root containing git-worktree-config.jsonc
pub fn find_project_root() -> Result<PathBuf> {
    let current_dir = std::env::current_dir().map_err(Error::Io)?;
    find_project_root_from(&current_dir)
}

/// Find the project root starting from a specific path
///
/// The project root is the main repository directory (where .git is).
/// This handles both:
/// - Running from inside the main repo
/// - Running from inside a worktree in the -worktrees folder
pub fn find_project_root_from(start_path: &Path) -> Result<PathBuf> {
    // Strategy 1: Check if we're in a git repository directly
    if let Ok(Some(git_root)) = crate::git::get_git_root() {
        // Check if this git root is inside a -worktrees folder (it's a worktree)
        if let Some(main_project) = find_main_project_from_worktree(&git_root) {
            return Ok(main_project);
        }
        // Otherwise, this is the main project
        return Ok(git_root);
    }

    // Strategy 2: Check if we're inside a -worktrees folder (but not in a git worktree)
    if let Some(main_project) = find_main_project_from_worktrees_path(start_path) {
        return Ok(main_project);
    }

    // Strategy 3: Check global config
    if let Ok(Some((_config_path, config))) = GitWorktreeConfig::find_config() {
        if let Some(project_path) = config.project_path {
            return Ok(project_path);
        }
    }

    Err(Error::Other(
        "Not in a git-worktree-cli project. Run 'gwt init' inside a git repository."
            .to_string(),
    ))
}

/// Check if a path is inside a -worktrees folder and return the main project path
fn find_main_project_from_worktree(worktree_path: &Path) -> Option<PathBuf> {
    // Walk up the path to see if any ancestor ends with -worktrees
    for ancestor in worktree_path.ancestors() {
        if let Some(name) = ancestor.file_name().and_then(|n| n.to_str()) {
            if name.ends_with("-worktrees") {
                // Found worktrees folder, derive main project path
                let main_name = name.trim_end_matches("-worktrees");
                if let Some(parent) = ancestor.parent() {
                    let main_project = parent.join(main_name);
                    // Check if main_project itself has .git
                    if main_project.join(".git").exists() {
                        return Some(main_project);
                    }
                    // Also check if main_project contains a subdirectory with .git
                    if let Ok(entries) = fs::read_dir(&main_project) {
                        for entry in entries.flatten() {
                            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                                let subdir = entry.path();
                                if subdir.join(".git").exists() {
                                    return Some(main_project);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

/// Check if start_path is inside a -worktrees folder structure
fn find_main_project_from_worktrees_path(start_path: &Path) -> Option<PathBuf> {
    for ancestor in start_path.ancestors() {
        if let Some(name) = ancestor.file_name().and_then(|n| n.to_str()) {
            if name.ends_with("-worktrees") {
                let main_name = name.trim_end_matches("-worktrees");
                if let Some(parent) = ancestor.parent() {
                    let main_project = parent.join(main_name);
                    // Check if main_project itself has .git
                    if main_project.join(".git").exists() {
                        return Some(main_project);
                    }
                    // Also check if main_project contains a subdirectory with .git
                    // (handles structures like agent-tools/main/.git)
                    if let Ok(entries) = fs::read_dir(&main_project) {
                        for entry in entries.flatten() {
                            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                                let subdir = entry.path();
                                if subdir.join(".git").exists() {
                                    return Some(main_project);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

/// Find the .git directory within a project
pub fn find_git_directory() -> Result<PathBuf> {
    let project_root = find_project_root()?;
    find_git_directory_from(&project_root)
}

/// Find the .git directory starting from a specific path
pub fn find_git_directory_from(project_root: &Path) -> Result<PathBuf> {
    // First check if the project root itself has a .git directory
    // This handles the case where config is inside main/ directory
    if project_root.join(".git").exists() {
        return Ok(project_root.to_path_buf());
    }

    let entries = fs::read_dir(project_root).map_err(Error::Io)?;

    for entry in entries {
        let entry = entry.map_err(Error::Io)?;
        if entry.file_type().map_err(Error::Io)?.is_dir() {
            let dir_path = entry.path();
            if dir_path.join(".git").exists() {
                // This is a git directory (worktree or regular repository)
                return Ok(dir_path);
            }
        }
    }

    Err(Error::GitDirectoryNotFound)
}

/// Find an existing git directory (worktree or main repository)
///
/// This function looks for any directory with a .git file or directory,
/// prioritizing worktrees (where .git is a file) over main repositories.
pub fn find_existing_worktree(project_root: &Path) -> Result<PathBuf> {
    // First check if the project root itself has a .git directory
    // This handles the case where config is inside main/ directory
    let root_git_path = project_root.join(".git");
    if root_git_path.exists() {
        if root_git_path.is_file() {
            // Project root is a worktree
            return Ok(project_root.to_path_buf());
        } else if root_git_path.is_dir() {
            // Project root is a main repository - save as fallback
            // But continue checking subdirectories for worktrees first
        }
    }

    let entries = fs::read_dir(project_root).map_err(Error::Io)?;

    let mut main_repo: Option<PathBuf> = None;

    // If project root has .git directory, use it as fallback
    if root_git_path.exists() && root_git_path.is_dir() {
        main_repo = Some(project_root.to_path_buf());
    }

    for entry in entries {
        let entry = entry.map_err(Error::Io)?;
        if entry.file_type().map_err(Error::Io)?.is_dir() {
            let dir_path = entry.path();
            let git_path = dir_path.join(".git");

            if git_path.exists() {
                if git_path.is_file() {
                    // This is a worktree - prefer these over main repos
                    return Ok(dir_path);
                } else if git_path.is_dir() {
                    // This is a main repository - save as fallback
                    main_repo = Some(dir_path);
                }
            }
        }
    }

    // If no worktree found, use main repository if available
    main_repo.ok_or_else(|| {
        Error::Other(format!(
            "No existing git directory found in project at {}. Have you run 'gwt init' yet?",
            project_root.display()
        ))
    })
}

/// Check if a path is an orphaned worktree
///
/// A worktree is orphaned if its .git file points to a non-existent gitdir path.
/// This can happen when a repository is moved to a new location.
pub fn is_orphaned_worktree(path: &Path) -> bool {
    let git_file = path.join(".git");

    // Check if .git is a file (worktree indicator)
    if !git_file.is_file() {
        return false;
    }

    // Read the .git file to get the gitdir path
    let Ok(content) = fs::read_to_string(&git_file) else {
        return false;
    };

    // Parse "gitdir: /path/to/gitdir"
    let Some(gitdir_line) = content.lines().find(|line| line.starts_with("gitdir: ")) else {
        return false;
    };

    let gitdir_path = gitdir_line.trim_start_matches("gitdir: ").trim();

    // Check if the gitdir path exists
    !Path::new(gitdir_path).exists()
}

/// Find a valid (non-orphaned) git directory in the project
///
/// This searches the project root for a git directory that is not orphaned.
/// Useful when the current directory is an orphaned worktree.
pub fn find_valid_git_directory(project_root: &Path) -> Result<PathBuf> {
    // Check if project root itself has valid .git directory
    let root_git_path = project_root.join(".git");
    if root_git_path.is_dir() {
        return Ok(project_root.to_path_buf());
    }

    // Search subdirectories for valid git directories
    let entries = fs::read_dir(project_root).map_err(Error::Io)?;

    for entry in entries {
        let entry = entry.map_err(Error::Io)?;
        if entry.file_type().map_err(Error::Io)?.is_dir() {
            let dir_path = entry.path();
            let git_path = dir_path.join(".git");

            if git_path.is_dir() {
                // Main repository - always valid
                return Ok(dir_path);
            } else if git_path.is_file() && !is_orphaned_worktree(&dir_path) {
                // Valid worktree (not orphaned)
                return Ok(dir_path);
            }
        }
    }

    Err(Error::Other(
        "No valid git directory found in project".to_string()
    ))
}

/// Clean a branch name by removing refs/heads/ prefix
pub fn clean_branch_name(branch: &str) -> &str {
    branch.trim().strip_prefix("refs/heads/").unwrap_or(branch.trim())
}
