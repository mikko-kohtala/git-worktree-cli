---
name: gwt
description: Use the gwt (git-worktree-cli) tool for managing git worktrees. Use when the user needs to work on multiple branches simultaneously, create worktrees, list branches with PR status, remove finished worktrees, or set up repository configuration for worktree workflows. Triggers on git worktree tasks, branch management, or when user mentions gwt commands.
---

# gwt - Git Worktree CLI

Manage git worktrees so you can work on multiple branches simultaneously without stashing or switching. Each branch gets its own directory.

## Directory Structure

```
~/projects/
  my-repo/                      # Main repository (run gwt init here)
  my-repo-worktrees/            # Created automatically by gwt
    feature/auth/               # Worktree for feature/auth branch
    bugfix/fix-123/             # Worktree for bugfix/fix-123 branch
```

Worktrees are sibling directories to the main repo, with `-worktrees` suffix.

## Quick Reference

```bash
gwt init                          # One-time setup (run inside the repo)
gwt init --local                  # Store config next to repo instead of globally
gwt add <branch>                  # Create worktree for a branch
gwt list                          # List worktrees with PR status
gwt list --local                  # List worktrees without remote PR info
gwt remove <branch>              # Remove worktree (interactive confirmation)
gwt remove <branch> --force      # Remove worktree (no confirmation, use for automation)
gwt remove                        # Remove current worktree
gwt auth github                   # Check GitHub auth status
gwt auth bitbucket-cloud setup    # Set up Bitbucket Cloud auth
gwt auth bitbucket-cloud test     # Test Bitbucket Cloud connection
gwt completions install           # Install shell tab completions
```

## Best Practices for AI Agents

1. **Always use `--force` with `gwt remove`** - The default remove flow is interactive and requires stdin confirmation
2. **Run gwt commands from the main repo or any worktree** - gwt auto-discovers the project from the current directory
3. **Use `gwt list --local` for fast status** - Skips remote PR API calls when you only need local worktrees
4. **After `gwt add`, cd into the worktree** - The new worktree is at `<repo>-worktrees/<branch-name>/`
5. **Branch names with slashes are supported** - Use names like `feature/my-feature` or `bugfix/issue-42`
6. **gwt init is one-time per repo** - Run once inside the main repository; auto-detects provider and default branch
7. **Protected branches** - main, master, dev, develop cannot be deleted by `gwt remove`

## Commands

### gwt init

Initialize gwt for an existing git repository. Must be run from inside a repo with a remote origin.

- Detects provider (GitHub, Bitbucket Cloud, Bitbucket Data Center) from remote URL
- Detects the default branch from the remote
- Derives worktrees path as `<repo-name>-worktrees/` sibling directory
- Saves config globally by default

**Options:**

- `--local` - Save config as `git-worktree-config.jsonc` in the parent directory instead of `~/.config/git-worktree-cli/projects/`

```bash
cd ~/projects/my-app
gwt init
# Config saved to ~/.config/git-worktree-cli/projects/github_owner_my-app.jsonc

gwt init --local
# Config saved to ./git-worktree-config.jsonc
```

### gwt add \<branch\>

Create a new worktree for a branch.

- Fetches from origin first to get latest remote state
- If branch exists locally: checks out the existing local branch
- If branch exists on remote only: checks out the remote branch
- If branch is new: creates it from `origin/<main-branch>`
- Runs `postAdd` hooks after creating the worktree

**Arguments:**

- `<branch>` (required) - Branch name, supports slashes like `feature/name`

```bash
gwt add feature/user-auth
# Creates ~/projects/my-app-worktrees/feature/user-auth/

cd ../my-app-worktrees/feature/user-auth
```

### gwt list

List all worktrees with optional PR status.

- Shows all local worktrees with branch names
- With auth configured: shows PR URL, status (open/draft/merged/closed), and title
- Shows open PRs that have no local worktree
- Works from main repo or any worktree directory

**Options:**

- `--local` / `-l` - Skip remote PR info (faster, works offline)

**Example output:**

```
Local Worktrees:

main

feature/user-auth
  https://github.com/owner/repo/pull/42 (open)
  Add user authentication

Open Pull Requests (no local worktree):

bugfix/fix-login
  https://github.com/owner/repo/pull/43 (open)
  Fix login redirect bug
```

