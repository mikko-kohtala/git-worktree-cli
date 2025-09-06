# CLAUDE.md

Concise guidance for AI/code assistants working in this repository.

## Project

- Single Rust binary providing an ergonomic wrapper around `git worktree`
- Config file: `git-worktree-config.jsonc` (JSON with comments)
- Supported providers for PR info: GitHub, Bitbucket Cloud, Bitbucket Data Center

## Build & Test

- Build (debug): `cargo build`
- Build (release): `cargo build --release`
- Run: `cargo run -- <command>`
- Tests: `cargo test` (unit + integration)

## CLI

- `gwt init <repo-url> [--provider <github|bitbucket-cloud|bitbucket-data-center>]`
- `gwt add <branch>`
- `gwt list` (shows PRs, requires provider auth)
- `gwt remove [branch] [--force]`
- `gwt auth <github|bitbucket-cloud|bitbucket-data-center> [setup|test]`
- `gwt completions [install|generate <shell>]`

## Completions

- Completions are generated at build time by `build.rs` and embedded
- Install for detected shell: `gwt completions install`
- Generate to stdout: `gwt completions generate <shell>`

## Versioning & Metadata

- Bump `version` in `Cargo.toml` for changes
  - Patch: bug fixes, small improvements
  - Minor: new features (backward compatible)
  - Major: breaking changes
- Ensure `Cargo.toml` keeps `license` and `readme` set

## Style & Conventions

- Rust 2021; format with `cargo fmt` (see `rustfmt.toml`)
- Error handling via `thiserror`-based custom errors in `src/error.rs`
- Prefer small, focused modules under `src/` and `src/commands/`
- Avoid adding Node/TypeScript artifacts; this is a Rust-only codebase

## Testing Notes

- Integration tests live in `tests/` and should not depend on network access
- Prefer fast, deterministic tests; use temporary directories where needed

## Local Dev Tips

- To try features quickly without installing: `cargo run -- <subcommand>`
- For shell completions during dev: `gwt completions install`

