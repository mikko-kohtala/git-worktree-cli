# Git Worktree CLI – Source Layout

Overview of the Rust code for the `gwt` binary.

## Structure

- `main.rs` – CLI entrypoint; routes to subcommands
- `cli.rs` – clap definitions for arguments and subcommands
- `commands/` – implementation of subcommands
  - `init.rs`, `add.rs`, `list.rs`, `remove.rs`, `auth.rs`
- `git.rs` – git command execution with real-time streaming
- `hooks.rs` – postAdd/postRemove hook execution
- `config.rs` – JSONC config (`git-worktree-config.jsonc`) handling
- `completions.rs` – embedded completion logic + install helpers
- `error.rs` – error types using `thiserror`
- `core/` – project discovery and utilities
  - `project.rs`, `utils.rs`
- Provider/PR integrations
  - `github.rs`, `bitbucket_api.rs`, `bitbucket_data_center_api.rs`
  - `bitbucket_auth.rs`, `bitbucket_data_center_auth.rs`

## Guidelines

- Keep subcommands small and focused; prefer helpers in modules
- Prefer descriptive errors; bubble up `Result` from helpers
- Format with `cargo fmt` (see `rustfmt.toml`)
- Tests: unit alongside modules; integration in `tests/`

## Adding a Command

1. Add clap enum variant and args in `cli.rs`
2. Implement `run(...)` in `src/commands/<name>.rs`
3. Wire in `match` in `main.rs`
