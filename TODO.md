# Git Worktree CLI (gwt) TODO

This file tracks pending tasks and future enhancements for the project.

## ✅ Recently Completed

### Bug Fixes
- [x] **Table formatting** - Fixed table right margin alignment issues with colored text (Dec 2024)
- [x] **PR status styling** - Fixed uppercase PR status, now shows lowercase with proper colors (Dec 2024)

### Features  
- [x] **GitHub PR Integration** - Show GitHub PR status and links in `gwt list` with colors (Dec 2024)
- [x] **Multi-Provider Support** - Added Bitbucket Cloud and Bitbucket Data Center support (Dec 2024)
- [x] **Authentication System** - Comprehensive auth management with `gwt auth` commands (Dec 2024)
- [x] **Secure Credential Storage** - System keyring integration for Bitbucket authentication (Dec 2024)
- [x] **Enhanced PR Display** - Show PRs without local worktrees, better formatting (Dec 2024)

## 🎯 High Priority

### Distribution & Release
- [ ] **GitHub Releases** - Automated binary releases with CI/CD
- [ ] **Install script** - curl-able install.sh for easy setup like other Rust tools
- [ ] **Homebrew formula** - Easy macOS installation with auto-completions
- [ ] **Cargo crates.io** - Publish to Rust package registry
- [ ] **Documentation updates** - Update all docs to reflect multi-provider support

## 🔧 Medium Priority  

### Core Features
- [x] **`gwt cd`** - Quick navigation between worktrees (✅ **Already implemented**)
- [ ] **`gwt doctor`** - Health check command to diagnose issues
- [ ] **Cleanup command** - Remove stale worktrees in bulk
- [ ] **Clone existing worktrees** - Support cloning projects with existing worktrees
- [ ] **Provider auto-detection** - Better URL parsing and provider detection
- [ ] **Credential management UI** - Better UX for managing stored credentials

### Shell Integration
- [ ] **Directory change on `gwt add`** - Auto-navigate to new worktree
  - [ ] Add `--print-path` flag for shell function integration
  - [ ] Create shell wrapper functions for auto-cd behavior
  - [ ] Update completion scripts to include wrapper

### Quality of Life
- [ ] **Command flags** - Add useful flags to existing commands:
  - [ ] `gwt init --provider <provider>` - (✅ **Already implemented**)
  - [ ] `gwt add --from <branch>` - Create from specific branch instead of main
  - [ ] `gwt add --no-hooks` - Skip hook execution
  - [x] `gwt remove --force` - Skip confirmation prompts (✅ **Already implemented**)
  - [ ] `gwt remove --keep-branch` - Preserve branch even for feature branches
- [ ] **Debug logging** - Add `--debug` parameter for troubleshooting:
  - [ ] API request/response logging for Bitbucket Data Center
  - [ ] Detailed error messages with context
  - [ ] Git command execution tracing
  - [ ] Configuration loading and validation steps
  - [ ] Hook execution debugging
- [ ] **Better error messages** - Actionable suggestions for common issues
- [ ] **Progress indicators** - For long-running operations
- [ ] **Colored output** - Better visual hierarchy in output
- [ ] **Config validation** - Validate hooks and settings on startup

## 🚀 Low Priority

### Advanced Features
- [x] **GitHub PR Integration** - Show GitHub PR status in list (✅ **Completed**)
- [x] **Bitbucket PR Integration** - Show Bitbucket PR status in list (✅ **Completed**)
  - [x] Bitbucket Cloud support with OAuth authentication
  - [x] Bitbucket Data Center support with personal access tokens
  - [x] Unified PR display across all providers  
- [ ] **Filtering** - Filter list by branch pattern, age, status
- [ ] **Metadata tracking** - Last commit date, creation time
- [ ] **Enhanced hooks** - Pre-hooks, conditional execution, error handling
- [ ] **Bulk operations** - Remove multiple worktrees with pattern matching

### Platform Support
- [ ] **AUR package** - Arch Linux support
- [ ] **Windows testing** - Verify Windows compatibility
- [ ] **Package managers** - Support for more Linux package managers

## 🐛 Known Issues

- [ ] Completion parsing assumes specific table format
- [ ] No Windows support tested
- [ ] Limited error recovery in hook execution
- [ ] Build warnings from completion generation (cosmetic)
- [ ] Provider detection could be more robust for edge cases
- [ ] Error messages for auth failures could be more helpful

## 💡 Ideas for Future Consideration

- [ ] **Integration with IDEs** - VS Code extension for worktree management
- [ ] **Git hooks integration** - Automatic setup of git hooks in worktrees
- [ ] **Template system** - Predefined project templates for different types
- [ ] **Remote worktrees** - Support for remote filesystem worktrees
- [ ] **Backup/restore** - Export/import worktree configurations