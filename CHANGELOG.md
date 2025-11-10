# Changelog

## [0.5.0] - 2025-01-10

### Added
- Configuration file discovery now checks `./main/` subdirectory for `git-worktree-config.jsonc`
- Project root detection now recognizes `./main/` subdirectory as a valid location for config

### Changed
- Makefile `install` target now uses `cargo install --path .` instead of manual binary copying
- `git-worktree-config.jsonc` is no longer ignored by git, allowing it to be committed to repositories

## [0.4.1] - 2025-01-09

### Fixed
- Permission denied error on Linux when installing completions to system directories
- Completion path prioritization now prefers user-writable directories (`~/.local/share/bash-completion/completions`) over system directories (`/etc/bash_completion.d`)
- Improved writability check on Unix systems using actual write test instead of readonly flag

## [0.4.0] - 2025-01-07

### Added
- `--force` flag requirement for `gwt init` to prevent accidental directory overwrites
- Branch name sanitization for directory creation (slashes replaced with hyphens)
- Strongly-typed structs for GitHub API responses
- `cargo clippy` linting in CI workflow

### Fixed
- Critical bug: reference to temporary value in `list_helpers.rs`
- Clippy warning in `remove.rs`

### Changed
- **BREAKING**: `gwt init` now requires `--force` flag to overwrite existing directories
- CI workflow improvements: added `--locked` flag for reproducible builds