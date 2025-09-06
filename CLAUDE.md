# Development Guide

## Commands

### Development
- **Build release binary**: `cargo build --release`
- **Build debug binary**: `cargo build`
- **Run tests**: `cargo test`
- **Type checking**: `cargo check`
- **Format code**: `cargo fmt`

### Version Management
When making code changes, increment the version in Cargo.toml:
- Patch version (x.x.N) for bug fixes
- Minor version (x.N.x) for new features
- Major version (N.x.x) for breaking changes

## Architecture

Single Rust binary providing git worktree management with these key modules:
- `commands/`: Command implementations (init, add, list, remove)
- `config.rs`: Configuration file handling  
- `git.rs`: Git operations with streaming output
- `hooks.rs`: Hook execution system
- `completions.rs`: Embedded shell completions

## Hooks System

Hooks allow custom commands after worktree operations. Define in `git-worktree-config.jsonc`:

```jsonc
{
  "hooks": {
    "postAdd": ["npm install", "npm run init"],
    "postRemove": ["echo 'Cleaned up ${branchName}'"]
  }
}
```

Variables: `${branchName}`, `${worktreePath}`

## Features

All core functionality is implemented:
- ✅ `gwt init` - Initialize from repository URLs with streaming output
- ✅ `gwt add` - Create worktrees with hook execution
- ✅ `gwt list` - Display worktrees with PR integration
- ✅ `gwt remove` - Remove with safety checks
- ✅ `gwt completions` - Auto-install shell completions
- ✅ Multi-provider support (GitHub, Bitbucket)

## Testing

Comprehensive test suite:
- Integration tests with `assert_cmd`
- Unit tests in source modules
- Real git repository testing
