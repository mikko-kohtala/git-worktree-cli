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

use crate::cli::{BitbucketCloudAuthAction, BitbucketDataCenterAuthAction};

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
