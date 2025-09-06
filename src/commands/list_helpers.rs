use crate::{
    bitbucket_api, bitbucket_data_center_api,
    error::{Error, Result},
    github,
};

pub struct PullRequestInfo {
    pub url: String,
    pub status: String,
    pub title: String,
}

pub async fn fetch_pr_for_branch(
    platform: &str,
    owner_or_workspace: &str,
    repo: &str,
    branch: &str,
    github_client: &Option<github::GitHubClient>,
    bitbucket_client: &Option<bitbucket_api::BitbucketClient>,
    bitbucket_data_center_client: &Option<bitbucket_data_center_api::BitbucketDataCenterClient>,
) -> Result<Option<PullRequestInfo>> {
    match platform {
        "github" => fetch_github_pr(github_client, owner_or_workspace, repo, branch),
        "bitbucket-cloud" => fetch_bitbucket_cloud_pr(bitbucket_client, owner_or_workspace, repo, branch).await,
        "bitbucket-data-center" => {
            fetch_bitbucket_data_center_pr(bitbucket_data_center_client, owner_or_workspace, repo, branch).await
        }
        _ => Ok(None),
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
                        "DRAFT"
                    } else {
                        match pr.state.to_lowercase().as_str() {
                            "open" => "OPEN",
                            "closed" => "CLOSED",
                            "merged" => "MERGED",
                            _ => &pr.state.to_uppercase(),
                        }
                    };

                    Ok(Some(PullRequestInfo {
                        url: pr.html_url.clone(),
                        status: status.to_string(),
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
