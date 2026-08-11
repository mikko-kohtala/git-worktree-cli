use serde::{Deserialize, Serialize};

use crate::constants::GITHUB_HOST;
use crate::error::{Error, Result};

#[derive(Debug, Serialize, Deserialize)]
pub struct PullRequest {
    pub number: u32,
    pub title: String,
    pub state: String,
    pub html_url: String,
    pub draft: bool,
}

// Structs for gh CLI JSON output
#[derive(Debug, Deserialize)]
struct GhPrResponse {
    number: u32,
    title: String,
    state: String,
    url: String,
    #[serde(rename = "isDraft")]
    is_draft: bool,
}

#[derive(Debug, Deserialize)]
struct GhPrWithBranchResponse {
    number: u32,
    title: String,
    state: String,
    url: String,
    #[serde(rename = "isDraft")]
    is_draft: bool,
    #[serde(rename = "headRefName")]
    head_ref_name: String,
}

pub struct GitHubClient {
    host: String,
}

impl Default for GitHubClient {
    fn default() -> Self {
        Self::new()
    }
}

impl GitHubClient {
    pub fn new() -> Self {
        Self::for_host(GITHUB_HOST.to_string())
    }

    pub fn for_host(host: String) -> Self {
        Self { host }
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    fn get_gh_token(&self) -> Option<String> {
        std::process::Command::new("gh")
            .args(["auth", "token", "--hostname", &self.host])
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
        self.get_gh_token().is_some()
    }

    pub fn get_pull_requests(&self, owner: &str, repo: &str, branch: &str) -> Result<Vec<PullRequest>> {
        // Use gh CLI instead of HTTP API
        let output = std::process::Command::new("gh")
            .args([
                "pr",
                "list",
                "--repo",
                &format!("{}/{}/{}", self.host, owner, repo),
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

        let prs: Vec<GhPrResponse> = serde_json::from_str(&stdout)
            .map_err(|e| Error::provider(format!("Failed to parse pull requests from gh output: {}", e)))?;

        Ok(prs
            .into_iter()
            .map(|pr| PullRequest {
                number: pr.number,
                title: pr.title,
                state: pr.state,
                html_url: pr.url,
                draft: pr.is_draft,
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
                &format!("{}/{}/{}", self.host, owner, repo),
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

        let prs: Vec<GhPrWithBranchResponse> = serde_json::from_str(&stdout)
            .map_err(|e| Error::provider(format!("Failed to parse pull requests from gh output: {}", e)))?;

        Ok(prs
            .into_iter()
            .map(|pr| {
                let pull_request = PullRequest {
                    number: pr.number,
                    title: pr.title,
                    state: pr.state,
                    html_url: pr.url,
                    draft: pr.is_draft,
                };
                (pull_request, pr.head_ref_name)
            })
            .collect())
    }

    /// Hosts recognized as GitHub: github.com and GitHub Enterprise Cloud
    /// data-residency tenants (<tenant>.ghe.com)
    fn is_github_host(host: &str) -> bool {
        host == GITHUB_HOST || host.ends_with(".ghe.com")
    }

    fn split_owner_repo(path: &str) -> Option<(String, String)> {
        let parts: Vec<&str> = path.trim_end_matches(".git").split('/').collect();
        if parts.len() >= 2 && !parts[0].is_empty() && !parts[1].is_empty() {
            Some((parts[0].to_string(), parts[1].to_string()))
        } else {
            None
        }
    }

    pub fn parse_github_url(url: &str) -> Option<(String, String)> {
        Self::parse_github_url_with_host(url).map(|(_, owner, repo)| (owner, repo))
    }

    pub fn parse_github_url_with_host(url: &str) -> Option<(String, String, String)> {
        // https://host/owner/repo(.git) and ssh://git@host/owner/repo(.git)
        for scheme in ["https://", "http://", "ssh://"] {
            if let Some(rest) = url.strip_prefix(scheme) {
                let rest = rest.split_once('@').map_or(rest, |(_, r)| r);
                let (host, path) = rest.split_once('/')?;
                if !Self::is_github_host(host) {
                    return None;
                }
                let (owner, repo) = Self::split_owner_repo(path)?;
                return Some((host.to_string(), owner, repo));
            }
        }

        // SCP-style SSH: user@host:owner/repo(.git). github.com uses git@,
        // GHE data-residency remotes use <tenant>@<tenant>.ghe.com
        let (user_host, path) = url.split_once(':')?;
        let (_, host) = user_host.split_once('@')?;
        if !Self::is_github_host(host) {
            return None;
        }
        let (owner, repo) = Self::split_owner_repo(path)?;
        Some((host.to_string(), owner, repo))
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
            (
                "acme@acme.ghe.com:owner/repo.git",
                Some(("owner".to_string(), "repo".to_string())),
            ),
            (
                "https://acme.ghe.com/owner/repo.git",
                Some(("owner".to_string(), "repo".to_string())),
            ),
            ("https://gitlab.com/owner/repo", None),
            ("git@gitlab.com:owner/repo.git", None),
        ];

        for (url, expected) in test_cases {
            assert_eq!(GitHubClient::parse_github_url(url), expected);
        }
    }

    #[test]
    fn test_parse_github_url_with_host() {
        let test_cases = vec![
            (
                "git@github.com:owner/repo.git",
                Some(("github.com".to_string(), "owner".to_string(), "repo".to_string())),
            ),
            (
                "acme@acme.ghe.com:owner/repo.git",
                Some(("acme.ghe.com".to_string(), "owner".to_string(), "repo".to_string())),
            ),
            (
                "https://acme.ghe.com/owner/repo",
                Some(("acme.ghe.com".to_string(), "owner".to_string(), "repo".to_string())),
            ),
            (
                "ssh://git@acme.ghe.com/owner/repo.git",
                Some(("acme.ghe.com".to_string(), "owner".to_string(), "repo".to_string())),
            ),
            // .ghe.com must be a subdomain suffix, not part of another domain
            ("git@evil-ghe.com:owner/repo.git", None),
            ("https://gitlab.com/owner/repo", None),
        ];

        for (url, expected) in test_cases {
            assert_eq!(GitHubClient::parse_github_url_with_host(url), expected);
        }
    }
}
