# Development Guide

## Commands

### Development
- **Build release binary**: `cargo build --release`
- **Build debug binary**: `cargo build`
- **Run tests**: `cargo test`
- **Type checking**: `cargo check`
- **Lint code**: `cargo clippy -- -D warnings`
- **Format code**: `cargo fmt`

### Version Management
When making code changes, increment the version in Cargo.toml:
- Patch version (x.x.N) for bug fixes
- Minor version (x.N.x) for new features
- Major version (N.x.x) for breaking changes

## Architecture

Single Rust binary providing git worktree management with these key modules:
- `commands/`: CLI actions (init, add, list, remove, auth, completions)
- `core/`: Project discovery and worktree layout helpers
- `config.rs`: Config load/save, global/local discovery, worktrees path derivation
- `git.rs`: Git operations with streaming output and worktree parsing
- `hooks.rs`: Pre/post hook execution
- `github.rs`, `bitbucket_*`: PR integrations and auth helpers
- `completions.rs`: Completion content, install, and status checks

## Hooks System

Hooks allow custom commands around worktree operations. Define in `git-worktree-config.jsonc`:

```jsonc
{
  "hooks": {
    "postAdd": ["npm install", "npm run init"],
    "preRemove": ["echo Cleaning up ${branchName}"],
    "postRemove": ["echo Removed ${worktreePath}"]
  }
}
```

Variables: `${branchName}`, `${worktreePath}`

## Features

All core functionality is implemented:
- ✅ `gwt init` - Detect provider from origin and write config (global or `--local`)
- ✅ `gwt add` - Create worktrees under the derived `-worktrees` path and run hooks
- ✅ `gwt list` - Show local worktrees with PR status (`--local` skips remote PRs)
- ✅ `gwt remove` - Safe removal with `--force`, handles orphaned worktrees, runs hooks
- ✅ `gwt auth` - GitHub + Bitbucket Cloud/Data Center setup and test helpers
- ✅ `gwt completions` - Status, install, and generate completions
- ✅ Multi-provider support (GitHub, Bitbucket Cloud, Bitbucket Data Center)

## Testing

Targeted test coverage:
- Integration tests for init flows and config path derivation using `assert_cmd`
- Unit tests in `src/config.rs` and Bitbucket auth helpers
- Integration tests create real git repos in temp directories
