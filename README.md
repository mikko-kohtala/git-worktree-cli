# Git Worktree CLI (gwt)

**Tooling around git worktrees to make managing multiple branches easier**

Work on multiple branches simultaneously without stashing or switching. Never lose context when switching between features. One repository, multiple working directories:

```bash
code/
├── my-project/             # Main branch (the repo)
└── .worktrees/             # Shared by the repos in code/
    └── my-project/
        ├── feature-123/    # Feature branch
        └── bugfix-456/     # Bugfix branch
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
cd ../.worktrees/my-project/feature/user-auth    # Work on feature
cd ../.worktrees/my-project/hotfix/login-bug     # Fix urgent bug
cd ../.worktrees/my-project/feature/user-auth    # Back to feature

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

- `gwt init [--local] [--worktrees-root <dir>]` - Detect the current repo and write config (global by default)
- `gwt add <branch> [--ignore-hook-errors]` - Create a worktree under `.worktrees/<repo>` next to the repo
- `gwt list [--local]` - Show worktrees with PR status (`--local` skips remote PRs)
- `gwt remove [branch] [--force] [--ignore-hook-errors]` - Delete a worktree (interactive picker without a branch; confirms one by one, `a` removes all remaining without further questions)
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

### Where worktrees go

`gwt init` stores the worktrees folder as `worktreesPath` in the config. By default it is `.worktrees/<repo>` in the folder that holds the repo, so `~/code/my-project` gets `~/code/.worktrees/my-project/`. One hidden `.worktrees` folder serves all the repos in `~/code`. A repo at the home directory itself (dotfiles) is the exception: it gets `~/.worktrees/<user>/`, since its parent folder is not writable. Worktrees are not put inside the repo itself: tools that look for config in parent folders (Cargo workspaces, `node_modules`, ESLint, `CLAUDE.md`) would pick up the main checkout's files.

To keep all worktrees under one folder, pass `--worktrees-root`, or set it for every `gwt init` in `~/.config/git-worktree-cli/settings.jsonc`:

```json
{ "worktreesRoot": "~/.worktrees" }
```

Each project then gets a folder named after its path relative to home: `~/code/mikko/my-project` becomes `~/.worktrees/code-mikko-my-project/`. This works for repos without a remote. If two paths map to the same name (`code/my-app` and `code-my/app`), `gwt init` refuses the second one; pass another `--worktrees-root` or set `worktreesPath` by hand.

`gwt remove` deletes the folders a removal leaves empty, up to the shared `.worktrees` folder.

Projects set up before 0.19 keep their `<repo>-worktrees` folder, since the path is in their config. To switch, run `gwt init` again (it rewrites the config, so copy your hooks first) and recreate or `git worktree move` the existing worktrees.

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

If the worktree was already removed without gwt (plain `git worktree remove`, or `gh pr merge --delete-branch`), `gwt remove -f feature/x` still runs `preRemove` and `postRemove` from the project root, with `${worktreePath}` set to where `gwt add` would have put the worktree. It also deletes the local branch if it is still there and prunes stale worktree references. It fails with "Worktree for 'feature/x' not found" (non-zero exit) only when the project has no remove hooks and no such local branch. With hooks configured, a mistyped name runs the hooks for that name, so check the name before passing `-f`. Invalid branch names and protected branches (main, master, dev, develop and the configured `mainBranch`) are never cleaned up this way. An orphaned worktree (its `.git` file points to a missing git directory) also runs the remove hooks and has its branch deleted.

### When a hook fails

Hooks fail closed: a hook command that exits non-zero stops the remaining hooks of that type and makes gwt exit non-zero, with the hook's own output shown above the error.

| Hook | On failure |
| --- | --- |
| `preRemove` | Nothing is removed: the worktree and branch stay (also when the worktree was already removed; the branch is not deleted). Fix the problem and re-run `gwt remove`. |
| `postRemove` | The worktree is already gone, so gwt finishes (branch delete, prune), then exits non-zero. |
| `postAdd` | The new worktree is kept for retrying. Re-run the failed provisioning command in it, or `gwt remove -f <branch>` it. |

`--ignore-hook-errors` (on `gwt add` and `gwt remove`) turns hook failures back into warnings: the remaining hooks still run, the removal goes ahead and gwt exits 0, as before 0.18.0. Use it when a `preRemove` cleanup cannot succeed and you accept leaking whatever it would have cleaned up.

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
