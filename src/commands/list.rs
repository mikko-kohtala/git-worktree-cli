use anyhow::Result;
use colored::Colorize;

use super::list_helpers::{
    extract_bitbucket_cloud_url, extract_bitbucket_data_center_url, fetch_pr_for_branch, PullRequestInfo,
};
use crate::{
    bitbucket_api, bitbucket_auth, bitbucket_data_center_api, bitbucket_data_center_auth, config,
    core::project::{clean_branch_name, find_git_directory},
    git, github,
};

struct WorktreeDisplay {
    branch: String,
    pr_info: Option<PullRequestInfo>,
}

struct RemotePullRequest {
    branch: String,
    pr_info: PullRequestInfo,
}

#[tokio::main]
pub async fn run(local_only: bool) -> Result<()> {
    // Find a git directory to work with
    let git_dir = find_git_directory()?;

    // Get the list of worktrees
    let worktrees = git::list_worktrees(Some(&git_dir))?;

    if worktrees.is_empty() {
        println!("{}", "No worktrees found.".yellow());
        return Ok(());
    }

    // Try to get GitHub/Bitbucket info automatically
    let (github_client, bitbucket_client, bitbucket_data_center_client, repo_info) = {
        let github_client = github::GitHubClient::new();
        let mut bitbucket_client: Option<bitbucket_api::BitbucketClient> = None;
        let mut bitbucket_data_center_client: Option<bitbucket_data_center_api::BitbucketDataCenterClient> = None;

        if let Some((_, config)) = config::GitWorktreeConfig::find_config()? {
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
                        (
                            Some(github_client),
                            bitbucket_client,
                            None,
                            Some(("bitbucket-cloud".to_string(), workspace, repo)),
                        )
                    } else {
                        (Some(github_client), None, None, None)
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
                        (
                            Some(github_client),
                            None,
                            bitbucket_data_center_client,
                            Some(("bitbucket-data-center".to_string(), project_key, repo_slug)),
                        )
                    } else {
                        // Could not get auth config - extract repo info for display but no client
                        let (owner, repo) = github::GitHubClient::parse_github_url(repo_url)
                            .unwrap_or_else(|| ("".to_string(), "".to_string()));
                        if !owner.is_empty() && !repo.is_empty() {
                            (
                                Some(github_client),
                                None,
                                None,
                                Some(("bitbucket-data-center".to_string(), owner, repo)),
                            )
                        } else {
                            (Some(github_client), None, None, None)
                        }
                    }
                }
                _ => {
                    // Try GitHub
                    let (owner, repo) = github::GitHubClient::parse_github_url(repo_url)
                        .unwrap_or_else(|| ("".to_string(), "".to_string()));

                    if !owner.is_empty() && !repo.is_empty() {
                        (
                            Some(github_client),
                            None,
                            None,
                            Some(("github".to_string(), owner, repo)),
                        )
                    } else {
                        (Some(github_client), None, None, None)
                    }
                }
            }
        } else {
            (Some(github_client), None, None, None)
        }
    };

    let has_pr_info = repo_info.is_some()
        && match &repo_info {
            Some((platform, _, _)) => match platform.as_str() {
                "github" => github_client.as_ref().map(|c| c.has_auth()).unwrap_or(false),
                "bitbucket-cloud" => bitbucket_client.is_some(),
                "bitbucket-data-center" => bitbucket_data_center_client.is_some(),
                _ => false,
            },
            None => false,
        };

    // Get local branch names for filtering
    let local_branches: Vec<String> = worktrees
        .iter()
        .filter_map(|wt| wt.branch.as_ref().map(|b| clean_branch_name(b).to_string()))
        .collect();

    // Convert to display format
    let mut display_worktrees: Vec<WorktreeDisplay> = Vec::new();

    for wt in &worktrees {
        let branch = wt
            .branch
            .as_ref()
            .map(|b| clean_branch_name(b).to_string())
            .unwrap_or_else(|| {
                if wt.bare {
                    "(bare)".to_string()
                } else {
                    wt.head.chars().take(8).collect()
                }
            });

        // Fetch PR info if available
        let pr_info = if has_pr_info && !wt.bare && branch != "(bare)" {
            match &repo_info {
                Some((platform, owner_or_workspace, repo)) => {
                    let pr_result = fetch_pr_for_branch(
                        platform,
                        owner_or_workspace,
                        repo,
                        &branch,
                        &github_client,
                        &bitbucket_client,
                        &bitbucket_data_center_client,
                    )
                    .await;

                    match pr_result {
                        Ok(info) => info,
                        Err(_) => None,
                    }
                }
                None => None,
            }
        } else {
            None
        };

        display_worktrees.push(WorktreeDisplay { branch, pr_info });
    }

    // Display local worktrees
    if !display_worktrees.is_empty() {
        println!("{}", "Local Worktrees:".bold());
        println!();

        for worktree in &display_worktrees {
            display_worktree(worktree);
        }
    }

    // Fetch all open pull requests and add ones that don't have local worktrees
    let mut remote_prs: Vec<RemotePullRequest> = Vec::new();

    if has_pr_info && !local_only {
        if let Some((platform, owner_or_workspace, repo)) = &repo_info {
            match platform.as_str() {
                "github" => {
                    if let Some(ref client) = github_client {
                        if let Ok(all_prs) = client.get_all_pull_requests(owner_or_workspace, repo) {
                            for (pr, branch_name) in all_prs {
                                // Skip if we already have a local worktree for this branch
                                if !local_branches.contains(&branch_name) {
                                    let status = if pr.draft { "DRAFT" } else { "OPEN" };
                                    remote_prs.push(RemotePullRequest {
                                        branch: branch_name,
                                        pr_info: PullRequestInfo {
                                            url: pr.html_url,
                                            status: status.to_string(),
                                            title: pr.title.clone(),
                                        },
                                    });
                                }
                            }
                        }
                    }
                }
                "bitbucket-cloud" => {
                    if let Some(ref client) = bitbucket_client {
                        if let Ok(all_prs) = client.get_pull_requests(owner_or_workspace, repo).await {
                            for pr in all_prs {
                                // Only include open PRs
                                if pr.state == "OPEN" {
                                    let branch_name = pr.source.branch.name.clone();
                                    // Skip if we already have a local worktree for this branch
                                    if !local_branches.contains(&branch_name) {
                                        let url = extract_bitbucket_cloud_url(&pr);
                                        remote_prs.push(RemotePullRequest {
                                            branch: branch_name,
                                            pr_info: PullRequestInfo {
                                                url,
                                                status: "OPEN".to_string(),
                                                title: pr.title.clone(),
                                            },
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
                "bitbucket-data-center" => {
                    if let Some(ref client) = bitbucket_data_center_client {
                        if let Ok(all_prs) = client.get_pull_requests(owner_or_workspace, repo).await {
                            for pr in all_prs {
                                // Only include open PRs
                                if pr.state == "OPEN" {
                                    let branch_name = pr.from_ref.display_id.clone();
                                    // Skip if we already have a local worktree for this branch
                                    if !local_branches.contains(&branch_name) {
                                        let status = if pr.draft.unwrap_or(false) { "DRAFT" } else { "OPEN" };
                                        let url = extract_bitbucket_data_center_url(&pr);
                                        remote_prs.push(RemotePullRequest {
                                            branch: branch_name,
                                            pr_info: PullRequestInfo {
                                                url,
                                                status: status.to_string(),
                                                title: pr.title.clone(),
                                            },
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    // Display remote PRs if any exist
    if !remote_prs.is_empty() && !local_only {
        if !display_worktrees.is_empty() {
            println!(); // Add spacing between sections
        }
        println!("{}", "Open Pull Requests (no local worktree):".bold());
        println!();

        for pr in &remote_prs {
            display_remote_pr(pr);
        }
    }

    if !has_pr_info && !local_only {
        if let Some((_, config)) = config::GitWorktreeConfig::find_config()? {
            match config.source_control.as_str() {
                "bitbucket-cloud" => {
                    println!(
                        "\n{}",
                        "Tip: Run 'gwt auth bitbucket-cloud setup' to enable Bitbucket Cloud pull request information"
                            .dimmed()
                    );
                }
                "bitbucket-data-center" => {
                    println!("\n{}", "Tip: Run 'gwt auth bitbucket-data-center setup' to enable Bitbucket Data Center pull request information".dimmed());
                }
                _ => {
                    println!(
                        "\n{}",
                        "Tip: Run 'gh auth login' to enable GitHub pull request information".dimmed()
                    );
                }
            }
        }
    }

    Ok(())
}

fn display_worktree(worktree: &WorktreeDisplay) {
    // Display branch name in cyan
    println!("{}", worktree.branch.cyan());

    // Display PR info if available
    if let Some(ref pr_info) = worktree.pr_info {
        // Display URL with status
        let status_colored = match pr_info.status.as_str() {
            "OPEN" => "open".green(),
            "CLOSED" => "closed".red(),
            "MERGED" => "merged".green(),
            "DRAFT" => "draft".yellow(),
            _ => pr_info.status.normal(),
        };
        println!("  {} ({})", pr_info.url.blue().underline(), status_colored);

        // Display title if not empty
        if !pr_info.title.is_empty() {
            println!("  {}", pr_info.title.dimmed());
        }
    }
    println!(); // Empty line between worktrees
}

fn display_remote_pr(pr: &RemotePullRequest) {
    // Display branch name in cyan
    println!("{}", pr.branch.cyan());

    // Display URL with status
    let status_colored = match pr.pr_info.status.as_str() {
        "OPEN" => "open".green(),
        "CLOSED" => "closed".red(),
        "MERGED" => "merged".green(),
        "DRAFT" => "draft".yellow(),
        _ => pr.pr_info.status.normal(),
    };
    println!("  {} ({})", pr.pr_info.url.blue().underline(), status_colored);

    // Display title
    if !pr.pr_info.title.is_empty() {
        println!("  {}", pr.pr_info.title.dimmed());
    }
    println!(); // Empty line between PRs
}
