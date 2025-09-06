# Git Worktree CLI (gwt)

Fast, ergonomic git worktree management with a single Rust binary. Create, list and remove worktrees with real-time streaming output, optional hooks, and pull request info from GitHub and Bitbucket.

## Features

- Real-time git output for clone and worktree operations
- Create/list/remove worktrees safely with helpful prompts
- Embedded shell completions with one-step install
- PR overview in `gwt list` (GitHub, Bitbucket Cloud & Data Center)
- Secure auth storage (system keyring) for Bitbucket
- Optional hooks on add/remove (postAdd, postRemove)

## Install

Build from source:

```bash
cargo install --path .            # installs `gwt` to ~/.cargo/bin
# or
cargo build --release && sudo cp target/release/gwt /usr/local/bin/
```

Install shell completions (recommended):

```bash
gwt completions           # shows status and guidance
gwt completions install   # auto-detects shell and installs
```

## Quick Start

```bash
# Initialize a project (detects default branch, writes git-worktree-config.jsonc)
gwt init <repo-url>

# Optionally specify provider if auto-detect fails
gwt init <bitbucket-url> --provider bitbucket-cloud
gwt init <on-prem-bb-url> --provider bitbucket-data-center

# Create and list work
gwt add feature/new-ui
gwt list

# Remove when done
gwt remove feature/new-ui     # or run inside the worktree with `gwt remove`
```

## PR Integration

- GitHub: requires `gh` CLI and `gh auth login`
- Bitbucket Cloud: `gwt auth bitbucket-cloud setup` then `gwt auth bitbucket-cloud test`
- Bitbucket Data Center: `gwt auth bitbucket-data-center setup` then `gwt auth bitbucket-data-center test`

`gwt list` shows PR link and status for each branch; it can also list open PRs without local worktrees.

## Requirements

- Rust 1.70+ (to build), Git 2.5+
- Shell for completions: bash, zsh, fish, powershell, or elvish

## Contributing

- Fork, create a branch, make changes
- Run tests with `cargo test`
- Open a pull request

## License

MIT — see `LICENSE`.

