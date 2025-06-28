use anyhow::{Context, Result};
use keyring::Entry;
use std::env;

const SERVICE_NAME: &str = "git-worktree-cli-bitbucket";
const EMAIL_ENV_VAR: &str = "BITBUCKET_CLOUD_EMAIL";
const TOKEN_ENV_VAR: &str = "BITBUCKET_CLOUD_API_TOKEN";

pub struct BitbucketAuth {
    workspace: String,
    repo: String,
    email: Option<String>,
    token_entry: Entry,
}

impl BitbucketAuth {
    pub fn new(workspace: String, repo: String, email: Option<String>) -> Result<Self> {
        // Use workspace/repo as the key identifier for better isolation
        let key_id = format!("{}/{}", workspace, repo);
        let token_entry = Entry::new(SERVICE_NAME, &key_id)
            .context("Failed to create keyring entry for Bitbucket token")?;
        
        Ok(BitbucketAuth {
            workspace,
            repo,
            email,
            token_entry,
        })
    }

    pub fn store_token(&self, token: &str) -> Result<()> {
        self.token_entry
            .set_password(token)
            .context("Failed to store Bitbucket API token in keyring")?;
        
        println!("✓ Bitbucket API token stored securely for {}/{}", self.workspace, self.repo);
        Ok(())
    }

    pub fn get_token(&self) -> Result<String> {
        // Check environment variable
        if let Ok(token) = env::var(TOKEN_ENV_VAR) {
            if !token.is_empty() {
                return Ok(token);
            }
        }
        
        // Then check keyring
        self.token_entry
            .get_password()
            .context(format!(
                "No Bitbucket Cloud API token found. Please set the {} and {} environment variables.\n\
                Run 'gwt auth bitbucket-cloud setup' for instructions.",
                EMAIL_ENV_VAR, TOKEN_ENV_VAR
            ))
    }

    pub fn remove_token(&self) -> Result<()> {
        self.token_entry
            .delete_credential()
            .context("Failed to remove Bitbucket API token from keyring")?;
        
        println!("✓ Bitbucket API token removed for {}/{}", self.workspace, self.repo);
        Ok(())
    }

    pub fn email(&self) -> Option<String> {
        // First check environment variable
        if let Ok(email) = env::var(EMAIL_ENV_VAR) {
            if !email.is_empty() {
                return Some(email);
            }
        }
        
        self.email.clone()
    }

    pub fn has_stored_token(&self) -> bool {
        // Check env var first
        if let Ok(token) = env::var(TOKEN_ENV_VAR) {
            if !token.is_empty() {
                return true;
            }
        }
        
        // Then check keyring
        self.token_entry.get_password().is_ok()
    }
}

pub fn get_auth_from_config() -> Result<(String, String, Option<String>)> {
    use crate::config::GitWorktreeConfig;
    use crate::bitbucket_api::extract_bitbucket_info_from_url;
    
    let (_, config) = GitWorktreeConfig::find_config()?
        .ok_or_else(|| anyhow::anyhow!("No git-worktree-config.yaml found"))?;
    
    if !config.repository_url.contains("bitbucket.org") {
        return Err(anyhow::anyhow!("This is not a Bitbucket repository"));
    }
    
    let (workspace, repo) = extract_bitbucket_info_from_url(&config.repository_url)
        .ok_or_else(|| anyhow::anyhow!("Failed to parse Bitbucket repository URL"))?;
    
    Ok((workspace, repo, config.bitbucket_email))
}

pub fn display_setup_instructions() {
    println!("Setting up Bitbucket Cloud authentication\n");
    println!("1. Create an API token (App Password) at:");
    println!("   https://bitbucket.org/account/settings/app-passwords/\n");
    println!("2. Required permissions for the token:");
    println!("   - Repositories: Read");
    println!("   - Pull requests: Read\n");
    println!("3. Copy the generated token\n");
    println!("4. Set environment variables:");
    println!("   export {}=your-email@example.com", EMAIL_ENV_VAR);
    println!("   export {}=YOUR_TOKEN", TOKEN_ENV_VAR);
    println!("\nNote: The email should match your Bitbucket account email.");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bitbucket_auth_creation() {
        let auth = BitbucketAuth::new(
            "myworkspace".to_string(),
            "myrepo".to_string(),
            Some("test@example.com".to_string())
        );
        assert!(auth.is_ok());
        
        let auth = auth.unwrap();
        assert_eq!(auth.workspace, "myworkspace");
        assert_eq!(auth.repo, "myrepo");
        assert_eq!(auth.email(), Some("test@example.com".to_string()));
    }

    #[test]
    fn test_workspace_repo_key() {
        let auth = BitbucketAuth::new(
            "workspace".to_string(),
            "repo".to_string(),
            None
        ).unwrap();
        
        // The key should be workspace/repo
        assert_eq!(auth.workspace, "workspace");
        assert_eq!(auth.repo, "repo");
    }
}