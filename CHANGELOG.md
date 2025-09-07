# Changelog

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