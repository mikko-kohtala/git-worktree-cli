use std::env;

use crate::error::{Error, Result};

const TOKEN_ENV_VAR: &str = "BITBUCKET_DATA_CENTER_HTTP_ACCESS_TOKEN";

pub struct BitbucketDataCenterAuth;

impl BitbucketDataCenterAuth {
    pub fn new(_project_key: String, _repo_slug: String, _base_url: String) -> Result<Self> {
        Ok(BitbucketDataCenterAuth)
    }

    pub fn get_token(&self) -> Result<String> {
        env::var(TOKEN_ENV_VAR)
            .map_err(|_| {
                Error::auth(format!(
                    "No Bitbucket Data Center access token found. Please set the {} environment variable.\n\
                Run 'gwt auth bitbucket-data-center setup' for instructions.",
                    TOKEN_ENV_VAR
                ))
            })
            .and_then(|token| {
                if token.is_empty() {
                    Err(Error::auth(format!(
                        "Bitbucket Data Center access token is empty. Please set the {} environment variable.\n\
                        Run 'gwt auth bitbucket-data-center setup' for instructions.",
                        TOKEN_ENV_VAR
                    )))
                } else {
                    Ok(token)
                }
            })
    }
}

fn derive_api_base_url_from_repo_url(repo_url: &str) -> Option<String> {
    // Extract domain from various URL patterns and construct API base URL

    // Handle HTTPS URLs like https://github.com/owner/repo.git
    if let Some(captures) = regex::Regex::new(r"https://([^/]+)").ok()?.captures(repo_url) {
        let domain = captures.get(1)?.as_str();
        return Some(format!("https://{}", domain));
    }

    // Handle SSH URLs like git@github.com:owner/repo.git
    if let Some(captures) = regex::Regex::new(r"git@([^:]+):").ok()?.captures(repo_url) {
        let domain = captures.get(1)?.as_str();
        return Some(format!("https://{}", domain));
    }

    // Handle other SSH formats like ssh://git@domain/path
    if let Some(captures) = regex::Regex::new(r"ssh://git@([^/]+)").ok()?.captures(repo_url) {
        let domain = captures.get(1)?.as_str();
        return Some(format!("https://{}", domain));
    }

    None
}

pub fn get_auth_from_config() -> Result<(String, String, String)> {
    use crate::bitbucket_data_center_api::extract_bitbucket_data_center_info_from_url;
    use crate::config::GitWorktreeConfig;
    use crate::github;

    let (_, config) =
        GitWorktreeConfig::find_config()?.ok_or_else(|| Error::config("No git-worktree-config.jsonc found"))?;

    // Check sourceControl field instead of URL pattern
    if config.source_control != "bitbucket-data-center" {
        return Err(Error::provider(format!(
            "Repository is not configured for Bitbucket Data Center (sourceControl: {})",
            config.source_control
        )));
    }

    let repo_url = &config.repository_url;

    // First try to extract from actual Bitbucket Data Center URL
    if let Some((base_url, project_key, repo_slug)) = extract_bitbucket_data_center_info_from_url(repo_url) {
        return Ok((base_url, project_key, repo_slug));
    }

    // If that fails, try to derive from other URL patterns (like GitHub URLs)
    if let Some((owner, repo)) = github::GitHubClient::parse_github_url(repo_url) {
        // For GitHub URLs with bitbucket-data-center config, derive API base URL from the domain
        if let Some(base_url) = derive_api_base_url_from_repo_url(repo_url) {
            return Ok((base_url, owner, repo));
        }

        return Err(Error::provider(format!(
            "Failed to derive Bitbucket Data Center base URL from: {}",
            repo_url
        )));
    }

    Err(Error::provider(format!("Failed to parse repository URL: {}", repo_url)))
}

pub fn display_setup_instructions() {
    println!("Setting up Bitbucket Data Center authentication\n");
    println!("1. Create an HTTP Access Token in your Bitbucket Data Center instance:");
    println!("   - Go to your Bitbucket Data Center instance");
    println!("   - Navigate to your profile settings");
    println!("   - Go to 'HTTP access tokens' or 'Personal access tokens'");
    println!("   - Create a new token with repository permissions\n");
    println!("2. Required permissions for the token:");
    println!("   - Repository: Read");
    println!("   - Pull requests: Read\n");
    println!("3. Copy the generated token\n");
    println!("4. Set the environment variable:");
    println!("   export {}=YOUR_TOKEN", TOKEN_ENV_VAR);
    println!("\nExample usage:");
    println!("   curl -H \"Authorization: Bearer ${}\" \\", TOKEN_ENV_VAR);
    println!("        \"https://git.acmeorg.com/rest/api/1.0/projects/PROJECT/repos/REPO/pull-requests\"");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bitbucket_data_center_auth_creation() {
        let auth = BitbucketDataCenterAuth::new(
            "PROJECT".to_string(),
            "repository".to_string(),
            "https://git.acmeorg.com".to_string(),
        );
        assert!(auth.is_ok());
    }

    #[test]
    fn test_auth_structure() {
        let auth = BitbucketDataCenterAuth::new(
            "PROJ".to_string(),
            "repo".to_string(),
            "https://git.example.com".to_string(),
        );
        assert!(auth.is_ok());
    }
}
