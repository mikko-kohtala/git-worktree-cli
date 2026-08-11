use colored::{ColoredString, Colorize};

use crate::{
    azure_devops, bitbucket_api, bitbucket_auth, bitbucket_data_center_api, bitbucket_data_center_auth, config,
    error::{Error, Result},
    github,
};

pub struct PullRequestInfo {
    pub url: String,
    pub status: String,
    pub title: String,
}

/// Color a PR status string the same way across commands
pub fn colored_pr_status(status: &str) -> ColoredString {
    match status {
        "OPEN" => "open".green(),
        "CLOSED" => "closed".red(),
        "MERGED" => "merged".green(),
        "DRAFT" => "draft".yellow(),
        _ => status.normal(),
    }
}

/// Detected source-control provider clients and repository info,
/// used to fetch pull request status for branches.
pub struct PrContext {
    pub github_client: Option<github::GitHubClient>,
    pub bitbucket_client: Option<bitbucket_api::BitbucketClient>,
    pub bitbucket_data_center_client: Option<bitbucket_data_center_api::BitbucketDataCenterClient>,
    pub azure_devops_client: Option<azure_devops::AzureDevOpsClient>,
    /// (platform, owner/workspace/project, repo)
    pub repo_info: Option<(String, String, String)>,
}

impl PrContext {
    /// Detect the provider from the project config and set up authenticated clients
    pub fn detect() -> Result<Self> {
        let mut github_client = github::GitHubClient::new();
        let mut bitbucket_client: Option<bitbucket_api::BitbucketClient> = None;
        let mut bitbucket_data_center_client: Option<bitbucket_data_center_api::BitbucketDataCenterClient> = None;
        let mut azure_devops_client: Option<azure_devops::AzureDevOpsClient> = None;

        let repo_info = if let Some((_, config)) = config::GitWorktreeConfig::find_config()? {
            let repo_url = &config.repository_url;

            // Use the configured sourceControl instead of URL pattern matching
            match config.source_control.as_str() {
                "bitbucket-cloud" => {
                    if let Some((workspace, repo)) = bitbucket_api::extract_bitbucket_info_from_url(repo_url) {
                        // Try to get Bitbucket Cloud auth
                        if let Ok(auth) = bitbucket_auth::BitbucketAuth::new(
                            workspace.clone(),
                            repo.clone(),
                            config.bitbucket_email.clone(),
                        ) {
                            if auth.has_stored_token() {
                                bitbucket_client = Some(bitbucket_api::BitbucketClient::new(auth));
                            }
                        }
                        Some(("bitbucket-cloud".to_string(), workspace, repo))
                    } else {
                        None
                    }
                }
                "bitbucket-data-center" => {
                    // Always use get_auth_from_config for bitbucket-data-center since it can derive the API URL
                    if let Ok((base_url, project_key, repo_slug)) = bitbucket_data_center_auth::get_auth_from_config() {
                        if let Ok(auth) = bitbucket_data_center_auth::BitbucketDataCenterAuth::new(
                            project_key.clone(),
                            repo_slug.clone(),
                            base_url.clone(),
                        ) {
                            if auth.get_token().is_ok() {
                                bitbucket_data_center_client = Some(
                                    bitbucket_data_center_api::BitbucketDataCenterClient::new(auth, base_url),
                                );
                            }
                        }
                        Some(("bitbucket-data-center".to_string(), project_key, repo_slug))
                    } else {
                        // Could not get auth config - extract repo info for display but no client
                        let (owner, repo) = github::GitHubClient::parse_github_url(repo_url)
                            .unwrap_or_else(|| ("".to_string(), "".to_string()));
                        if !owner.is_empty() && !repo.is_empty() {
                            Some(("bitbucket-data-center".to_string(), owner, repo))
                        } else {
                            None
                        }
                    }
                }
                "azure-devops" => {
                    if let Some((organization, project, repo)) =
                        azure_devops::AzureDevOpsClient::parse_azure_url(repo_url)
                    {
                        azure_devops_client = Some(azure_devops::AzureDevOpsClient::new(organization.clone(), project));
                        Some(("azure-devops".to_string(), organization, repo))
                    } else {
                        None
                    }
                }
                _ => {
                    // Try GitHub (github.com or GitHub Enterprise *.ghe.com)
                    if let Some((host, owner, repo)) = github::GitHubClient::parse_github_url_with_host(repo_url) {
                        github_client = github::GitHubClient::for_host(host);
                        Some(("github".to_string(), owner, repo))
                    } else {
                        None
                    }
                }
            }
        } else {
            None
        };

        Ok(PrContext {
            github_client: Some(github_client),
            bitbucket_client,
            bitbucket_data_center_client,
            azure_devops_client,
            repo_info,
        })
    }

