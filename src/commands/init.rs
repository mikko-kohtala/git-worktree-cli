use anyhow::{Context, Result};
use colored::Colorize;
use std::fs;
use std::path::{Path, PathBuf};

use crate::cli::Provider;
use crate::config::{GitWorktreeConfig, CONFIG_FILENAME};
use crate::git;
use crate::{bitbucket_api, github};

pub fn run(repo_url: &str, provider: Option<Provider>) -> Result<()> {
    // Detect or validate the repository provider
    let detected_provider = detect_repository_provider(repo_url, provider)?;

    println!("{}", format!("✓ Detected provider: {:?}", detected_provider).green());

    // Extract repository name from URL
    let repo_name = extract_repo_name(repo_url)?;
    let project_root = std::env::current_dir()?;

    // Remove existing clone directory if it exists
    if Path::new(&repo_name).exists() {
        fs::remove_dir_all(&repo_name).context("Failed to remove existing directory")?;
    }

    // Clone the repository with streaming output (this is the key improvement!)
    git::clone(repo_url, &repo_name)?;

    // Get the default branch name
    let repo_path = PathBuf::from(&repo_name);
    let default_branch = git::get_default_branch(&repo_path).context("Failed to get default branch")?;

    // Rename directory to match branch name
    let final_dir_name = &default_branch;
    if Path::new(final_dir_name).exists() {
        fs::remove_dir_all(final_dir_name).context("Failed to remove existing directory")?;
    }

    fs::rename(&repo_name, final_dir_name).context("Failed to rename directory")?;

    // Create configuration file
    let config = GitWorktreeConfig::new(repo_url.to_string(), default_branch.clone(), detected_provider);
    let config_path = project_root.join(CONFIG_FILENAME);
    config.save(&config_path).context("Failed to save configuration")?;

    // Print success messages
    println!("{}", format!("✓ Repository cloned to: {}", final_dir_name).green());
    println!("{}", format!("✓ Default branch: {}", default_branch).green());
    println!("{}", format!("✓ Config saved to: {}", config_path.display()).green());

    // Post-init hooks removed - no longer needed

    Ok(())
}

fn extract_repo_name(repo_url: &str) -> Result<String> {
    let name = repo_url
        .split('/')
        .next_back()
        .context("Invalid repository URL")?
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

fn create_provider_error(repo_url: &str) -> anyhow::Error {
    anyhow::anyhow!(
        "Could not detect repository provider from URL: {}\n\
         Please specify the provider using --provider:\n\
         - For GitHub: --provider github\n\
         - For Bitbucket Cloud: --provider bitbucket-cloud\n\
         - For Bitbucket Data Center: --provider bitbucket-data-center",
        repo_url
    )
}
