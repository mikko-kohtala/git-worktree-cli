//! Core utility functions
//!
//! This module contains utility functions used throughout the core module.

use std::path::Path;

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
    let path = if url.ends_with(".git") {
        &url[..url.len() - 4]
    } else {
        url
    };

    Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .map(|s| s.to_string())
}

/// Check if a branch name is a main branch (shouldn't be deleted)
pub fn is_main_branch(branch_name: &str) -> bool {
    matches!(branch_name, "main" | "master" | "develop" | "dev")
}
