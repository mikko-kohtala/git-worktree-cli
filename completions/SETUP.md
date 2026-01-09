# Setting up GWT Tab Completions

## Automatic Installation (Recommended)

The easiest way to install completions is to use the built-in install command:

```bash
# Check if completions are installed
gwt completions

# Install completions for your shell (auto-detects shell)
gwt completions install

# Install for a specific shell
gwt completions install zsh
```

The installer will:
- Detect your shell automatically
- Find the completion files
- Update your shell configuration (~/.zshrc for zsh)
- Remove any duplicate entries
- Provide instructions to activate completions

## Manual Installation

### For Zsh

If you prefer manual installation, add the following to your `~/.zshrc`:

```bash
# GWT completions
if [[ -f ~/.config/git-worktree-cli/completions/gwt-completions.zsh ]]; then
    fpath=(~/.config/git-worktree-cli/completions $fpath)
    autoload -Uz compinit && compinit
    source ~/.config/git-worktree-cli/completions/gwt-completions.zsh
fi
```

## For Bash

Generate bash completions:
```bash
gwt completions bash > ~/.config/git-worktree-cli/completions/gwt-completions.bash
```

Add to your `~/.bashrc`:
```bash
# GWT completions
if [[ -f ~/.config/git-worktree-cli/completions/gwt-completions.bash ]]; then
    source ~/.config/git-worktree-cli/completions/gwt-completions.bash
fi
```

## Features

The completions provide:

- **Command completion**: Tab after `gwt` shows all available commands
- **Branch name completion**: 
  - `gwt add <TAB>` shows available remote branches
  - `gwt remove <TAB>` shows existing worktrees that can be removed
- **URL completion**: `gwt init <TAB>` provides URL completion
- **Shell completion**: `gwt completions <TAB>` shows available shells (bash, zsh, fish)

## Testing

After setting up, test with:
```bash
gwt <TAB>              # Should show: init, add, list, remove, completions
gwt add <TAB>          # Should show available branches
gwt remove <TAB>       # Should show existing worktrees
```