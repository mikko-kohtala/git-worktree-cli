use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::cli::Provider;
use crate::error::{Error, Result};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitWorktreeConfig {
    pub repository_url: String,
    pub main_branch: String,
    pub created_at: DateTime<Utc>,
    pub source_control: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bitbucket_email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hooks: Option<Hooks>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Hooks {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_add: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pre_remove: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_remove: Option<Vec<String>>,
}

impl GitWorktreeConfig {
    pub fn new(repository_url: String, main_branch: String, provider: Provider) -> Self {
        // Convert provider enum to string
        let source_control = match provider {
            Provider::Github => "github".to_string(),
            Provider::BitbucketCloud => "bitbucket-cloud".to_string(),
            Provider::BitbucketDataCenter => "bitbucket-data-center".to_string(),
        };

        Self {
            repository_url,
            main_branch,
            created_at: Utc::now(),
            source_control,
            bitbucket_email: None,
            hooks: Some(Hooks {
                post_add: Some(vec![]),
                pre_remove: Some(vec![]),
                post_remove: Some(vec![]),
            }),
        }
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let json_string = serde_json::to_string_pretty(self)?;

        fs::write(path, json_string).map_err(|e| Error::config(format!("Failed to write config file: {}", e)))?;

        Ok(())
    }

    pub fn load(path: &Path) -> Result<Self> {
        let content =
            fs::read_to_string(path).map_err(|e| Error::config(format!("Failed to read config file: {}", e)))?;

        let config: Self = json5::from_str(&content)?;

        Ok(config)
    }

    pub fn find_config() -> Result<Option<(PathBuf, Self)>> {
        let mut current_dir = std::env::current_dir()?;

        loop {
            let config_path = current_dir.join("git-worktree-config.jsonc");
            if config_path.exists() {
                let config = Self::load(&config_path)?;
                return Ok(Some((config_path, config)));
            }

            if !current_dir.pop() {
                break;
            }
        }

        Ok(None)
    }
}

pub const CONFIG_FILENAME: &str = "git-worktree-config.jsonc";

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_config_creation() {
        let config = GitWorktreeConfig::new(
            "git@github.com:test/repo.git".to_string(),
            "main".to_string(),
            Provider::Github,
        );

        assert_eq!(config.repository_url, "git@github.com:test/repo.git");
        assert_eq!(config.main_branch, "main");
        assert_eq!(config.source_control, "github");
        assert_eq!(config.bitbucket_email, None);
        assert!(config.hooks.is_some());

        let hooks = config.hooks.unwrap();
        assert!(hooks.post_add.is_some());
        assert!(hooks.pre_remove.is_some());
        assert!(hooks.post_remove.is_some());
    }

    #[test]
    fn test_config_creation_bitbucket() {
        let config = GitWorktreeConfig::new(
            "https://bitbucket.org/workspace/repo.git".to_string(),
            "main".to_string(),
            Provider::BitbucketCloud,
        );

        assert_eq!(config.repository_url, "https://bitbucket.org/workspace/repo.git");
        assert_eq!(config.main_branch, "main");
        assert_eq!(config.source_control, "bitbucket-cloud");
        assert_eq!(config.bitbucket_email, None);
    }

    #[test]
    fn test_config_creation_bitbucket_data_center() {
        let config = GitWorktreeConfig::new(
            "https://bitbucket.company.com/scm/project/repo.git".to_string(),
            "main".to_string(),
            Provider::BitbucketDataCenter,
        );

        assert_eq!(
            config.repository_url,
            "https://bitbucket.company.com/scm/project/repo.git"
        );
        assert_eq!(config.main_branch, "main");
        assert_eq!(config.source_control, "bitbucket-data-center");
        assert_eq!(config.bitbucket_email, None);
    }

    #[test]
    fn test_config_save_and_load() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("test-config.jsonc");

        let original_config = GitWorktreeConfig::new(
            "git@github.com:test/repo.git".to_string(),
            "develop".to_string(),
            Provider::Github,
        );

        // Save config
        original_config.save(&config_path).unwrap();
        assert!(config_path.exists());

        // Load config
        let loaded_config = GitWorktreeConfig::load(&config_path).unwrap();
        assert_eq!(loaded_config.repository_url, original_config.repository_url);
        assert_eq!(loaded_config.main_branch, original_config.main_branch);
    }

    #[test]
    fn test_config_find_in_current_dir() {
        let temp_dir = tempdir().unwrap();
        let original_cwd = std::env::current_dir().unwrap();

        // Create config in temp directory first
        let config = GitWorktreeConfig::new(
            "git@github.com:test/repo.git".to_string(),
            "main".to_string(),
            Provider::Github,
        );
        config.save(&temp_dir.path().join(CONFIG_FILENAME)).unwrap();

        // Change to temp directory
        std::env::set_current_dir(temp_dir.path()).unwrap();

        // Find config should return the config
        let result = GitWorktreeConfig::find_config().unwrap();
        assert!(result.is_some());

        let (_found_path, found_config) = result.unwrap();
        assert_eq!(found_config.repository_url, "git@github.com:test/repo.git");
        assert_eq!(found_config.main_branch, "main");

        // Restore original directory before temp_dir is dropped
        // Use unwrap_or_else to handle case where original_cwd may not exist
        if original_cwd.exists() {
            std::env::set_current_dir(&original_cwd).unwrap();
        } else {
            // Fallback to a directory that should exist
            std::env::set_current_dir("/").unwrap();
        }
    }

    #[test]
    fn test_config_not_found() {
        let temp_dir = tempdir().unwrap();
        let original_cwd = std::env::current_dir().unwrap();

        // Change to empty temp directory
        std::env::set_current_dir(temp_dir.path()).unwrap();

        // Find config should return None
        let result = GitWorktreeConfig::find_config().unwrap();
        assert!(result.is_none());

        // Restore original directory before temp_dir is dropped
        // Use unwrap_or_else to handle case where original_cwd may not exist
        if original_cwd.exists() {
            std::env::set_current_dir(&original_cwd).unwrap();
        } else {
            // Fallback to a directory that should exist
            std::env::set_current_dir("/").unwrap();
        }
    }
}
