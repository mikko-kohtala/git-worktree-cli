# Contributing to Git Worktree CLI (gwt)

This document provides guidance for developers contributing to this project.

## Development Workflow

### Prerequisites
- Rust 1.70+
- Git 2.5+

### Common Commands

- **Build debug binary**: `cargo build`
  - Creates a debug binary at `target/debug/gwt`.
- **Build release binary**: `cargo build --release`
  - Creates an optimized binary at `target/release/gwt`.
- **Run tests**: `cargo test`
  - Executes all unit and integration tests.
- **Check code**: `cargo check`
  - Quickly checks the code for errors without producing an executable.
- **Format code**: `cargo fmt`
  - Formats the code according to the `rustfmt.toml` configuration.
- **Run locally**: `cargo run -- <command>`
  - Example: `cargo run -- list`

### Testing Locally

To test changes locally, you can create temporary repositories.

```bash
# Create a temporary directory for testing
mkdir test-temp && cd test-temp

# Use the locally built binary to initialize a project
../target/release/gwt init https://github.com/octocat/Hello-World.git

# Run other commands
../target/release/gwt list

# Clean up
cd .. && rm -rf test-temp
```

## Version Management

When submitting changes, please consider the scope of your contribution and update the version in `Cargo.toml` according to [Semantic Versioning](https://semver.org/):

- **Patch** (`x.x.N`): For bug fixes and minor, backward-compatible changes.
- **Minor** (`x.N.x`): For new features that are backward-compatible.
- **Major** (`N.x.x`): For breaking changes that are not backward-compatible.

## Code Style

- **Formatting**: All code should be formatted with `cargo fmt`.
- **Error Handling**: Use `anyhow::Result` for error propagation to provide context.
- **CLI**: The command-line interface is built with `clap`.
- **Modules**: Each command is implemented in its own module within the `src/commands/` directory.

## Submitting Changes

1. Fork the repository.
2. Create a new branch for your feature or bug fix.
3. Make your changes and commit them with clear, descriptive messages.
4. Ensure all tests pass (`cargo test`).
5. Push your branch and open a pull request.
