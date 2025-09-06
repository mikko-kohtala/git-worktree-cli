# CLAUDE.md

This file provides guidance to [Claude Code](https://www.anthropic.com/claude-code) when working with this repository.

## Quick Commands

- **Build**: `cargo build --release` (optimized) or `cargo build` (debug)
- **Test**: `cargo test` - Comprehensive unit and integration tests
- **Format**: `cargo fmt` - Format code according to rustfmt.toml
- **Run**: `cargo run -- <command>` - Run directly with cargo

## Testing Workflow

For local testing:
```bash
mkdir test-temp && cd test-temp
../target/release/gwt init https://github.com/octocat/Hello-World.git
../target/release/gwt list
cd .. && rm -rf test-temp
```

## Version Management

**Always increment version in Cargo.toml when making changes:**
- Patch (x.x.N): Bug fixes
- Minor (x.N.x): New features  
- Major (N.x.x): Breaking changes

## Core Commands

- `gwt init <url>` - Initialize worktree project from repository
- `gwt add <branch>` - Create new worktree for branch
- `gwt list` - List all worktrees in formatted table
- `gwt remove [branch]` - Remove worktree with confirmation
- `gwt completions` - Manage shell completions
- `gwt auth <provider>` - Manage authentication

## Architecture

**Single Rust binary** with:
- `src/main.rs` - CLI entry point with clap
- `src/commands/` - Individual command implementations
- `src/completions.rs` - Embedded shell completions
- `src/config.rs` - JSONC configuration handling
- `src/hooks.rs` - Hook execution system
- `build.rs` - Build-time completion generation

**Key Features:**
- Real-time streaming output for git operations
- Embedded shell completions (bash, zsh, fish, powershell, elvish)
- Multi-provider support (GitHub, Bitbucket Cloud/Data Center)
- Secure credential storage via system keyring
- Hooks system with variable substitution

## Hooks System

Hooks run custom commands after worktree operations:

```jsonc
{
  "hooks": {
    "postAdd": ["npm install", "npm run build"],
    "postRemove": ["echo 'Cleaned up ${branchName}'"]
  }
}
```

Variables: `${branchName}`, `${worktreePath}`

## Testing

- **Unit tests**: Embedded in source modules
- **Integration tests**: Real git repository testing with `assert_cmd`
- **Fast execution**: All tests complete in ~6 seconds
