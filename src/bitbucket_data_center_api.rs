use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::bitbucket_data_center_auth::BitbucketDataCenterAuth;
use crate::error::{Error, Result};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BitbucketDataCenterUser {
    pub name: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(rename = "emailAddress")]
    pub email_address: Option<String>,
    pub id: u64,
    pub slug: String,
    #[serde(rename = "type")]
    pub user_type: Option<String>,
    pub active: Option<bool>,
    pub links: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BitbucketDataCenterAuthor {
    pub user: BitbucketDataCenterUser,
    pub role: String,
    pub approved: bool,
    pub status: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BitbucketDataCenterProject {
    pub key: String,
    pub name: String,
    pub id: u64,
    pub description: Option<String>,
    #[serde(rename = "public")]
    pub is_public: Option<bool>,
    #[serde(rename = "type")]
    pub project_type: Option<String>,
    pub links: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BitbucketDataCenterRepository {
    pub slug: String,
    pub name: String,
    pub id: u64,
    pub project: BitbucketDataCenterProject,
    pub description: Option<String>,
    #[serde(rename = "hierarchyId")]
    pub hierarchy_id: Option<String>,
    #[serde(rename = "scmId")]
    pub scm_id: Option<String>,
    pub state: Option<String>,
    #[serde(rename = "statusMessage")]
    pub status_message: Option<String>,
    pub forkable: Option<bool>,
    #[serde(rename = "public")]
    pub is_public: Option<bool>,
    pub archived: Option<bool>,
    pub links: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BitbucketDataCenterBranch {
    pub id: String,
    #[serde(rename = "displayId")]
    pub display_id: String,
    pub repository: Option<BitbucketDataCenterRepository>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BitbucketDataCenterPullRequestRef {
    pub id: String,
    #[serde(rename = "displayId")]
    pub display_id: String,
    #[serde(rename = "latestCommit")]
    pub latest_commit: String,
    #[serde(rename = "type")]
    pub ref_type: String,
    pub repository: BitbucketDataCenterRepository,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BitbucketDataCenterPullRequest {
    pub id: u64,
    pub version: u32,
    pub title: String,
    pub description: Option<String>,
    pub state: String,
    pub open: bool,
    pub closed: bool,
    pub draft: Option<bool>,
    pub author: BitbucketDataCenterAuthor,
    #[serde(rename = "fromRef")]
    pub from_ref: BitbucketDataCenterPullRequestRef,
    #[serde(rename = "toRef")]
    pub to_ref: BitbucketDataCenterPullRequestRef,
    #[serde(rename = "createdDate")]
    pub created_date: u64,
    #[serde(rename = "updatedDate")]
    pub updated_date: u64,
    pub locked: Option<bool>,
    pub reviewers: Option<Vec<serde_json::Value>>,
    pub participants: Option<Vec<serde_json::Value>>,
    pub properties: Option<HashMap<String, serde_json::Value>>,
    pub links: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct BitbucketDataCenterPullRequestsResponse {
    pub values: Vec<BitbucketDataCenterPullRequest>,
    #[allow(dead_code)]
    pub size: u32,
    #[allow(dead_code)]
    pub limit: u32,
    #[serde(rename = "isLastPage")]
    #[allow(dead_code)]
    pub is_last_page: bool,
    #[allow(dead_code)]
    pub start: u32,
}

pub struct BitbucketDataCenterClient {
    client: Client,
    auth: BitbucketDataCenterAuth,
    base_url: String,
}

impl BitbucketDataCenterClient {
    pub fn new(auth: BitbucketDataCenterAuth, base_url: String) -> Self {
        let client = Client::new();
        BitbucketDataCenterClient { client, auth, base_url }
    }

    pub async fn get_pull_requests(
        &self,
        project_key: &str,
        repo_slug: &str,
    ) -> Result<Vec<BitbucketDataCenterPullRequest>> {
        let token = self.auth.get_token()?;
        let url = format!(
            "{}/rest/api/1.0/projects/{}/repos/{}/pull-requests",
            self.base_url.trim_end_matches('/'),
            project_key,
            repo_slug
        );

        let response = self
            .client
            .get(&url)
            .bearer_auth(&token)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| Error::network(format!("Failed to send request to Bitbucket Data Center API: {}", e)))?;

        if response.status().is_client_error() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();

            if status == 401 {
                return Err(Error::auth(
                    "Authentication failed. Please check your Bitbucket Data Center access token and run 'gwt auth bitbucket-data-center' to update it."
                ));
            } else if status == 404 {
                return Err(Error::provider(format!(
                    "Repository not found: {}/{}. Please check the project key and repository slug.",
                    project_key, repo_slug
                )));
            } else {
                return Err(Error::provider(format!(
                    "API request failed with status {}: {}",
                    status, text
                )));
            }
        }

        let pr_response: BitbucketDataCenterPullRequestsResponse = response
            .json()
            .await
            .map_err(|e| Error::provider(format!("Failed to parse Bitbucket Data Center API response: {}", e)))?;

        Ok(pr_response.values)
    }

    pub async fn test_connection(&self) -> Result<()> {
        let token = self.auth.get_token()?;
        let url = format!("{}/rest/api/1.0/users", self.base_url.trim_end_matches('/'));

        let response = self
            .client
            .get(&url)
            .bearer_auth(&token)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| Error::network(format!("Failed to test Bitbucket Data Center API connection: {}", e)))?;

        if response.status().is_success() {
            println!("✓ Bitbucket Data Center API connection successful");
            Ok(())
        } else {
            let status = response.status();
            if status == 401 {
                Err(Error::auth(
                    "Authentication failed. Please check your Bitbucket Data Center access token.",
                ))
            } else {
                Err(Error::provider(format!(
                    "API connection failed with status: {}",
                    status
                )))
            }
        }
    }
}

pub fn extract_bitbucket_data_center_info_from_url(url: &str) -> Option<(String, String, String)> {
    // Parse URLs like:
    // https://git.acmeorg.com/scm/PROJECT/repository.git
    // https://git.acmeorg.com/projects/PROJECT/repos/repository
    // git@git.acmeorg.com:PROJECT/repository.git

    // Pattern for Data Center URLs with /scm/ path
    if let Some(captures) = regex::Regex::new(r"([^/]+)/scm/([^/]+)/([^/\.]+)").ok()?.captures(url) {
        let base_url = captures.get(1)?.as_str();
        let project = captures.get(2)?.as_str();
        let repo = captures.get(3)?.as_str();

        // Reconstruct the base URL for API calls
        let api_base_url = if base_url.starts_with("http") {
            base_url.to_string()
        } else {
            format!("https://{}", base_url)
        };

        return Some((api_base_url, project.to_string(), repo.to_string()));
    }

    // Pattern for Data Center URLs with /projects/ path
    if let Some(captures) = regex::Regex::new(r"([^/]+)/projects/([^/]+)/repos/([^/\.]+)")
        .ok()?
        .captures(url)
    {
        let base_url = captures.get(1)?.as_str();
        let project = captures.get(2)?.as_str();
        let repo = captures.get(3)?.as_str();

        let api_base_url = if base_url.starts_with("http") {
            base_url.to_string()
        } else {
            format!("https://{}", base_url)
        };

        return Some((api_base_url, project.to_string(), repo.to_string()));
    }

    // Pattern for SSH URLs: git@host:project/repo.git
    if let Some(captures) = regex::Regex::new(r"git@([^:]+):([^/]+)/([^/\.]+)").ok()?.captures(url) {
        let host = captures.get(1)?.as_str();
        let project = captures.get(2)?.as_str();
        let repo = captures.get(3)?.as_str();

        return Some((format!("https://{}", host), project.to_string(), repo.to_string()));
    }

    // Pattern for SSH URLs with protocol: ssh://git@host/project/repo.git
    if let Some(captures) = regex::Regex::new(r"ssh://git@([^/]+)/([^/]+)/([^/\.]+)")
        .ok()?
        .captures(url)
    {
        let host = captures.get(1)?.as_str();
        let project = captures.get(2)?.as_str();
        let repo = captures.get(3)?.as_str();

        return Some((format!("https://{}", host), project.to_string(), repo.to_string()));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_bitbucket_data_center_info_scm() {
        let url = "https://git.acmeorg.com/scm/PROJ/repo";
        let result = extract_bitbucket_data_center_info_from_url(url);
        assert_eq!(
            result,
            Some((
                "https://git.acmeorg.com".to_string(),
                "PROJ".to_string(),
                "repo".to_string()
            ))
        );
    }

    #[test]
    fn test_extract_bitbucket_data_center_info_scm_git() {
        let url = "https://git.acmeorg.com/scm/PROJ/repo.git";
        let result = extract_bitbucket_data_center_info_from_url(url);
        assert_eq!(
            result,
            Some((
                "https://git.acmeorg.com".to_string(),
                "PROJ".to_string(),
                "repo".to_string()
            ))
        );
    }

    #[test]
    fn test_extract_bitbucket_data_center_info_projects() {
        let url = "https://git.acmeorg.com/projects/PROJ/repos/repo";
        let result = extract_bitbucket_data_center_info_from_url(url);
        assert_eq!(
            result,
            Some((
                "https://git.acmeorg.com".to_string(),
                "PROJ".to_string(),
                "repo".to_string()
            ))
        );
    }

    #[test]
    fn test_extract_bitbucket_data_center_info_ssh() {
        let url = "git@git.acmeorg.com:PROJ/repo.git";
        let result = extract_bitbucket_data_center_info_from_url(url);
        assert_eq!(
            result,
            Some((
                "https://git.acmeorg.com".to_string(),
                "PROJ".to_string(),
                "repo".to_string()
            ))
        );
    }

    #[test]
    fn test_extract_bitbucket_data_center_info_ssh_protocol() {
        let url = "ssh://git@git.acmeorg.com/PROJECT_ID/REPO_ID.git";
        let result = extract_bitbucket_data_center_info_from_url(url);
        assert_eq!(
            result,
            Some((
                "https://git.acmeorg.com".to_string(),
                "PROJECT_ID".to_string(),
                "REPO_ID".to_string()
            ))
        );
    }

    #[test]
    fn test_extract_bitbucket_data_center_info_invalid() {
        let url = "https://github.com/user/repo";
        let result = extract_bitbucket_data_center_info_from_url(url);
        assert_eq!(result, None);
    }
}
