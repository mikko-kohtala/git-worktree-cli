use colored::Colorize;

use super::list_helpers::{
    colored_pr_status, extract_bitbucket_cloud_url, extract_bitbucket_data_center_url, PrContext, PullRequestInfo,
};
use crate::{
    config,
    core::project::{clean_branch_name, find_git_directory},
    error::Result,
    git,
};

struct WorktreeDisplay {
    branch: String,
    path: String,
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
    let ctx = PrContext::detect()?;
    let has_pr_info = ctx.has_pr_info();

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
            ctx.fetch_pr(&branch).await
        } else {
            None
        };

        display_worktrees.push(WorktreeDisplay {
            branch,
            path: wt.path.display().to_string(),
            pr_info,
        });
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
        if let Some((platform, owner_or_workspace, repo)) = &ctx.repo_info {
            match platform.as_str() {
                "github" => {
                    if let Some(ref client) = ctx.github_client {
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
                    if let Some(ref client) = ctx.bitbucket_client {
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
                    if let Some(ref client) = ctx.bitbucket_data_center_client {
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

    // Display worktree directory
    println!("  {}", worktree.path.dimmed());

    // Display PR info if available
    if let Some(ref pr_info) = worktree.pr_info {
        // Display URL with status
        println!(
            "  {} ({})",
            pr_info.url.blue().underline(),
            colored_pr_status(&pr_info.status)
        );

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
    println!(
        "  {} ({})",
        pr.pr_info.url.blue().underline(),
        colored_pr_status(&pr.pr_info.status)
    );

    // Display title
    if !pr.pr_info.title.is_empty() {
        println!("  {}", pr.pr_info.title.dimmed());
    }
    println!(); // Empty line between PRs
}