    /// Whether PR information can be fetched (provider detected and authenticated)
    pub fn has_pr_info(&self) -> bool {
        match &self.repo_info {
            Some((platform, _, _)) => match platform.as_str() {
                "github" => self.github_client.as_ref().map(|c| c.has_auth()).unwrap_or(false),
                "bitbucket-cloud" => self.bitbucket_client.is_some(),
                "bitbucket-data-center" => self.bitbucket_data_center_client.is_some(),
                "azure-devops" => self.azure_devops_client.as_ref().map(|c| c.has_auth()).unwrap_or(false),
                _ => false,
            },
            None => false,
        }
    }

    /// Fetch PR info for a branch, returning None on any failure
    pub async fn fetch_pr(&self, branch: &str) -> Option<PullRequestInfo> {
        if !self.has_pr_info() {
            return None;
        }
        let (platform, owner_or_workspace, repo) = self.repo_info.as_ref()?;
        fetch_pr_for_branch(
            platform,
            owner_or_workspace,
            repo,
            branch,
            &self.github_client,
            &self.bitbucket_client,
            &self.bitbucket_data_center_client,
            &self.azure_devops_client,
        )
        .await
        .unwrap_or_default()
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn fetch_pr_for_branch(
    platform: &str,
    owner_or_workspace: &str,
    repo: &str,
    branch: &str,
    github_client: &Option<github::GitHubClient>,
    bitbucket_client: &Option<bitbucket_api::BitbucketClient>,
    bitbucket_data_center_client: &Option<bitbucket_data_center_api::BitbucketDataCenterClient>,
    azure_devops_client: &Option<azure_devops::AzureDevOpsClient>,
) -> Result<Option<PullRequestInfo>> {
    match platform {
        "github" => fetch_github_pr(github_client, owner_or_workspace, repo, branch),
        "bitbucket-cloud" => fetch_bitbucket_cloud_pr(bitbucket_client, owner_or_workspace, repo, branch).await,
        "bitbucket-data-center" => {
            fetch_bitbucket_data_center_pr(bitbucket_data_center_client, owner_or_workspace, repo, branch).await
        }
        "azure-devops" => fetch_azure_devops_pr(azure_devops_client, repo, branch),
        _ => Ok(None),
    }
}

/// Map an Azure DevOps PR to the shared display status vocabulary
fn azure_devops_display_status(pr: &azure_devops::PullRequest) -> String {
    if pr.draft {
        return "DRAFT".to_string();
    }
    match pr.status.as_str() {
        "active" => "OPEN".to_string(),
        "completed" => "MERGED".to_string(),
        "abandoned" => "CLOSED".to_string(),
        other => other.to_uppercase(),
    }
}

fn fetch_azure_devops_pr(
    client: &Option<azure_devops::AzureDevOpsClient>,
    repo: &str,
    branch: &str,
) -> Result<Option<PullRequestInfo>> {
    if let Some(ref client) = client {
        match client.get_pull_requests(repo, branch) {
            Ok(prs) => {
                if let Some(pr) = prs.first() {
                    Ok(Some(PullRequestInfo {
                        url: pr.html_url.clone(),
                        status: azure_devops_display_status(pr),
                        title: pr.title.clone(),
                    }))
                } else {
                    Ok(None)
                }
            }
            Err(_) => Err(Error::provider("Failed to fetch Azure DevOps PRs")),
        }
    } else {
        Ok(None)
    }
}

fn fetch_github_pr(
    client: &Option<github::GitHubClient>,
    owner: &str,
    repo: &str,
    branch: &str,
) -> Result<Option<PullRequestInfo>> {
    if let Some(ref client) = client {
        match client.get_pull_requests(owner, repo, branch) {
            Ok(prs) => {
                if let Some(pr) = prs.first() {
                    let status = if pr.draft {
                        "DRAFT".to_string()
                    } else {
                        match pr.state.to_lowercase().as_str() {
                            "open" => "OPEN".to_string(),
                            "closed" => "CLOSED".to_string(),
                            "merged" => "MERGED".to_string(),
                            _ => pr.state.to_uppercase(),
                        }
                    };

                    Ok(Some(PullRequestInfo {
                        url: pr.html_url.clone(),
                        status,
                        title: pr.title.clone(),
                    }))
                } else {
                    Ok(None)
                }
            }
            Err(_) => Err(Error::provider("Failed to fetch GitHub PRs")),
        }
    } else {
        Ok(None)
    }
}

async fn fetch_bitbucket_cloud_pr(
    client: &Option<bitbucket_api::BitbucketClient>,
    workspace: &str,
    repo: &str,
    branch: &str,
) -> Result<Option<PullRequestInfo>> {
    if let Some(ref client) = client {
        match client.get_pull_requests(workspace, repo).await {
            Ok(prs) => {
                if let Some(pr) = prs.iter().find(|pr| pr.source.branch.name == branch) {
                    let url = extract_bitbucket_cloud_url(pr);
                    Ok(Some(PullRequestInfo {
                        url,
                        status: pr.state.to_uppercase(),
                        title: pr.title.clone(),
                    }))
                } else {
                    Ok(None)
                }
            }
            Err(_) => Err(Error::provider("Failed to fetch Bitbucket Cloud PRs")),
        }
    } else {
        Ok(None)
    }
}

async fn fetch_bitbucket_data_center_pr(
    client: &Option<bitbucket_data_center_api::BitbucketDataCenterClient>,
    project: &str,
    repo: &str,
    branch: &str,
) -> Result<Option<PullRequestInfo>> {
    if let Some(ref client) = client {
        match client.get_pull_requests(project, repo).await {
            Ok(prs) => {
                if let Some(pr) = prs.iter().find(|pr| pr.from_ref.display_id == branch) {
                    let url = extract_bitbucket_data_center_url(pr);
                    Ok(Some(PullRequestInfo {
                        url,
                        status: pr.state.to_uppercase(),
                        title: pr.title.clone(),
                    }))
                } else {
                    Ok(None)
                }
            }
            Err(_) => Err(Error::provider("Failed to fetch Bitbucket Data Center PRs")),
        }
    } else {
        Ok(None)
    }
}

pub fn extract_bitbucket_cloud_url(pr: &bitbucket_api::BitbucketPullRequest) -> String {
    if let Some(html_link) = pr.links.get("html") {
        if let Some(href) = html_link.get("href") {
            if let Some(url) = href.as_str() {
                return url.to_string();
            }
        }
    }
    format!("PR #{}", pr.id)
}

pub fn extract_bitbucket_data_center_url(pr: &bitbucket_data_center_api::BitbucketDataCenterPullRequest) -> String {
    if let Some(self_link) = pr.links.get("self") {
        if let Some(links_array) = self_link.as_array() {
            if let Some(first_link) = links_array.first() {
                if let Some(href) = first_link.get("href") {
                    if let Some(url) = href.as_str() {
                        return url.to_string();
                    }
                }
            }
        }
    }
    format!("PR #{}", pr.id)
}
