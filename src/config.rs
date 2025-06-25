use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use anyhow::{Result, Context};
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitWorktreeConfig {
    pub repository_url: String,
    pub main_branch: String,
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hooks: Option<Hooks>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Hooks {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_add: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_remove: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_init: Option<Vec<String>>,
}

impl GitWorktreeConfig {
    pub fn new(repository_url: String, main_branch: String) -> Self {
        Self {
            repository_url,
            main_branch,
            created_at: Utc::now(),
            hooks: Some(Hooks {
                post_add: Some(vec!["# npm install".to_string()]),
                post_remove: Some(vec!["# echo 'Removed worktree for branch ${branchName}'".to_string()]),
                post_init: Some(vec!["# echo 'Initialized git worktree project'".to_string()]),
            }),
        }
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let yaml_string = serde_yaml::to_string(self)
            .context("Failed to serialize config to YAML")?;
        
        fs::write(path, yaml_string)
            .context("Failed to write config file")?;
        
        Ok(())
    }

    pub fn load(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .context("Failed to read config file")?;
        
        let config: Self = serde_yaml::from_str(&content)
            .context("Failed to parse YAML config")?;
        
        Ok(config)
    }

    pub fn find_config() -> Result<Option<(PathBuf, Self)>> {
        let mut current_dir = std::env::current_dir()?;
        
        loop {
            let config_path = current_dir.join("git-worktree-config.yaml");
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

pub const CONFIG_FILENAME: &str = "git-worktree-config.yaml";

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs;

    #[test]
    fn test_config_creation() {
        let config = GitWorktreeConfig::new(
            "git@github.com:test/repo.git".to_string(),
            "main".to_string(),
        );
        
        assert_eq!(config.repository_url, "git@github.com:test/repo.git");
        assert_eq!(config.main_branch, "main");
        assert!(config.hooks.is_some());
        
        let hooks = config.hooks.unwrap();
        assert!(hooks.post_add.is_some());
        assert!(hooks.post_remove.is_some());
        assert!(hooks.post_init.is_some());
    }

    #[test]
    fn test_config_save_and_load() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("test-config.yaml");
        
        let original_config = GitWorktreeConfig::new(
            "git@github.com:test/repo.git".to_string(),
            "develop".to_string(),
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
        
        // Restore original directory
        std::env::set_current_dir(original_cwd).unwrap();
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
        
        // Restore original directory
        std::env::set_current_dir(original_cwd).unwrap();
    }
}