use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::cli::Provider;
use crate::error::{Error, Result};
use crate::git;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitWorktreeConfig {
    pub repository_url: String,
    pub main_branch: String,
    pub created_at: DateTime<Utc>,
    pub source_control: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_path: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worktrees_path: Option<PathBuf>,
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
    pub fn new(
        repository_url: String,
        main_branch: String,
        provider: Provider,
        project_path: Option<PathBuf>,
        worktrees_path: Option<PathBuf>,
    ) -> Self {
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
            project_path,
            worktrees_path,
            bitbucket_email: None,
            hooks: Some(Hooks {
                post_add: Some(vec![]),
                pre_remove: Some(vec![]),
                post_remove: Some(vec![]),
            }),
        }
    }

    /// Derive worktrees path from project path (repo-name -> repo-name-worktrees)
    pub fn derive_worktrees_path(project_path: &Path) -> PathBuf {
        let repo_name = project_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("repo");
        project_path
            .parent()
            .map(|p| p.join(format!("{}-worktrees", repo_name)))
            .unwrap_or_else(|| project_path.join("worktrees"))
    }

    /// Get worktrees path, deriving from project_path if not stored
    pub fn get_worktrees_path(&self) -> Option<PathBuf> {
        self.worktrees_path.clone().or_else(|| {
            self.project_path
                .as_ref()
                .map(|p| Self::derive_worktrees_path(p))
        })
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

    /// Find configuration for the current project
    /// Priority: Local config first, then global config
    pub fn find_config() -> Result<Option<(PathBuf, Self)>> {
        let current_dir = std::env::current_dir()?;

        // Step 1: Check for local config (walk up directory tree)
        if let Some(result) = Self::find_local_config(&current_dir)? {
            return Ok(Some(result));
        }

        // Step 2: Try to find global config
        if let Some(result) = Self::find_global_config(&current_dir)? {
            return Ok(Some(result));
        }

        Ok(None)
    }

    /// Find local config by walking up directory tree
    fn find_local_config(start_dir: &Path) -> Result<Option<(PathBuf, Self)>> {
        let mut current_dir = start_dir.to_path_buf();

        loop {
            // Check in current directory
            let config_path = current_dir.join(CONFIG_FILENAME);
            if config_path.exists() {
                let config = Self::load(&config_path)?;
                return Ok(Some((config_path, config)));
            }

            // Check in ./main/ subdirectory
            let main_config_path = current_dir.join("main").join(CONFIG_FILENAME);
            if main_config_path.exists() {
                let config = Self::load(&main_config_path)?;
                return Ok(Some((main_config_path, config)));
            }

            if !current_dir.pop() {
                break;
            }
        }

        Ok(None)
    }

    /// Find global config by matching repository URL or project path
    fn find_global_config(start_dir: &Path) -> Result<Option<(PathBuf, Self)>> {
        let projects_dir = match Self::projects_config_dir() {
            Ok(dir) => dir,
            Err(_) => return Ok(None),
        };

        if !projects_dir.exists() {
            return Ok(None);
        }

        // Strategy 1: Try to match by repository URL
        if let Some(repo_url) = git::get_remote_origin_url(start_dir) {
            let filename = generate_config_filename(&repo_url);
            let config_path = projects_dir.join(&filename);
            if config_path.exists() {
                let config = Self::load(&config_path)?;
                return Ok(Some((config_path, config)));
            }
        }

        // Strategy 2: Search all configs for matching project_path
        if let Ok(entries) = fs::read_dir(&projects_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "jsonc").unwrap_or(false) {
                    if let Ok(config) = Self::load(&path) {
                        if let Some(ref project_path) = config.project_path {
                            if start_dir.starts_with(project_path) {
                                return Ok(Some((path, config)));
                            }
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    /// Get the global config directory (~/.config/git-worktree-cli)
    pub fn global_config_dir() -> Result<PathBuf> {
        dirs::home_dir()
            .ok_or_else(|| Error::config("Could not determine home directory"))
            .map(|home| home.join(".config").join("git-worktree-cli"))
    }

    /// Get the projects config directory (~/.config/git-worktree-cli/projects)
    pub fn projects_config_dir() -> Result<PathBuf> {
        Self::global_config_dir().map(|d| d.join("projects"))
    }
}

/// Generate a safe filename from a repository URL
pub fn generate_config_filename(repo_url: &str) -> String {
    if let Some(id) = extract_repo_identifier(repo_url) {
        format!("{}.jsonc", sanitize_for_filename(&id))
    } else {
        // Fallback: hash the URL
        format!("url_{}.jsonc", short_hash(repo_url))
    }
}

fn extract_repo_identifier(url: &str) -> Option<String> {
    // GitHub SSH: git@github.com:owner/repo.git
    if let Some(rest) = url.strip_prefix("git@github.com:") {
        let cleaned = rest.trim_end_matches(".git");
        return Some(format!("github_{}", cleaned.replace('/', "_")));
    }

    // GitHub HTTPS: https://github.com/owner/repo.git
    if let Some(rest) = url.strip_prefix("https://github.com/") {
        let cleaned = rest.trim_end_matches(".git");
        return Some(format!("github_{}", cleaned.replace('/', "_")));
    }

    // Bitbucket Cloud SSH
    if let Some(rest) = url.strip_prefix("git@bitbucket.org:") {
        let cleaned = rest.trim_end_matches(".git");
        return Some(format!("bitbucket_{}", cleaned.replace('/', "_")));
    }

    // Bitbucket Cloud HTTPS
    if let Some(rest) = url.strip_prefix("https://bitbucket.org/") {
        let cleaned = rest.trim_end_matches(".git");
        return Some(format!("bitbucket_{}", cleaned.replace('/', "_")));
    }

    // Generic SSH format: git@host:path
    if url.starts_with("git@") {
        let rest = url.strip_prefix("git@").unwrap();
        if let Some((host, path)) = rest.split_once(':') {
            let host_clean = host.replace('.', "_");
            let path_clean = path.trim_end_matches(".git").replace('/', "_");
            return Some(format!("{}_{}", host_clean, path_clean));
        }
    }

    // Generic HTTPS: try to extract host and path
    if url.starts_with("https://") || url.starts_with("http://") {
        let without_protocol = url
            .strip_prefix("https://")
            .or_else(|| url.strip_prefix("http://"))
            .unwrap();
        let parts: Vec<&str> = without_protocol.splitn(2, '/').collect();
        if parts.len() == 2 {
            let host_clean = parts[0].replace('.', "_");
            let path_clean = parts[1].trim_end_matches(".git").replace('/', "_");
            return Some(format!("{}_{}", host_clean, path_clean));
        }
    }

    None
}

fn sanitize_for_filename(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>()
        .to_lowercase()
}

fn short_hash(s: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    format!("{:x}", hasher.finish())[..12].to_string()
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
            None,
            None,
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
            None,
            None,
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
            None,
            None,
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
            None,
            None,
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
    fn test_config_find_local_in_current_dir() {
        let temp_dir = tempdir().unwrap();

        // Create config in temp directory first
        let config = GitWorktreeConfig::new(
            "git@github.com:test/repo.git".to_string(),
            "main".to_string(),
            Provider::Github,
            None,
            None,
        );
        config.save(&temp_dir.path().join(CONFIG_FILENAME)).unwrap();

        // Find local config should return the config
        let result = GitWorktreeConfig::find_local_config(temp_dir.path()).unwrap();
        assert!(result.is_some());

        let (_found_path, found_config) = result.unwrap();
        assert_eq!(found_config.repository_url, "git@github.com:test/repo.git");
        assert_eq!(found_config.main_branch, "main");
    }

    #[test]
    fn test_generate_config_filename() {
        assert_eq!(
            generate_config_filename("git@github.com:owner/repo.git"),
            "github_owner_repo.jsonc"
        );
        assert_eq!(
            generate_config_filename("https://github.com/owner/repo.git"),
            "github_owner_repo.jsonc"
        );
        assert_eq!(
            generate_config_filename("git@bitbucket.org:workspace/repo.git"),
            "bitbucket_workspace_repo.jsonc"
        );
        assert_eq!(
            generate_config_filename("https://bitbucket.org/workspace/repo.git"),
            "bitbucket_workspace_repo.jsonc"
        );
    }

    #[test]
    fn test_config_local_not_found() {
        let temp_dir = tempdir().unwrap();

        // Find local config in empty temp directory should return None
        let result = GitWorktreeConfig::find_local_config(temp_dir.path()).unwrap();
        assert!(result.is_none());
    }
}
