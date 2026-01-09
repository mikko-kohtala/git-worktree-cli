use colored::Colorize;
use std::fs;
use std::path::{Path, PathBuf};

use crate::cli::Provider;
use crate::config::{generate_config_filename, GitWorktreeConfig, CONFIG_FILENAME};
use crate::error::{Error, Result};
use crate::git;
use crate::{bitbucket_api, github};

pub fn run(repo_url: Option<&str>, provider: Option<Provider>, force: bool, local: bool) -> Result<()> {
    match repo_url {
        Some(url) => run_clone(url, provider, force, local),
        None => run_existing(provider, local),
    }
}

/// Initialize by cloning a repository
fn run_clone(repo_url: &str, provider: Option<Provider>, force: bool, local: bool) -> Result<()> {
    // Detect or validate the repository provider
    let detected_provider = detect_repository_provider(repo_url, provider)?;

    println!("{}", format!("✓ Detected provider: {:?}", detected_provider).green());

    // Extract repository name from URL
    let repo_name = extract_repo_name(repo_url)?;
    let project_root = std::env::current_dir()?;

    // Check for existing clone directory
    if Path::new(&repo_name).exists() {
        if !force {
            return Err(Error::msg(format!(
                "Directory '{}' already exists. Use --force to overwrite or remove it manually.",
                repo_name
            )));
        }
        fs::remove_dir_all(&repo_name)
            .map_err(|e| Error::msg(format!("Failed to remove existing directory: {}", e)))?;
    }

    // Clone the repository with streaming output
    git::clone(repo_url, &repo_name)?;

    // Get the default branch name
    let repo_path = PathBuf::from(&repo_name);
    let default_branch =
        git::get_default_branch(&repo_path).map_err(|e| Error::git(format!("Failed to get default branch: {}", e)))?;

    // Sanitize branch name for use as directory name
    let final_dir_name = default_branch.replace(['/', '\\'], "-");

    // Check for existing branch directory
    if Path::new(&final_dir_name).exists() {
        if !force {
            return Err(Error::msg(format!(
                "Directory '{}' already exists. Use --force to overwrite or remove it manually.",
                final_dir_name
            )));
        }
        fs::remove_dir_all(&final_dir_name)
            .map_err(|e| Error::msg(format!("Failed to remove existing directory: {}", e)))?;
    }

    fs::rename(&repo_name, &final_dir_name).map_err(|e| Error::msg(format!("Failed to rename directory: {}", e)))?;

    // Create configuration file with project path
    let absolute_project_root = project_root.join(&final_dir_name).canonicalize().unwrap_or_else(|_| project_root.join(&final_dir_name));
    let config = GitWorktreeConfig::new(
        repo_url.to_string(),
        default_branch.clone(),
        detected_provider,
        Some(absolute_project_root),
    );

    // Determine config location
    let config_path = if local {
        project_root.join(CONFIG_FILENAME)
    } else {
        let projects_dir = GitWorktreeConfig::projects_config_dir()?;
        fs::create_dir_all(&projects_dir)
            .map_err(|e| Error::config(format!("Failed to create config directory: {}", e)))?;
        let filename = generate_config_filename(repo_url);
        projects_dir.join(filename)
    };

    config
        .save(&config_path)
        .map_err(|e| Error::config(format!("Failed to save configuration: {}", e)))?;

    // Print success messages
    println!("{}", format!("✓ Repository cloned to: {}", final_dir_name).green());
    println!("{}", format!("✓ Default branch: {}", default_branch).green());
    println!("{}", format!("✓ Config saved to: {}", config_path.display()).green());

    if !local {
        println!("{}", "  (Use --local to store config in project directory)".dimmed());
    }

    Ok(())
}

/// Initialize an existing repository (no cloning)
fn run_existing(provider: Option<Provider>, local: bool) -> Result<()> {
    let current_dir = std::env::current_dir()?;

    // Check if we're in a git repository
    let git_root = git::get_git_root()?
        .ok_or_else(|| Error::git("Not in a git repository. Run 'gwt init <url>' to clone a new repository."))?;

    // Get the remote URL
    let repo_url = git::get_remote_origin_url(&git_root)
        .ok_or_else(|| Error::git("No remote 'origin' found. Please add a remote or use 'gwt init <url>'."))?;

    // Detect or validate the repository provider
    let detected_provider = detect_repository_provider(&repo_url, provider)?;

    println!("{}", format!("✓ Detected provider: {:?}", detected_provider).green());

    // Get the current branch name
    let current_branch = git::get_default_branch(&current_dir)
        .map_err(|e| Error::git(format!("Failed to get current branch: {}", e)))?;

    // Use the git root as the project path
    let absolute_project_root = git_root.canonicalize().unwrap_or_else(|_| git_root.clone());

    // Create configuration
    let config = GitWorktreeConfig::new(
        repo_url.clone(),
        current_branch.clone(),
        detected_provider,
        Some(absolute_project_root.clone()),
    );

    // Determine config location
    let config_path = if local {
        // For local, put it in the parent of git root (worktree project root)
        absolute_project_root.parent()
            .map(|p| p.join(CONFIG_FILENAME))
            .unwrap_or_else(|| absolute_project_root.join(CONFIG_FILENAME))
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
    println!("{}", format!("✓ Config saved to: {}", config_path.display()).green());

    if !local {
        println!("{}", "  (Use --local to store config in project directory)".dimmed());
    }

    Ok(())
}

fn extract_repo_name(repo_url: &str) -> Result<String> {
    let name = repo_url
        .split('/')
        .next_back()
        .ok_or_else(|| Error::msg("Invalid repository URL"))?
        .strip_suffix(".git")
        .unwrap_or_else(|| repo_url.split('/').next_back().unwrap());

    Ok(name.to_string())
}

fn detect_repository_provider(repo_url: &str, provider: Option<Provider>) -> Result<Provider> {
    let auto_detected = detect_provider_from_url(repo_url);

    match provider {
        // Use explicit provider if provided
        Some(explicit) => {
            if let Some(detected) = auto_detected {
                if !providers_match(&detected, &explicit) {
                    warn_provider_mismatch(&detected, &explicit);
                }
            }
            Ok(explicit)
        }

        // Use auto-detected if no explicit provider
        None => match auto_detected {
            Some(detected) => Ok(detected),
            None => Err(create_provider_error(repo_url)),
        },
    }
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

fn providers_match(a: &Provider, b: &Provider) -> bool {
    std::mem::discriminant(a) == std::mem::discriminant(b)
}

fn warn_provider_mismatch(detected: &Provider, explicit: &Provider) {
    println!(
        "{}",
        format!(
            "⚠ URL suggests {:?} but --provider {:?} specified. Using {:?}.",
            detected, explicit, explicit
        )
        .yellow()
    );
}

fn create_provider_error(repo_url: &str) -> Error {
    Error::provider(format!(
        "Could not detect repository provider from URL: {}\n\
         Please specify the provider using --provider:\n\
         - For GitHub: --provider github\n\
         - For Bitbucket Cloud: --provider bitbucket-cloud\n\
         - For Bitbucket Data Center: --provider bitbucket-data-center",
        repo_url
    ))
}
