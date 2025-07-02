# Git Worktree CLI - Source Code Structure

This directory contains the Rust source code for the git worktree CLI tool (`gwt`).

## Architecture Overview

The codebase follows a modular architecture with clear separation of concerns:

### Core Modules

- **`lib.rs`** - Library root that exposes the public API
- **`main.rs`** - Thin binary entry point that uses the library
- **`error.rs`** - Centralized error handling with custom error types
- **`constants.rs`** - Application-wide constants

### Business Logic

- **`core/`** - Core business logic independent of CLI and external APIs
  - `project.rs` - Project discovery and management (finding project roots, git directories)
  - `utils.rs` - Shared utility functions

### Command Line Interface

- **`cli.rs`** - Command-line argument definitions using clap
- **`commands/`** - Individual command implementations
  - `init.rs` - Initialize new worktree projects
  - `add.rs` - Add worktrees from branches
  - `list.rs` - List worktrees with PR information
  - `remove.rs` - Remove worktrees safely
  - `auth.rs` - Authentication management

### External Integrations

- **`providers/`** - Trait-based API provider architecture
  - `mod.rs` - Provider trait definition
  - `github.rs` - GitHub integration
  - `bitbucket_cloud.rs` - Bitbucket Cloud integration
  - `bitbucket_server.rs` - Bitbucket Server/Data Center integration

### Git Operations

- **`git.rs`** - Low-level git command execution with streaming output
- **`hooks.rs`** - Hook execution system for post-add/remove actions

### Configuration

- **`config.rs`** - YAML configuration file handling
- **`completions.rs`** - Shell completion generation and installation

### Authentication Modules

- **`github.rs`** - GitHub CLI integration
- **`bitbucket_auth.rs`** - Bitbucket Cloud OAuth authentication
- **`bitbucket_data_center_auth.rs`** - Bitbucket Server authentication

### API Clients

- **`bitbucket_api.rs`** - Bitbucket Cloud API client
- **`bitbucket_data_center_api.rs`** - Bitbucket Server API client

## Design Principles

1. **Separation of Concerns**: Business logic (core/) is separate from CLI and external APIs
2. **Error Handling**: Centralized error types with context-rich messages
3. **Streaming Output**: Real-time output for long-running operations
4. **Trait-Based Design**: Provider trait allows easy addition of new git platforms
5. **Type Safety**: Leveraging Rust's type system for correctness

## Adding New Features

### Adding a New Command

1. Create a new file in `commands/`
2. Implement the command logic with a `run()` function
3. Add the command to `cli.rs`
4. Wire it up in `main.rs`

### Adding a New Provider

1. Create a new file in `providers/`
2. Implement the `Provider` trait
3. Update `create_provider()` in `providers/mod.rs`

### Error Handling

Use the centralized error types from `error.rs`:

```rust
use crate::error::{Error, Result};

// Return errors using the convenience methods
return Err(Error::git("Git operation failed"));
return Err(Error::config("Invalid configuration"));
```

## Testing

- Unit tests are embedded in modules using `#[cfg(test)]`
- Integration tests are in the `tests/` directory
- Run tests with `cargo test`

## Code Style

- Format code with `cargo fmt`
- Line width: 120 characters (configured in rustfmt.toml)
- Follow Rust naming conventions
- Add documentation comments for public APIs