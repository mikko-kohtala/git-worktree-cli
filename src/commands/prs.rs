use colored::Colorize;

use crate::{
    azure_devops, bitbucket_api, bitbucket_data_center_api,
    config::GitWorktreeConfig,
    error::{Error, Result},
    github,
};

pub fn run() -> Result<()> {
    let (_config_path, config) = GitWorktreeConfig::find_config()?
        .ok_or_else(|| Error::config("Config not found. Run 'gwt init' from your project directory to create one."))?;

    let url = pr_list_url(&config.source_control, &config.repository_url)?;

    println!("Opening pull requests: {}", url.blue().underline());

    open_in_browser(&url)
}

/// Build the provider's pull request list web URL from the configured repository
fn pr_list_url(source_control: &str, repo_url: &str) -> Result<String> {
    match source_control {
        "bitbucket-cloud" => {
            let (workspace, repo) = bitbucket_api::extract_bitbucket_info_from_url(repo_url)
                .ok_or_else(|| Error::provider(format!("Failed to parse Bitbucket Cloud URL: {}", repo_url)))?;
            Ok(format!("https://bitbucket.org/{}/{}/pull-requests", workspace, repo))
        }
        "bitbucket-data-center" => {
            let (base_url, project_key, repo_slug) =
                bitbucket_data_center_api::extract_bitbucket_data_center_info_from_url(repo_url).ok_or_else(|| {
                    Error::provider(format!("Failed to parse Bitbucket Data Center URL: {}", repo_url))
                })?;
            Ok(format!(
                "{}/projects/{}/repos/{}/pull-requests",
                base_url, project_key, repo_slug
            ))
        }
        "azure-devops" => {
            let (organization, project, repo) = azure_devops::AzureDevOpsClient::parse_azure_url(repo_url)
                .ok_or_else(|| Error::provider(format!("Failed to parse Azure DevOps URL: {}", repo_url)))?;
            Ok(azure_devops::pr_list_web_url(&organization, &project, &repo))
        }
        // GitHub (github.com or GitHub Enterprise *.ghe.com)
        _ => {
            let (host, owner, repo) = github::GitHubClient::parse_github_url_with_host(repo_url)
                .ok_or_else(|| Error::provider(format!("Failed to parse repository URL: {}", repo_url)))?;
            Ok(format!("https://{}/{}/{}/pulls", host, owner, repo))
        }
    }
}

fn open_in_browser(url: &str) -> Result<()> {
    #[cfg(target_os = "macos")]
    let cmd = "open";
    #[cfg(target_os = "linux")]
    let cmd = "xdg-open";
    #[cfg(target_os = "windows")]
    let cmd = "start";

    std::process::Command::new(cmd)
        .arg(url)
        .spawn()
        .map_err(|e| Error::msg(format!("Failed to open browser: {}", e)))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pr_list_url_github() {
        assert_eq!(
            pr_list_url("github", "git@github.com:owner/repo.git").unwrap(),
            "https://github.com/owner/repo/pulls"
        );
        assert_eq!(
            pr_list_url("github", "https://github.com/owner/repo").unwrap(),
            "https://github.com/owner/repo/pulls"
        );
    }

    #[test]
    fn test_pr_list_url_github_enterprise() {
        assert_eq!(
            pr_list_url("github", "acme@acme.ghe.com:owner/repo.git").unwrap(),
            "https://acme.ghe.com/owner/repo/pulls"
        );
    }

    #[test]
    fn test_pr_list_url_bitbucket_cloud() {
        assert_eq!(
            pr_list_url("bitbucket-cloud", "git@bitbucket.org:workspace/repo.git").unwrap(),
            "https://bitbucket.org/workspace/repo/pull-requests"
        );
    }

    #[test]
    fn test_pr_list_url_bitbucket_data_center() {
        assert_eq!(
            pr_list_url(
                "bitbucket-data-center",
                "https://git.acmeorg.com/scm/PROJECT/repository.git"
            )
            .unwrap(),
            "https://git.acmeorg.com/projects/PROJECT/repos/repository/pull-requests"
        );
    }

    #[test]
    fn test_pr_list_url_azure_devops() {
        assert_eq!(
            pr_list_url("azure-devops", "git@ssh.dev.azure.com:v3/myorg/MyProject/my-repo").unwrap(),
            "https://dev.azure.com/myorg/MyProject/_git/my-repo/pullrequests"
        );
    }

    #[test]
    fn test_pr_list_url_unparseable() {
        assert!(pr_list_url("github", "not-a-url").is_err());
    }
}
