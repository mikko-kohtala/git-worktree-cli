# Development Guide

`gwt`: a Rust CLI for managing git worktrees, with PR integrations for GitHub, Bitbucket and Azure DevOps. Usage and hooks: README.md.

## Project workflow

Before changing this repository, read and follow
`.agents/skills/project-workflow/SKILL.md` from the repository root.

Repository-specific instructions and explicit user directions take precedence.

## Validation
Validate all work with `make check` (fmt, clippy, tests) before calling it done. `make format` applies formatting.

## Version Management
When making code changes, increment the version in Cargo.toml:
- Patch version (x.x.N) for bug fixes
- Minor version (x.N.x) for new features
- Major version (N.x.x) for breaking changes

After the PR is squash-merged, tag the resulting commit on main, not a branch commit: `git fetch origin && git tag v<version> origin/main` (e.g., `v0.9.0`)
