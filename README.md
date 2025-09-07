# Git Worktree CLI (gwt)

Just a test

**Tooling around git worktrees to make managing multiple branches easier**

Work on multiple branches simultaneously without stashing or switching. Never lose context switching between features. One repository, multiple working directories:

```bash
my-project/
├── main/           # Main branch
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
# Setup once per project
gwt init git@github.com:company/app.git
cd main

# Create branches instantly
gwt add feature/user-auth
gwt add hotfix/login-bug

# Switch contexts with cd (no stashing)
cd ../feature/user-auth    # Work on feature
cd ../hotfix/login-bug     # Fix urgent bug
cd ../feature/user-auth    # Back to feature

# See all work with PR status
gwt list
# ┌───────────────────┬─────────────────────────────┐
# │ BRANCH            │ PULL REQUEST                │
# │ main              │ -                           │
# │ feature/user-auth │ #42 (open)                  │
# │ hotfix/login-bug  │ #41 (merged)               │
# └───────────────────┴─────────────────────────────┘

# Clean up finished work
gwt remove hotfix/login-bug
```

## Commands

- `gwt init <url>` - Setup project from repository
- `gwt add <branch>` - Create branch directory  
- `gwt list` - Show branches with PR status
- `gwt remove [branch]` - Delete branch directory
- `gwt auth <provider>` - Setup GitHub/Bitbucket auth

## Automation

Auto-run commands when creating/removing branches. Edit `git-worktree-config.jsonc`:

```json
{
  "hooks": {
    "postAdd": [
      "npm install",
      "npm run init"
    ]
  }
}
```

Now `gwt add feature/x` automatically installs dependencies.

## PR Integration

Setup once to see PR status in `gwt list`:

**GitHub**: `gh auth login`
**Bitbucket**: `gwt auth bitbucket-cloud setup`

Works with GitHub, Bitbucket Cloud, and Bitbucket Data Center.

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
