# Git Worktree CLI (gwt)

[![CI](https://github.com/mikko-kohtala/git-worktree-cli/actions/workflows/ci.yml/badge.svg)](https://github.com/mikko-kohtala/git-worktree-cli/actions/workflows/ci.yml)

🌿 **Enhanced Git Worktree Management with Rust** 🌿

Stop juggling multiple git clones or constantly switching branches. `git worktree` lets you check out multiple branches in the same repository at once. This tool makes managing them effortless.

## What are Git Worktrees?

Instead of cloning a repository multiple times or constantly running `git checkout`, worktrees let you have multiple working directories, each tied to a different branch, all within a single repo.

**Before:**
```bash
# Multiple clones or constant branch switching
git clone repo.git feature-work
git clone repo.git bugfix-work
# OR: git checkout feature && git checkout main ...
```

**After:**
```bash
# One repository, multiple working directories
my-project/
├── main/
├── feature-123/
└── bugfix-456/
```

## Installation

### Build from Source

1.  **Clone the repository:**
    ```bash
    git clone https://github.com/mikko-kohtala/git-worktree-cli.git ~/.gwt
    cd ~/.gwt
    ```

2.  **Build the binary:**
    ```bash
    cargo build --release
    ```

3.  **Add to your PATH:**
    ```bash
    # For macOS/Linux
    echo 'export PATH="$HOME/.gwt/target/release:$PATH"' >> ~/.zshrc # or ~/.bashrc
    source ~/.zshrc # or ~/.bashrc
    ```

4.  **Install Shell Completions (Recommended):**
    ```bash
    # Auto-install for your detected shell
    gwt completions install
    ```
    This command supports `bash`, `zsh`, `fish`, `powershell`, and `elvish`.

### Other Methods (Coming Soon)

-   **Direct Binary Download**: Pre-built binaries for macOS, Linux, and Windows.
-   **Cargo**: `cargo install gwt`

## Quick Start

1.  **Initialize a Project:**
    ```bash
    # Clone a repo and set it up for worktree management
    gwt init git@github.com:username/repo.git
    ```
    This clones the repository into a directory named after its default branch (e.g., `main/`) and creates a `git-worktree-config.jsonc` file.

2.  **Add a Worktree:**
    ```bash
    # Create a new worktree for a feature branch
    gwt add feature/user-auth
    ```

3.  **List Worktrees:**
    ```bash
    # See all your worktrees and their pull request status
    gwt list
    ```

4.  **Remove a Worktree:**
    ```bash
    # Clean up a completed feature
    gwt remove feature/user-auth
    ```

## Commands

| Command                       | Description                                       |
| ----------------------------- | ------------------------------------------------- |
| `gwt init <url>`              | Initialize a worktree project from a repository.  |
| `gwt list`                    | List all worktrees and their PR status.           |
| `gwt add <branch>`            | Create a new worktree for a branch.               |
| `gwt remove [branch]`         | Remove a worktree (defaults to current).          |
| `gwt auth <provider>`         | Manage authentication for providers (GitHub, etc).|
| `gwt completions install`     | Install shell completions.                        |

## Benefits

-   **🚀 No Context Switching**: Each branch has its own directory.
-   **🔄 Instant Branch Switching**: Just `cd` to the directory.
-   **🛡️ Safe Experimentation**: Isolated directories prevent conflicts.
-   **⚡ Parallel Development**: Work on multiple features at once.
-   **🧹 Easy Cleanup**: Remove completed work with a single command.
-   **🪝 Smart Automation**: Use hooks for setup and cleanup tasks.
-   **🔗 Multi-Provider Support**: Works with GitHub, Bitbucket Cloud, and Bitbucket Data Center.
-   **🔐 Secure Authentication**: Credentials stored securely in the system keyring.

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for details.

## License

This project is licensed under the MIT License.
