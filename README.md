# Git Worktree CLI (gwt)

**Tooling around git worktrees to make managing multiple branches easier**

Work on multiple branches simultaneously without stashing or switching. Never lose context when switching between features. One repository, multiple working directories:

```bash
my-project/                # Main branch (the repo)
my-project-worktrees/
├── feature-123/            # Feature branch
└── bugfix-456/             # Bugfix branch
```

Each directory is independent. `cd` to switch between branches.

## Installation

```bash
git clone https://github.com/mikko-kohtala/git-worktree-cli.git
cd git-worktree-cli
cargo build --release && cargo install --path .
gwt completions install  # Optional: tab completion
```

## Daily Workflow

```bash
# Setup once per project (run inside the repo)
cd my-project
gwt init
# gwt init --local          # Store config next to the repo

# Create branches instantly
gwt add feature/user-auth
gwt add hotfix/login-bug

# Switch contexts with cd (no stashing)
cd ../my-project-worktrees/feature/user-auth    # Work on feature
cd ../my-project-worktrees/hotfix/login-bug     # Fix urgent bug
cd ../my-project-worktrees/feature/user-auth    # Back to feature

# See all work with PR status
gwt list
# Local Worktrees:
#
# main
#
# feature/user-auth
#   https://github.com/company/app/pull/42 (open)
#   Add user auth
#
# Open Pull Requests (no local worktree):
# hotfix/login-bug
#   https://github.com/company/app/pull/43 (open)
#   Fix login bug

# Clean up finished work
gwt remove hotfix/login-bug
```

## Commands

- `gwt init [--local]` - Detect the current repo and write config (global by default)
- `gwt add <branch>` - Create a worktree under `<repo>-worktrees`
- `gwt list [--local]` - Show worktrees with PR status (`--local` skips remote PRs)
- `gwt remove [branch] [--force]` - Delete a worktree (interactive picker without a branch; confirms one by one, `a` removes all remaining without further questions)
- `gwt cd [branch]` - Print the worktrees folder (or a worktree) path; the shell wrapper installed by `gwt completions install` (bash, zsh, fish) makes it change directory
- `gwt prs` - Open the provider's pull request list in the browser
- `gwt config` - Open the project config file in the default application
- `gwt auth github` - Check GitHub auth (uses `gh`)
- `gwt auth bitbucket-cloud [setup|test]` - Configure or test Bitbucket Cloud auth
- `gwt auth bitbucket-data-center [setup|test]` - Configure or test Bitbucket Data Center auth
- `gwt auth azure-devops [setup|test]` - Configure or test Azure DevOps auth (uses the `az` CLI with the azure-devops extension)
- `gwt completions` - Check completion installation status
- `gwt completions install [shell]` - Install completions (auto-detects shell)
- `gwt completions generate <shell>` - Output completion script to stdout
- Supported shells: bash, zsh, fish, powershell, elvish

## Configuration

Config is stored globally by default at `~/.config/git-worktree-cli/projects/`. Use `gwt init --local` to store `git-worktree-config.jsonc` next to your repo instead.

## Automation

Auto-run commands when creating/removing branches. Edit `git-worktree-config.jsonc`:

```json
{
  "hooks": {
    "postAdd": [
      "npm install",
      "npm run init"
    ],
    "preRemove": [
      "echo Cleaning up ${branchName}"
    ],
    "postRemove": [
      "echo Removed ${worktreePath}"
    ]
  }
}
```

Variables: `${branchName}`, `${worktreePath}`.

Now `gwt add feature/x` and `gwt remove feature/x` run hooks automatically.

If the worktree was already removed without gwt (plain `git worktree remove`, or `gh pr merge --delete-branch`), `gwt remove -f feature/x` still runs `preRemove` and `postRemove` from the project root, with `${worktreePath}` set to where `gwt add` would have put the worktree. It also deletes the local branch if it is still there and prunes stale worktree references. It fails with "Worktree for 'feature/x' not found" (non-zero exit) only when the project has no remove hooks and no such local branch. With hooks configured, a mistyped name runs the hooks for that name, so check the name before passing `-f`. Protected branches (main, master, dev, develop) are never cleaned up this way.

## PR Integration

Setup once to see PR status in `gwt list`:

**GitHub**: `gh auth login` (or `gwt auth github`)
**Bitbucket Cloud**: `gwt auth bitbucket-cloud setup`
**Bitbucket Data Center**: `gwt auth bitbucket-data-center setup`
**Azure DevOps**: `gwt auth azure-devops setup`

Works with GitHub, Bitbucket Cloud, Bitbucket Data Center, and Azure DevOps.

## Why This Makes Work Easier

- **No stashing** - Switch branches instantly with `cd`
- **No losing context** - Each branch keeps its state
- **Parallel work** - Handle urgent fixes without disrupting features
- **Automated setup** - Dependencies install automatically via hooks
- **PR visibility** - See all pull requests from terminal

## Requirements

- Git 2.5+
- Rust 1.70+ (for building)

---

**MIT License** • Contributions welcome
