use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "gwt",
    version,
    author,
    about = "Git worktree management tool",
    long_about = "\
A tool for managing git worktrees efficiently with hooks and configuration support.

gwt organizes git worktrees so you can work on multiple branches simultaneously.
Instead of stashing and switching branches, each branch gets its own directory:

  my-repo/                    <- main repository (run gwt commands here)
  my-repo-worktrees/          <- created automatically by gwt
    feature/auth/             <- each branch is a separate directory
    bugfix/fix-123/

Switch between branches by changing directories with cd.

Typical workflow:
  1. gwt init              (one-time setup inside your repository)
  2. gwt add <branch>      (create worktree for a branch)
  3. cd into the worktree directory to work on the branch
  4. gwt list              (see all worktrees with PR status)
  5. gwt remove <branch>   (clean up when done)",
    after_long_help = "\
EXAMPLES:
  # One-time setup inside your repo
  cd ~/projects/my-repo && gwt init

  # Create a worktree for a new feature branch
  gwt add feature/user-auth

  # Switch to it
  cd ../my-repo-worktrees/feature/user-auth

  # See all worktrees and their PR status
  gwt list

  # Remove a worktree when done (--force skips confirmation)
  gwt remove feature/user-auth --force

CONFIG:
  Global: ~/.config/git-worktree-cli/projects/<repo>.jsonc
  Local:  ./git-worktree-config.jsonc (with gwt init --local)

  Config supports hooks (postAdd, preRemove, postRemove) that run
  shell commands automatically. Variables: ${branchName}, ${worktreePath}

PROVIDERS:
  GitHub (via gh CLI), Bitbucket Cloud, Bitbucket Data Center
  Run 'gwt auth <provider>' to set up PR integration for 'gwt list'.",
    disable_version_flag = true
)]
pub struct Cli {
    /// Print version
    #[arg(short = 'v', long = "version", action = clap::ArgAction::Version)]
    pub version: (),

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum CompletionAction {
    /// Generate completions to stdout
    Generate {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
    /// Install completions for your shell
    Install {
        /// Shell to install completions for (auto-detected if not specified)
        #[arg(value_enum)]
        shell: Option<clap_complete::Shell>,
    },
}

#[derive(Subcommand)]
pub enum AuthAction {
    /// Authenticate with GitHub
    Github,
    /// Authenticate with Bitbucket Cloud
    BitbucketCloud {
        #[command(subcommand)]
        action: Option<BitbucketCloudAuthAction>,
    },
    /// Authenticate with Bitbucket Data Center
    BitbucketDataCenter {
        #[command(subcommand)]
        action: Option<BitbucketDataCenterAuthAction>,
    },
}

#[derive(Subcommand)]
pub enum BitbucketCloudAuthAction {
    /// Show setup instructions
    Setup,
    /// Test the authentication connection
    Test,
}

#[derive(Subcommand)]
pub enum BitbucketDataCenterAuthAction {
    /// Show setup instructions
    Setup,
    /// Test the authentication connection
    Test,
}

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum Provider {
    /// GitHub repository
    Github,
    /// Bitbucket Cloud repository
    BitbucketCloud,
    /// Bitbucket Data Center repository
    BitbucketDataCenter,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize git-worktree-cli for an existing repository
    #[command(long_about = "\
Initialize git-worktree-cli for an existing repository.

Run this once inside a git repository that has a remote origin. gwt will:
  - Detect the provider (GitHub, Bitbucket Cloud, Bitbucket Data Center)
  - Detect the default branch from the remote
  - Derive the worktrees path (<repo-name>-worktrees/ as a sibling directory)
  - Save configuration globally (~/.config/git-worktree-cli/projects/)

Use --local to save config as git-worktree-config.jsonc next to the repo instead.

The config file can be edited to add hooks (postAdd, preRemove, postRemove)
that run automatically when creating or removing worktrees.")]
    Init {
        /// Write config to project directory instead of global location
        #[arg(long)]
        local: bool,
    },

    /// Add a new worktree for a branch
    #[command(long_about = "\
Add a new worktree for a branch.

Creates a git worktree at <repo>-worktrees/<branch-name>/. If the branch
exists locally or on the remote, it checks out that branch. Otherwise,
creates a new branch from origin/<main-branch>.

The command fetches from origin first to ensure the latest remote state.
After creating the worktree, any postAdd hooks from the config are executed
in the new worktree directory.

Branch names can include slashes (e.g., feature/user-auth, bugfix/fix-123).
The directory structure mirrors the branch name.")]
    Add {
        /// Branch name (can include slashes like feature/branch-name)
        branch_name: String,
    },

    /// List all worktrees in the current project
    #[command(long_about = "\
List all worktrees in the current project.

Shows local worktrees with their branch names. If authentication is
configured (via gwt auth), also shows PR status (open, draft, merged,
closed) and PR URLs for each branch.

Additionally shows open pull requests that have no local worktree,
making it easy to check out branches that need review.

Use --local to skip fetching remote PR information (faster, offline).

Can be run from the main repository or from any worktree directory.")]
    List {
        /// Show only local worktrees (skip remote PRs)
        #[arg(short, long)]
        local: bool,
    },

    /// Remove a worktree
    #[command(long_about = "\
Remove a worktree.

Removes the worktree directory and deletes the branch (unless it is a
protected branch: main, master, dev, develop). Asks for confirmation
before proceeding unless --force is used.

If the branch has unmerged changes, asks again before force-deleting
the branch. Use --force to skip all confirmation prompts.

If no branch name is given, removes the worktree for the current
directory. Also handles orphaned worktrees with stale git references.

Runs preRemove hooks before removal and postRemove hooks after.

NOTE: --force is required for non-interactive (AI agent) usage.")]
    Remove {
        /// Branch name to remove (current worktree if not specified)
        branch_name: Option<String>,
        /// Skip confirmation prompts
        #[arg(short, long)]
        force: bool,
    },

    /// Manage authentication for external services
    #[command(long_about = "\
Manage authentication for external services.

Authentication enables PR status display in 'gwt list'. Each provider
has its own setup flow:

  github                - Uses the gh CLI. Run 'gh auth login' to set up.
  bitbucket-cloud       - Uses app passwords stored in the system keychain.
  bitbucket-data-center - Uses personal access tokens in the system keychain.

Use 'gwt auth <provider> setup' for setup instructions and
'gwt auth <provider> test' to verify the connection.")]
    Auth {
        #[command(subcommand)]
        action: AuthAction,
    },

    /// Generate or install shell completions
    #[command(long_about = "\
Generate or install shell completions.

Without a subcommand, checks whether completions are installed for
your current shell. Supports bash, zsh, fish, powershell, and elvish.

Use 'gwt completions install' to auto-install for your detected shell.
Use 'gwt completions generate <shell>' to output the completion script
to stdout (useful for piping or manual installation).")]
    Completions {
        /// Action to perform (defaults to generate)
        #[command(subcommand)]
        action: Option<CompletionAction>,
    },

    /// Open the project config file
    #[command(long_about = "\
Open the project config file in the default application.

Finds the config file (local or global) for the current repository
and opens it with the system default application (e.g., your text editor).

Local config:  ./git-worktree-config.jsonc
Global config: ~/.config/git-worktree-cli/projects/<repo>.jsonc")]
    Config,
}
