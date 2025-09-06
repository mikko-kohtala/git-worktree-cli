//! Project discovery and management utilities
//!
//! This module handles finding project roots, git directories, and managing
//! project-related operations.

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
pub fn find_project_root_from(start_path: &Path) -> Result<PathBuf> {
    let mut search_path = start_path.to_path_buf();

    loop {
        if search_path.join("git-worktree-config.jsonc").exists() {
            return Ok(search_path);
        }

        if !search_path.pop() {
            break;
        }
    }

    // Check if we're in a git repository but missing config
    if let Ok(Some(_)) = crate::git::get_git_root() {
        Err(Error::Other(
            "Found git repository but no git-worktree-config.jsonc. This doesn't appear to be a worktree project."
                .to_string(),
        ))
    } else {
        Err(Error::ProjectRootNotFound)
    }
}

/// Find the .git directory within a project
pub fn find_git_directory() -> Result<PathBuf> {
    let project_root = find_project_root()?;
    find_git_directory_from(&project_root)
}

/// Find the .git directory starting from a specific path
pub fn find_git_directory_from(project_root: &Path) -> Result<PathBuf> {
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
    let entries = fs::read_dir(project_root).map_err(Error::Io)?;

    let mut main_repo: Option<PathBuf> = None;

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
    main_repo.ok_or_else(|| Error::Other("No existing git directory found in project".to_string()))
}

/// Clean a branch name by removing refs/heads/ prefix
pub fn clean_branch_name(branch: &str) -> &str {
    branch.trim().strip_prefix("refs/heads/").unwrap_or(branch.trim())
}
// Core utility functions
//
// This module contains utility functions used throughout the core module.

/// Check if a path looks like a git SSH URL
pub fn is_git_ssh_url(url: &str) -> bool {
    url.starts_with("git@") || url.contains(":")
}

/// Convert SSH URL to HTTPS URL for cloning
pub fn ssh_to_https_url(url: &str) -> String {
    if url.starts_with("git@") {
        // Convert git@github.com:user/repo.git to https://github.com/user/repo.git
        url.replace(":", "/").replace("git@", "https://")
    } else {
        url.to_string()
    }
}

/// Get the repository name from a URL
pub fn get_repo_name_from_url(url: &str) -> Option<String> {
    let path = url.strip_suffix(".git").unwrap_or(url);

    Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .map(|s| s.to_string())
}

/// Check if a branch name is a main branch (shouldn't be deleted)
pub fn is_main_branch(branch_name: &str) -> bool {
    matches!(branch_name, "main" | "master" | "develop" | "dev")
}
