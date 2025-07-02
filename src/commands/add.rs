use colored::Colorize;
use std::path::{Path, PathBuf};

use crate::config::GitWorktreeConfig;
use crate::core::project::{find_existing_worktree, find_project_root};
use crate::error::{Error, Result};
use crate::git;
use crate::hooks;

pub fn run(branch_name: &str) -> Result<()> {
    if branch_name.is_empty() {
        return Err(Error::msg("Error: Branch name is required\nUsage: gwt add <branch-name>"));
    }

    // Determine git root and target path
    let (git_working_dir, target_path, project_root) = determine_paths(branch_name)?;

    println!(
        "{}",
        format!("Preparing worktree (new branch '{}')", branch_name).cyan()
    );

    // Get main branch from config
    let main_branch = get_main_branch(&project_root)?;

    // Check if branch exists locally or remotely
    let (local_exists, remote_exists) = git::branch_exists(&git_working_dir, branch_name)?;

    // Create worktree based on branch existence
    if local_exists {
        println!(
            "{}",
            format!(
                "Branch '{}' exists locally, checking out existing branch...",
                branch_name
            )
            .yellow()
        );
        git::execute_streaming(
            &["worktree", "add", target_path.to_str().unwrap(), branch_name],
            Some(&git_working_dir),
        )?;
    } else if remote_exists {
        println!(
            "{}",
            format!(
                "Branch '{}' exists remotely, checking out remote branch...",
                branch_name
            )
            .yellow()
        );
        git::execute_streaming(
            &[
                "worktree",
                "add",
                target_path.to_str().unwrap(),
                "-b",
                branch_name,
                &format!("origin/{}", branch_name),
            ],
            Some(&git_working_dir),
        )?;
    } else {
        println!(
            "{}",
            format!("Creating new branch '{}' from 'origin/{}'...", branch_name, main_branch).cyan()
        );
        git::execute_streaming(
            &[
                "worktree",
                "add",
                "--no-track",
                target_path.to_str().unwrap(),
                "-b",
                branch_name,
                &format!("origin/{}", main_branch),
            ],
            Some(&git_working_dir),
        )?;
    }

    // Success messages
    println!(
        "{}",
        format!("✓ Worktree created at: {}", target_path.display()).green()
    );
    println!("{}", format!("✓ Branch: {}", branch_name).green());

    // Execute post-add hooks
    hooks::execute_hooks(
        "postAdd",
        &target_path,
        &[
            ("branchName", branch_name),
            ("worktreePath", target_path.to_str().unwrap()),
        ],
    )?;

    Ok(())
}

fn determine_paths(branch_name: &str) -> Result<(PathBuf, PathBuf, PathBuf)> {
    let project_root = find_project_root()?;
    let target_path = project_root.join(branch_name);
    let git_working_dir = find_existing_worktree(&project_root)?;

    Ok((git_working_dir, target_path, project_root))
}



fn get_main_branch(project_root: &Path) -> Result<String> {
    let config_path = project_root.join("git-worktree-config.yaml");
    if config_path.exists() {
        let config = GitWorktreeConfig::load(&config_path)?;
        Ok(config.main_branch)
    } else {
        // Fallback to detecting from git if no config
        if let Some(git_root) = git::get_git_root()? {
            Ok(git::get_default_branch(&git_root)?)
        } else {
            Ok("main".to_string())
        }
    }
}
