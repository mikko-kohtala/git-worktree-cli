use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::bitbucket_auth::BitbucketAuth;
use crate::error::{Error, Result};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BitbucketUser {
    pub display_name: String,
    pub uuid: String,
    pub nickname: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BitbucketRepository {
    pub name: String,
    pub full_name: String,
    pub uuid: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BitbucketBranch {
    pub name: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BitbucketSource {
    pub branch: BitbucketBranch,
    pub repository: BitbucketRepository,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BitbucketDestination {
    pub branch: BitbucketBranch,
    pub repository: BitbucketRepository,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BitbucketPullRequest {
    pub id: u64,
    pub title: String,
    pub state: String,
    pub author: BitbucketUser,
    pub source: BitbucketSource,
    pub destination: BitbucketDestination,
    pub created_on: String,
    pub updated_on: String,
    pub links: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct BitbucketPullRequestsResponse {
    pub values: Vec<BitbucketPullRequest>,
}

pub struct BitbucketClient {
    client: Client,
    auth: BitbucketAuth,
}

impl BitbucketClient {
    pub fn new(auth: BitbucketAuth) -> Self {
        let client = Client::new();
        BitbucketClient { client, auth }
    }

    fn get_email(&self) -> String {
        // Use email from auth if available, otherwise use a placeholder
        self.auth.email().unwrap_or_else(|| "user".to_string())
    }

    pub async fn get_pull_requests(&self, workspace: &str, repo_slug: &str) -> Result<Vec<BitbucketPullRequest>> {
        let token = self.auth.get_token()?;
        let url = format!(
            "https://api.bitbucket.org/2.0/repositories/{}/{}/pullrequests",
            workspace, repo_slug
        );

        let response = self
            .client
            .get(&url)
            .basic_auth(self.get_email(), Some(&token))
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| Error::network(format!("Failed to send request to Bitbucket API: {}", e)))?;

        if response.status().is_client_error() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();

            if status == 401 {
                return Err(Error::auth(
                    "Authentication failed. Please check your Bitbucket credentials and run 'gwt auth bitbucket' to update them."
                ));
            } else if status == 404 {
                return Err(Error::provider(format!(
                    "Repository not found: {}/{}. Please check the workspace and repository name.",
                    workspace,
                    repo_slug
                )));
            } else {
                return Err(Error::provider(format!("API request failed with status {}: {}", status, text)));
            }
        }

        let pr_response: BitbucketPullRequestsResponse = response
            .json()
            .await
            .map_err(|e| Error::provider(format!("Failed to parse Bitbucket API response: {}", e)))?;

        Ok(pr_response.values)
    }

    pub async fn test_connection(&self) -> Result<()> {
        let token = self.auth.get_token()?;
        let url = "https://api.bitbucket.org/2.0/user";

        let response = self
            .client
            .get(url)
            .basic_auth(self.get_email(), Some(&token))
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| Error::network(format!("Failed to test Bitbucket API connection: {}", e)))?;

        if response.status().is_success() {
            println!("✓ Bitbucket API connection successful");
            Ok(())
        } else {
            let status = response.status();
            if status == 401 {
                Err(Error::auth(
                    "Authentication failed. Please check your Bitbucket credentials."
                ))
            } else {
                Err(Error::provider(format!("API connection failed with status: {}", status)))
            }
        }
    }
}

pub fn extract_bitbucket_info_from_url(url: &str) -> Option<(String, String)> {
    // Parse URLs like:
    // https://bitbucket.org/workspace/repo
    // git@bitbucket.org:workspace/repo.git
    // https://bitbucket.org/workspace/repo.git

    if url.contains("bitbucket.org") {
        if let Some(captures) = regex::Regex::new(r"bitbucket\.org[:/]([^/]+)/([^/\.]+)")
            .ok()?
            .captures(url)
        {
            let workspace = captures.get(1)?.as_str();
            let repo = captures.get(2)?.as_str();
            return Some((workspace.to_string(), repo.to_string()));
        }
    }

    None
}

pub fn is_bitbucket_repository(remote_url: &str) -> bool {
    remote_url.contains("bitbucket.org")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_bitbucket_info_https() {
        let url = "https://bitbucket.org/myworkspace/myrepo";
        let result = extract_bitbucket_info_from_url(url);
        assert_eq!(result, Some(("myworkspace".to_string(), "myrepo".to_string())));
    }

    #[test]
    fn test_extract_bitbucket_info_https_git() {
        let url = "https://bitbucket.org/myworkspace/myrepo.git";
        let result = extract_bitbucket_info_from_url(url);
        assert_eq!(result, Some(("myworkspace".to_string(), "myrepo".to_string())));
    }

    #[test]
    fn test_extract_bitbucket_info_ssh() {
        let url = "git@bitbucket.org:myworkspace/myrepo.git";
        let result = extract_bitbucket_info_from_url(url);
        assert_eq!(result, Some(("myworkspace".to_string(), "myrepo".to_string())));
    }

    #[test]
    fn test_extract_bitbucket_info_invalid() {
        let url = "https://github.com/user/repo";
        let result = extract_bitbucket_info_from_url(url);
        assert_eq!(result, None);
    }

    #[test]
    fn test_is_bitbucket_repository() {
        assert!(is_bitbucket_repository("https://bitbucket.org/workspace/repo"));
        assert!(is_bitbucket_repository("git@bitbucket.org:workspace/repo.git"));
        assert!(!is_bitbucket_repository("https://github.com/user/repo"));
    }
}