### gwt remove [branch] [--force]

Remove a worktree and delete its branch.

- Without branch name: removes the worktree for the current directory
- Removes the worktree directory and the git branch
- Protected branches (main, master, dev, develop) are preserved
- Handles orphaned worktrees (stale git references)
- Runs `preRemove` hooks before removal and `postRemove` hooks after

**Arguments:**

- `[branch]` (optional) - Branch name, defaults to current worktree

**Options:**

- `--force` / `-f` - Skip all confirmation prompts. **Required for non-interactive use.**

```bash
gwt remove feature/user-auth --force
```

### gwt auth \<provider\>

Set up authentication for PR status in `gwt list`.

**Providers:**

- `github` - Uses the `gh` CLI. Run `gh auth login` to authenticate
- `bitbucket-cloud` - Uses app passwords. Run `gwt auth bitbucket-cloud setup` for instructions
- `bitbucket-data-center` - Uses personal access tokens. Run `gwt auth bitbucket-data-center setup` for instructions

**Subcommands:**

- `setup` - Show setup instructions
- `test` - Test the authentication connection

```bash
gwt auth github                       # Check GitHub auth status
gwt auth bitbucket-cloud setup        # Show Bitbucket Cloud setup instructions
gwt auth bitbucket-data-center test   # Test Bitbucket DC connection
```

### gwt completions

Manage shell tab completions.

- Without subcommand: checks if completions are installed
- `install [shell]` - Install completions (auto-detects shell if not specified)
- `generate <shell>` - Output completion script to stdout
- Supported shells: bash, zsh, fish, powershell, elvish

```bash
gwt completions                # Check installation status
gwt completions install        # Auto-install for detected shell
gwt completions generate zsh   # Output zsh completions to stdout
```

## Configuration

Config file: `git-worktree-config.jsonc` (JSONC format, supports comments)

**Global location:** `~/.config/git-worktree-cli/projects/<provider_owner_repo>.jsonc`
**Local location:** `./git-worktree-config.jsonc` (created with `gwt init --local`)

Local config takes priority over global config.

```jsonc
{
  "repositoryUrl": "git@github.com:owner/repo.git",
  "mainBranch": "main",
  "createdAt": "2025-01-01T00:00:00Z",
  "sourceControl": "github",
  "projectPath": "/home/user/projects/repo",
  "worktreesPath": "/home/user/projects/repo-worktrees",
  "hooks": {
    "postAdd": ["npm install", "npm run init"],
    "preRemove": ["echo Cleaning up ${branchName}"],
    "postRemove": ["echo Removed ${worktreePath}"]
  }
}
```

**Fields:**

- `repositoryUrl` - Remote origin URL
- `mainBranch` - Default branch name (used as base for new branches)
- `sourceControl` - Provider: `github`, `bitbucket-cloud`, or `bitbucket-data-center`
- `projectPath` - Absolute path to the main repository
- `worktreesPath` - Absolute path to the worktrees directory
- `hooks` - Commands to run on worktree operations (optional)

## Hooks

Hooks run shell commands at specific points in worktree operations.

| Hook         | Runs                          | Working Directory        |
| ------------ | ----------------------------- | ------------------------ |
| `postAdd`    | After creating a worktree     | New worktree directory   |
| `preRemove`  | Before removing a worktree    | Worktree being removed   |
| `postRemove` | After removing a worktree     | Project root             |

**Available variables:**

- `${branchName}` - The branch name
- `${worktreePath}` - Absolute path to the worktree directory

Hooks continue executing even if one fails.

## Typical Workflow

```bash
# 1. One-time setup
cd ~/projects/my-app
gwt init

# 2. Start working on a feature
gwt add feature/new-dashboard
cd ../my-app-worktrees/feature/new-dashboard

# 3. Handle an urgent bug (no need to stash)
gwt add hotfix/fix-crash
cd ../../my-app-worktrees/hotfix/fix-crash
# fix the bug, commit, push

# 4. Check all work
gwt list

# 5. Clean up finished branches
gwt remove hotfix/fix-crash --force
```

## Prerequisites

- Git 2.5+ (for worktree support)
- `gh` CLI (for GitHub PR integration, optional)
