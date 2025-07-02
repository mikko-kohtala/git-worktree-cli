/// Main/master branch names that are protected from deletion
pub const PROTECTED_BRANCHES: &[&str] = &["main", "master", "dev", "develop"];

/// Default main branch names to check
#[allow(dead_code)]
pub const DEFAULT_MAIN_BRANCHES: &[&str] = &["main", "master"];

/// Git provider detection patterns
#[allow(dead_code)]
pub const GITHUB_HOST: &str = "github.com";
#[allow(dead_code)]
pub const BITBUCKET_CLOUD_HOST: &str = "bitbucket.org";

/// API endpoints
#[allow(dead_code)]
pub const BITBUCKET_API_BASE: &str = "https://api.bitbucket.org/2.0";

/// Hook environment variables
#[allow(dead_code)]
pub const HOOK_ENV_FORCE_COLOR: &str = "1";

/// Git command patterns
#[allow(dead_code)]
pub const GIT_REFS_HEADS_PREFIX: &str = "refs/heads/";
#[allow(dead_code)]
pub const GIT_REFS_REMOTES_PREFIX: &str = "refs/remotes/";

/// File extensions
#[allow(dead_code)]
pub const GIT_EXTENSION: &str = ".git";

/// Configuration file
#[allow(dead_code)]
pub const CONFIG_FILE_NAME: &str = "git-worktree-config.yaml";
