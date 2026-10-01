use colored::Colorize;
use std::fs;
use std::path::{Path, PathBuf};

use crate::cli::Provider;
use crate::config::{generate_config_filename, GitWorktreeConfig, Settings, CONFIG_FILENAME};
use crate::error::{Error, Result};
use crate::git;
use crate::{azure_devops, bitbucket_api, github};

/// Initialize git-worktree-cli for an existing repository
pub fn run(local: bool, worktrees_root: Option<PathBuf>) -> Result<()> {
    // Check if we're in a git repository
    let git_root = git::get_git_root()?
        .ok_or_else(|| Error::git("Not in a git repository. Please run this command from inside a git repository."))?;

    // Get the remote URL
    let repo_urls = git::get_remote_origin_urls(&git_root);
    let first_url = repo_urls
        .first()
        .ok_or_else(|| Error::git("No remote 'origin' found. Please add a remote first."))?;

    // Detect the repository provider: the configured URL first, the insteadOf-rewritten one if
    // only it names a known provider
    let (repo_url, detected_provider) = repo_urls
        .iter()
        .find_map(|url| detect_provider_from_url(url).map(|provider| (url.clone(), provider)))
        .ok_or_else(|| create_provider_error(first_url))?;

    println!("{}", format!("✓ Detected provider: {:?}", detected_provider).green());

    // Get the default branch name from the remote
    let current_branch = git::get_remote_default_branch(&git_root)
        .map_err(|e| Error::git(format!("Failed to detect default branch: {}", e)))?;

    // Use the git root as the project path
    let project_path = git_root.canonicalize().unwrap_or_else(|_| git_root.clone());

    // Derive the worktrees path: under the worktrees root when one is given
    // (flag, then settings), otherwise <parent>/.worktrees/<repo-name>
    let worktrees_root = match worktrees_root {
        Some(root) => Some(root),
        None => Settings::load()?.worktrees_root,
    };
    let worktrees_path = GitWorktreeConfig::default_worktrees_path(&project_path, worktrees_root.as_deref());
    ensure_worktrees_path_is_free(&worktrees_path, &project_path)?;

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

/// Refuse a worktrees folder that another project already uses, either by
/// config or with worktrees on disk. Two project paths can encode to the same
/// folder name under a worktrees root (code/my-app and code-my/app).
fn ensure_worktrees_path_is_free(worktrees_path: &Path, project_path: &Path) -> Result<()> {
    let in_use_by = |other: &Path| {
        Error::config(format!(
            "Worktrees folder {} is already used by {}.\n\
             Run 'gwt init' with a different --worktrees-root, or set worktreesPath in the config by hand.",
            worktrees_path.display(),
            other.display()
        ))
    };

    for (_, config) in GitWorktreeConfig::global_configs()? {
        if let (Some(other_worktrees), Some(other_project)) = (&config.worktrees_path, &config.project_path) {
            if other_worktrees == worktrees_path && other_project != project_path {
                return Err(in_use_by(other_project));
            }
        }
    }

    let own_git_dir = git::get_common_dir(project_path).and_then(|dir| dir.canonicalize().ok());
    if let Some(other_git_dir) = find_foreign_worktree(worktrees_path, own_git_dir.as_deref(), 0) {
        let other = other_git_dir.parent().unwrap_or(&other_git_dir).to_path_buf();
        return Err(in_use_by(&other));
    }
    Ok(())
}

/// The `.git` directory of the first worktree under `dir` that belongs to a
/// repository other than `own_git_dir`. Branch folders nest (feature/x), so
/// this descends until it finds a worktree.
fn find_foreign_worktree(dir: &Path, own_git_dir: Option<&Path>, depth: usize) -> Option<PathBuf> {
    const MAX_DEPTH: usize = 6;
    let entries = fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if !entry.file_type().is_ok_and(|t| t.is_dir()) {
            continue;
        }
        let git_file = path.join(".git");
        if git_file.is_file() {
            // "gitdir: <repo>/.git/worktrees/<name>"
            let Some(common_dir) = fs::read_to_string(&git_file).ok().and_then(|content| {
                let gitdir = content
                    .lines()
                    .find_map(|l| l.strip_prefix("gitdir: "))?
                    .trim()
                    .to_string();
                // Relative with worktree.useRelativePaths; join keeps absolute paths as they are
                let gitdir = path.join(gitdir);
                let common_dir = gitdir.parent()?.parent()?;
                Some(common_dir.canonicalize().unwrap_or_else(|_| common_dir.to_path_buf()))
            }) else {
                continue;
            };
            if own_git_dir != Some(common_dir.as_path()) {
                return Some(common_dir);
            }
        } else if depth < MAX_DEPTH {
            if let Some(found) = find_foreign_worktree(&path, own_git_dir, depth + 1) {
                return Some(found);
            }
        }
    }
    None
}

fn detect_provider_from_url(repo_url: &str) -> Option<Provider> {
    if github::GitHubClient::parse_github_url(repo_url).is_some() {
        Some(Provider::Github)
    } else if bitbucket_api::is_bitbucket_repository(repo_url) {
        Some(Provider::BitbucketCloud)
    } else if azure_devops::AzureDevOpsClient::parse_azure_url(repo_url).is_some() {
        Some(Provider::AzureDevops)
    } else {
        None
    }
}

fn create_provider_error(repo_url: &str) -> Error {
    Error::provider(format!(
        "Could not detect repository provider from URL: {}\n\
         Supported providers: GitHub, Bitbucket Cloud, Azure DevOps",
        repo_url
    ))
}
