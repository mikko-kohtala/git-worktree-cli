# CLAUDE.md

This file provides guidance to ([Claude Code](https://www.anthropic.com/claude-code)) when working with code in this repository.

## Commands

### Development
- **Build release binary**: `cargo build --release` - Creates optimized binary at `target/release/gwt`
- **Build debug binary**: `cargo build` - Creates debug binary for development
- **Run tests**: `cargo test` - Comprehensive unit and integration tests
- **Type checking**: `cargo check` - Fast compilation check without building binary
- **Run with cargo**: `cargo run -- <command>` - Run directly with cargo for development
- **Format code**: `cargo fmt` - Format all Rust code according to rustfmt.toml settings

### Testing and Local Development
When Claude needs to test functionality locally:
- Use `cargo build` for debug builds or `cargo build --release` for optimized builds
- Run the binary directly: `./target/release/gwt <command>` or `./target/debug/gwt <command>`
- Create test repositories in the `test-temp/` directory which is git-ignored
- Use `gwt init` to create proper worktree projects for testing
- Clean up test directories after testing with `rm -rf test-temp`
- Example test workflow:
  ```bash
  mkdir test-temp && cd test-temp
  ../target/release/gwt init https://github.com/octocat/Hello-World.git
  ../target/release/gwt list
  cd .. && rm -rf test-temp
  ```

## Version Management

**IMPORTANT**: Whenever making code changes, always increment the version in Cargo.toml:
- Patch version (x.x.N) for bug fixes and minor improvements
- Minor version (x.N.x) for new features
- Major version (N.x.x) for breaking changes

### Git Worktree Tool (gwt)
- **Initialize project**: `gwt init <repository-url>` - Initialize a new worktree project from a repository URL
- **Add worktree**: `gwt add <branch-name>` - Create new worktrees from branch names  
- **List worktrees**: `gwt list` - List all worktrees in a formatted table
- **Remove worktree**: `gwt remove [branch-name]` - Remove existing worktrees with confirmation
- **Check completions**: `gwt completions` - Check if shell completions are installed
- **Install completions**: `gwt completions install [shell]` - Automatically install completions (defaults to detected shell)
- **Generate completions**: `gwt completions generate <shell>` - Generate shell completion scripts to stdout
- **Tab completion**: Dynamic branch name completion for add/remove commands (see completions/SETUP.md)

### Legacy TypeScript Version
The original TypeScript implementation has been moved to `typescript-version/` directory for reference.

## Architecture

This project is a **single Rust binary** that provides git worktree management functionality. The architecture consists of:

1. **Rust Binary** (`src/`): Core functionality written in Rust
   - `main.rs`: CLI entry point using clap for argument parsing
   - `cli.rs`: Separated CLI structure for build-time completion generation
   - `commands/`: Individual command implementations (init, add, list, remove)
   - `completions.rs`: Embedded shell completions with auto-install functionality
   - `config.rs`: YAML configuration file handling using serde
   - `git.rs`: Git operations with native process execution and streaming output
   - `hooks.rs`: Hook execution system with real-time output streaming
   - `utils.rs`: Shared utility functions
   - `build.rs`: Build script that generates shell completions at compile time

2. **Key Dependencies**:
   - `clap`: Command-line argument parsing with derive macros
   - `clap_complete`: Shell completion generation (build dependency)
   - `serde` + `serde_yaml`: Configuration file serialization/deserialization
   - `anyhow`: Error handling and context
   - `colored`: Terminal output colorization
   - `chrono`: Date/time handling for config timestamps
   - `tabled`: Table formatting for list output

3. **Test Infrastructure**:
   - `tests/`: Integration tests using `assert_cmd` and `tempfile`
   - Unit tests embedded in source modules
   - Real git repository testing with streaming output verification

## Hooks System

The project includes a flexible hooks system that allows users to run custom commands after various worktree operations. Hooks are defined in the `git-worktree-config.jsonc` file and support variable substitution.

### Available Hooks

- **`postAdd`**: Executed after `gwt add` creates a new worktree
- **`postRemove`**: Executed after `gwt remove` removes a worktree

### Variable Substitution

Hooks support the following variables:
- `${branchName}`: The name of the branch
- `${worktreePath}`: The full path to the worktree directory

### Default Configuration

When `gwt init` creates a new project, it generates a `git-worktree-config.jsonc` with empty hooks arrays by default:

```jsonc
{
  "hooks": {
    "postAdd": [],
    "postRemove": []
  }
}
```

### Active Configuration Example

To enable hooks, add commands to the arrays:

```jsonc
{
  "hooks": {
    "postAdd": [
      "echo 'Created worktree for ${branchName} at ${worktreePath}'",
      "npm install",
      "npm run init"
    ],
    "postRemove": [
      "echo 'Removed worktree for branch ${branchName}'"
    ]
  }
}
```

### Hook Behavior

- **Real-time output**: Commands stream output live using `execSync` with `stdio: 'inherit'`
- **Execution context**: 
  - `postAdd`: Execute in the worktree directory
  - `postRemove`: Execute in the project root directory
- **Comment handling**: Use JSONC comments (`//` or `/* */`) outside of strings
- **Error handling**: Failed hooks show warnings but don't stop execution
- **Sequential execution**: Hooks run in the order they're defined
- **Environment**: Hooks inherit the current environment with `FORCE_COLOR: '1'` for colored output

### Common Hook Patterns

```jsonc
{
  "hooks": {
    "postAdd": [
      "npm install",                    // Install dependencies
      "npm run build",                  // Build project
      "code ."                          // Open in VS Code
    ],
    "postRemove": [
      "echo 'Cleaned up ${branchName}'"  // Log cleanup
    ]
  }
}
```

### Troubleshooting Hooks

- **Hook not executing**: Check that the hook command is in the array
- **No output visible**: Hooks use real-time streaming - output should appear immediately
- **Hook fails**: Check the command syntax and file permissions
- **Variable not substituted**: Ensure correct syntax: `${branchName}` or `${worktreePath}`

## Current Implementation Status

### ✅ Implemented Features (Rust)

1. **`gwt init`**: Initialize worktrees from repository URLs ✅
   - ✅ Clones repository with **real-time streaming output** (major improvement!)
   - ✅ Detects the default branch name
   - ✅ Renames the cloned directory to match the branch name
   - ✅ Creates `git-worktree-config.jsonc` with repository metadata

2. **`gwt list`**: Display all worktrees in a formatted table ✅
   - ✅ Finds worktrees in project directory
   - ✅ Displays in clean table format with sharp borders
   - ✅ Shows path, branch name, and HEAD commit
   - ✅ Works from any subdirectory within project

3. **`gwt completions`**: Enhanced shell completions support ✅
   - ✅ Check if completions are installed
   - ✅ Auto-install completions with `gwt completions install [shell]`
   - ✅ Generate completions for any shell with `gwt completions generate <shell>`
   - ✅ Smart detection of user's shell
   - ✅ Completions embedded in binary at compile time
   - ✅ Support for bash, zsh, fish, powershell, and elvish
   - ✅ Automatic path detection for each shell's completion directory
   - ✅ Branch name completion for add/remove commands

4. **`gwt add`**: Create new worktrees from branch names ✅
   - ✅ Create worktree from existing local branch
   - ✅ Create worktree from existing remote branch
   - ✅ Create new branch from main/master branch
   - ✅ Smart branch detection and handling
   - ✅ Execute post-add hooks with variable substitution
   - ✅ Real-time streaming output for git operations
   - ✅ Project root detection and validation

5. **`gwt remove`**: Remove worktrees with safety checks ✅
   - ✅ Remove worktree by branch name or current worktree
   - ✅ Interactive confirmation prompts
   - ✅ Safety checks (prevents removing bare repository)
   - ✅ Automatic branch deletion for feature branches
   - ✅ Preserves main branches (main/master/dev/develop)
   - ✅ Handles current directory when removing current worktree
   - ✅ Execute post-remove hooks with variable substitution
   - ✅ Comprehensive error handling and user feedback


### 🎯 Major Improvements Achieved

- **✅ Real-time streaming output**: Git commands show progress in real-time using Rust's native `Command` with `Stdio::inherit()`
- **✅ Single binary distribution**: No Node.js runtime or shell wrapper functions needed
- **✅ Embedded completions**: Shell completions are generated at compile time and embedded in the binary
- **✅ Enhanced shell completions**: Auto-install support for all major shells
- **✅ Better error handling**: Rust's `Result` type provides robust error propagation
- **✅ Faster execution**: Compiled binary vs interpreted TypeScript
- **✅ Cross-platform compatibility**: Easy to build for different OS/architectures
- **✅ Sharp table formatting**: Professional table output using Unicode box-drawing characters
- **✅ Smart shell detection**: Automatically detects user's shell for completion installation

## Test Suite

The project includes comprehensive testing in Rust:
- **Integration tests** (`tests/`): Uses `assert_cmd` and `tempfile` for real command testing
- **Unit tests**: Embedded in source modules with `#[cfg(test)]`
- **Real streaming verification**: Tests actually verify git clone output streaming
- **6 integration tests**: Covering init command, help, version, error handling
- **4 unit tests**: Testing config module functionality
- **Fast execution**: All tests run in ~6 seconds vs 15+ seconds for TypeScript version

## Project Management

Project tasks and TODOs are tracked in `TODO.md` for persistence across Claude Code sessions.

## Code Style

- **Language**: Rust 2021 edition with standard formatting
- **Formatting**: `cargo fmt` with rustfmt.toml configuration (120 character line width)
- **Error Handling**: `anyhow::Result` for error propagation with context
- **CLI Framework**: `clap` with derive macros for argument parsing
- **Serialization**: `serde` with `serde_yaml` for configuration files
- **Testing**: Integration tests with `assert_cmd`, unit tests with `#[cfg(test)]`
- **File Organization**: Each command has its own module, shared utilities in dedicated modules

## Important Technical Notes

### Real-time Streaming Output with Rust

The Rust implementation naturally handles streaming output using native `std::process::Command`:

```rust
// ✅ Real-time streaming with Rust - simple and effective!
use std::process::{Command, Stdio};

Command::new("git")
    .args(["clone", repo_url, repo_name])
    .stdout(Stdio::inherit())
    .stderr(Stdio::inherit())
    .status()?;
```

**Key advantages over the TypeScript/zx approach:**
- **No buffering issues**: Output streams directly to terminal
- **Native support**: No workarounds needed for real-time output
- **Better performance**: Direct process spawning without Node.js overhead
- **Cross-platform**: Works consistently across different operating systems

### Hook Execution with Streaming

Hooks also benefit from native process execution:

```rust
// Hooks execute with real-time output
Command::new("sh")
    .arg("-c")
    .arg(command)
    .current_dir(working_directory)
    .stdout(Stdio::inherit())
    .stderr(Stdio::inherit())
    .env("FORCE_COLOR", "1")
    .status()?;
```

This eliminates the complex issues that existed with the TypeScript implementation where streaming output required workarounds with `execSync` and `stdio: 'inherit'`.

### Shell Completions Architecture

The Rust implementation uses a build-time completion generation approach:

1. **Build Script (`build.rs`)**:
   - Generates completions for all supported shells at compile time
   - Uses the `cli.rs` module to access the CLI structure without dependencies
   - Stores generated completions in the `OUT_DIR` for embedding

2. **Embedded Completions (`completions.rs`)**:
   - Uses `include_str!` to embed completion files directly in the binary
   - Provides shell detection and automatic installation paths
   - Handles shell-specific installation requirements

3. **Benefits**:
   - **No external files needed**: Completions are part of the binary
   - **Always in sync**: Completions automatically match the CLI structure
   - **Easy distribution**: Single binary includes everything
   - **Multi-shell support**: All shells supported without extra downloads

Example of embedded completion:
```rust
const BASH_COMPLETION: &str = include_str!(concat!(env!("OUT_DIR"), "/completions/gwt.bash"));
```

### Secure Credential Management

The application uses the system keyring for secure credential storage:

```rust
use keyring::Entry;

// Store credentials securely
let entry = Entry::new("gwt", &username)?;
entry.set_password(&token)?;

// Retrieve credentials
let password = entry.get_password()?;
```

**Security features:**
- **No plaintext storage**: All credentials encrypted by OS
- **Per-user isolation**: Credentials tied to user account
- **Cross-platform**: Works on Windows, macOS, and Linux
- **Automatic cleanup**: Credentials can be easily removed

## Provider URLs and Authentication

### GitHub
- **URL patterns**: `github.com`, `git@github.com:`
- **Authentication**: GitHub CLI (`gh auth login`)
- **API**: Uses `gh pr list` command for PR information

### Bitbucket Cloud
- **URL patterns**: `bitbucket.org`
- **Authentication**: OAuth app passwords via interactive setup
- **API**: Direct REST API calls to `api.bitbucket.org`
- **Credentials**: Stored in system keyring as `gwt-bitbucket-cloud`

### Bitbucket Data Center
- **URL patterns**: Custom domains (detected by elimination)
- **Authentication**: Personal access tokens via interactive setup
- **API**: REST API calls to custom Bitbucket instance
- **Credentials**: Stored in system keyring as `gwt-bitbucket-datacenter`
