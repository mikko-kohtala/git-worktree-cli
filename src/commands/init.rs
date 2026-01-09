use colored::Colorize;
use std::fs;

use crate::cli::Provider;
use crate::config::{generate_config_filename, GitWorktreeConfig, CONFIG_FILENAME};
use crate::error::{Error, Result};
use crate::git;
use crate::{bitbucket_api, github};

/// Initialize git-worktree-cli for an existing repository
pub fn run(local: bool) -> Result<()> {
    // Check if we're in a git repository
    let git_root = git::get_git_root()?
        .ok_or_else(|| Error::git("Not in a git repository. Please run this command from inside a git repository."))?;

    // Get the remote URL
    let repo_url = git::get_remote_origin_url(&git_root)
        .ok_or_else(|| Error::git("No remote 'origin' found. Please add a remote first."))?;

    // Detect the repository provider
    let detected_provider = detect_provider_from_url(&repo_url)
        .ok_or_else(|| create_provider_error(&repo_url))?;

    println!("{}", format!("✓ Detected provider: {:?}", detected_provider).green());

    // Get the current branch name (this will be the main branch)
    let current_branch = git::get_default_branch(&git_root)
        .map_err(|e| Error::git(format!("Failed to get current branch: {}", e)))?;

    // Use the git root as the project path
    let project_path = git_root.canonicalize().unwrap_or_else(|_| git_root.clone());

    // Derive the worktrees path (repo-name -> repo-name-worktrees)
    let worktrees_path = GitWorktreeConfig::derive_worktrees_path(&project_path);

    // Create configuration
    let config = GitWorktreeConfig::new(
        repo_url.clone(),
        current_branch.clone(),
        detected_provider,
        Some(project_path.clone()),
        Some(worktrees_path.clone()),
    );

    // Determine config location
    let config_path = if local {
        // For local, put config in the parent directory (next to the repo)
        project_path
            .parent()
            .map(|p| p.join(CONFIG_FILENAME))
            .unwrap_or_else(|| project_path.join(CONFIG_FILENAME))
    } else {
        let projects_dir = GitWorktreeConfig::projects_config_dir()?;
        fs::create_dir_all(&projects_dir)
            .map_err(|e| Error::config(format!("Failed to create config directory: {}", e)))?;
        let filename = generate_config_filename(&repo_url);
        projects_dir.join(filename)
    };

    config
        .save(&config_path)
        .map_err(|e| Error::config(format!("Failed to save configuration: {}", e)))?;

    // Print success messages
    println!("{}", format!("✓ Repository: {}", repo_url).green());
    println!("{}", format!("✓ Main branch: {}", current_branch).green());
    println!("{}", format!("✓ Project path: {}", project_path.display()).green());
    println!("{}", format!("✓ Worktrees path: {}", worktrees_path.display()).green());
    println!("{}", format!("✓ Config saved to: {}", config_path.display()).green());

    if !local {
        println!("{}", "  (Use --local to store config in project directory)".dimmed());
    }

    Ok(())
}

fn detect_provider_from_url(repo_url: &str) -> Option<Provider> {
    if github::GitHubClient::parse_github_url(repo_url).is_some() {
        Some(Provider::Github)
    } else if bitbucket_api::is_bitbucket_repository(repo_url) {
        Some(Provider::BitbucketCloud)
    } else {
        None
    }
}

fn create_provider_error(repo_url: &str) -> Error {
    Error::provider(format!(
        "Could not detect repository provider from URL: {}\n\
         Supported providers: GitHub, Bitbucket Cloud",
        repo_url
    ))
}
