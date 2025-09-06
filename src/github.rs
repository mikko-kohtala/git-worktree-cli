use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

#[derive(Debug, Serialize, Deserialize)]
pub struct PullRequest {
    pub number: u32,
    pub title: String,
    pub state: String,
    pub html_url: String,
    pub draft: bool,
}

pub struct GitHubClient;

impl Default for GitHubClient {
    fn default() -> Self {
        Self::new()
    }
}

impl GitHubClient {
    pub fn new() -> Self {
        Self
    }

    fn get_gh_token() -> Option<String> {
        std::process::Command::new("gh")
            .args(["auth", "token"])
            .output()
            .ok()
            .and_then(|output| {
                if output.status.success() {
                    String::from_utf8(output.stdout)
                        .ok()
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                } else {
                    None
                }
            })
    }

    pub fn has_auth(&self) -> bool {
        Self::get_gh_token().is_some()
    }

    pub fn get_pull_requests(&self, owner: &str, repo: &str, branch: &str) -> Result<Vec<PullRequest>> {
        // Use gh CLI instead of HTTP API
        let output = std::process::Command::new("gh")
            .args([
                "pr",
                "list",
                "--repo",
                &format!("{}/{}", owner, repo),
                "--head",
                branch,
                "--state",
                "all",
                "--json",
                "number,title,state,url,isDraft",
            ])
            .output()
            .map_err(|e| Error::provider(format!("Failed to execute gh command: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("not authenticated") || stderr.contains("authentication") {
                return Err(Error::auth(
                    "GitHub authentication failed. Run 'gh auth login' to authenticate.",
                ));
            }
            return Err(Error::provider(format!("Failed to fetch pull requests: {}", stderr)));
        }

        let stdout = String::from_utf8(output.stdout)?;
        if stdout.trim().is_empty() {
            return Ok(vec![]);
        }

        let prs: Vec<serde_json::Value> = serde_json::from_str(&stdout)
            .map_err(|e| Error::provider(format!("Failed to parse pull requests from gh output: {}", e)))?;

        Ok(prs
            .into_iter()
            .map(|pr| PullRequest {
                number: pr["number"].as_u64().unwrap_or(0) as u32,
                title: pr["title"].as_str().unwrap_or("").to_string(),
                state: pr["state"].as_str().unwrap_or("").to_string(),
                html_url: pr["url"].as_str().unwrap_or("").to_string(), // Changed from html_url to url
                draft: pr["isDraft"].as_bool().unwrap_or(false),        // Changed from draft to isDraft
            })
            .collect())
    }

    pub fn get_all_pull_requests(&self, owner: &str, repo: &str) -> Result<Vec<(PullRequest, String)>> {
        // Fetch all open pull requests with branch information
        let output = std::process::Command::new("gh")
            .args([
                "pr",
                "list",
                "--repo",
                &format!("{}/{}", owner, repo),
                "--state",
                "open",
                "--json",
                "number,title,state,url,isDraft,headRefName",
                "--limit",
                "100",
            ])
            .output()
            .map_err(|e| Error::provider(format!("Failed to execute gh command: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("not authenticated") || stderr.contains("authentication") {
                return Err(Error::auth(
                    "GitHub authentication failed. Run 'gh auth login' to authenticate.",
                ));
            }
            return Err(Error::provider(format!("Failed to fetch pull requests: {}", stderr)));
        }

        let stdout = String::from_utf8(output.stdout)?;
        if stdout.trim().is_empty() {
            return Ok(vec![]);
        }

        let prs: Vec<serde_json::Value> = serde_json::from_str(&stdout)
            .map_err(|e| Error::provider(format!("Failed to parse pull requests from gh output: {}", e)))?;

        Ok(prs
            .into_iter()
            .map(|pr| {
                let pull_request = PullRequest {
                    number: pr["number"].as_u64().unwrap_or(0) as u32,
                    title: pr["title"].as_str().unwrap_or("").to_string(),
                    state: pr["state"].as_str().unwrap_or("").to_string(),
                    html_url: pr["url"].as_str().unwrap_or("").to_string(),
                    draft: pr["isDraft"].as_bool().unwrap_or(false),
                };
                let branch = pr["headRefName"].as_str().unwrap_or("").to_string();
                (pull_request, branch)
            })
            .collect())
    }

    pub fn parse_github_url(url: &str) -> Option<(String, String)> {
        // Parse both HTTPS and SSH URLs
        if let Some(captures) = url.strip_prefix("https://github.com/") {
            let parts: Vec<&str> = captures.trim_end_matches(".git").split('/').collect();
            if parts.len() >= 2 {
                return Some((parts[0].to_string(), parts[1].to_string()));
            }
        } else if let Some(captures) = url.strip_prefix("git@github.com:") {
            let parts: Vec<&str> = captures.trim_end_matches(".git").split('/').collect();
            if parts.len() >= 2 {
                return Some((parts[0].to_string(), parts[1].to_string()));
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_github_url() {
        let test_cases = vec![
            (
                "https://github.com/owner/repo.git",
                Some(("owner".to_string(), "repo".to_string())),
            ),
            (
                "https://github.com/owner/repo",
                Some(("owner".to_string(), "repo".to_string())),
            ),
            (
                "git@github.com:owner/repo.git",
                Some(("owner".to_string(), "repo".to_string())),
            ),
            (
                "git@github.com:owner/repo",
                Some(("owner".to_string(), "repo".to_string())),
            ),
            ("https://gitlab.com/owner/repo", None),
        ];

        for (url, expected) in test_cases {
            assert_eq!(GitHubClient::parse_github_url(url), expected);
        }
    }
}
