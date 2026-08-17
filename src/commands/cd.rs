use std::io::IsTerminal;
use std::path::PathBuf;

use crate::config::GitWorktreeConfig;
use crate::core::project::find_project_root;
use crate::error::{Error, Result};

pub fn run(branch_name: Option<&str>) -> Result<()> {
    let worktrees_path = resolve_worktrees_path()?;

    let target = match branch_name {
        Some(branch) => {
            let path = worktrees_path.join(branch);
            if !path.exists() {
                return Err(Error::msg(format!(
                    "No worktree found for branch '{}' at {}\nRun 'gwt add {}' to create one.",
                    branch,
                    path.display(),
                    branch
                )));
            }
            path
        }
        None => {
            if !worktrees_path.exists() {
                return Err(Error::msg(format!(
                    "Worktrees folder does not exist yet: {}\nRun 'gwt add <branch>' to create the first worktree.",
                    worktrees_path.display()
                )));
            }
            worktrees_path
        }
    };

    // The shell wrapper installed by 'gwt completions install' captures stdout
    // and runs cd on it, so print the bare path with no decoration
    println!("{}", target.display());

    // Interactive invocation without the wrapper: the printed path alone can't
    // change the shell's directory, so explain how to make it work
    if std::io::stdout().is_terminal() {
        eprintln!();
        eprintln!("Note: a command cannot change its parent shell's directory.");
        eprintln!("Run 'gwt completions install' to install the shell wrapper that makes");
        eprintln!("'gwt cd' change directory, or use: cd \"$(gwt cd)\"");
    }

    Ok(())
}

/// Resolve the worktrees folder path the same way 'gwt add' does
fn resolve_worktrees_path() -> Result<PathBuf> {
    let project_root = find_project_root()?;

    if let Some((_config_path, config)) = GitWorktreeConfig::find_config()? {
        if let Some(path) = config.get_worktrees_path() {
            return Ok(path);
        }
    }

    Ok(GitWorktreeConfig::derive_worktrees_path(&project_root))
}
