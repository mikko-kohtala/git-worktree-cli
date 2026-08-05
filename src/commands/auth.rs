use crate::bitbucket_api::BitbucketClient;
use crate::bitbucket_auth::{self, BitbucketAuth};
use crate::bitbucket_data_center_api::BitbucketDataCenterClient;
use crate::bitbucket_data_center_auth::{self, BitbucketDataCenterAuth};
use crate::error::Result;
use crate::github::GitHubClient;

pub fn run() -> Result<()> {
    let client = GitHubClient::new();
    if client.has_auth() {
        println!("✓ You are already authenticated with GitHub via gh CLI");
        println!("Run 'gh auth logout' to remove credentials if needed");
    } else {
        println!("Please authenticate with GitHub using: gh auth login");
    }
    Ok(())
}

use crate::cli::{AzureDevopsAuthAction, BitbucketCloudAuthAction, BitbucketDataCenterAuthAction};

#[tokio::main]
pub async fn run_bitbucket_cloud(action: Option<BitbucketCloudAuthAction>) -> Result<()> {
    match action {
        None | Some(BitbucketCloudAuthAction::Setup) => {
            bitbucket_auth::display_setup_instructions();
        }
        Some(BitbucketCloudAuthAction::Test) => {
            let (workspace, repo, email) = bitbucket_auth::get_auth_from_config()?;
            let auth = BitbucketAuth::new(workspace, repo, email)?;
            let client = BitbucketClient::new(auth);
            client.test_connection().await?;
        }
    }
    Ok(())
}

pub fn run_azure_devops(action: Option<AzureDevopsAuthAction>) -> Result<()> {
    match action {
        None | Some(AzureDevopsAuthAction::Setup) => {
            println!("Azure DevOps PR integration uses the az CLI:");
            println!("  1. Install the Azure CLI: https://learn.microsoft.com/cli/azure/install-azure-cli");
            println!("  2. Add the DevOps extension: az extension add --name azure-devops");
            println!("  3. Authenticate: az login  (or 'az devops login' with a PAT)");
            println!("\nThen 'gwt list' will show PR status for Azure DevOps repositories.");
            println!("Verify with: gwt auth azure-devops test");
        }
        Some(AzureDevopsAuthAction::Test) => {
            let (_, config) = crate::config::GitWorktreeConfig::find_config()?
                .ok_or_else(|| crate::error::Error::config("No gwt config found — run 'gwt init' first"))?;
            let (organization, project, repo) =
                crate::azure_devops::AzureDevOpsClient::parse_azure_url(&config.repository_url).ok_or_else(|| {
                    crate::error::Error::config(format!(
                        "Not an Azure DevOps repository URL: {}",
                        config.repository_url
                    ))
                })?;
            let client = crate::azure_devops::AzureDevOpsClient::new(organization, project);
            client.test_connection(&repo)?;
            println!("✓ Azure DevOps connection works ({})", config.repository_url);
        }
    }
    Ok(())
}

#[tokio::main]
pub async fn run_bitbucket_data_center(action: Option<BitbucketDataCenterAuthAction>) -> Result<()> {
    match action {
        None | Some(BitbucketDataCenterAuthAction::Setup) => {
            bitbucket_data_center_auth::display_setup_instructions();
        }
        Some(BitbucketDataCenterAuthAction::Test) => {
            let (base_url, project_key, repo_slug) = bitbucket_data_center_auth::get_auth_from_config()?;
            let auth = BitbucketDataCenterAuth::new(project_key, repo_slug, base_url.clone())?;
            let client = BitbucketDataCenterClient::new(auth, base_url);
            client.test_connection().await?;
        }
    }
    Ok(())
}
