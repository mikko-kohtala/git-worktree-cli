use anyhow::{bail, Result};
use colored::Colorize;
use std::io::{self, Write};

use crate::{
    constants,
    core::project::{clean_branch_name, find_git_directory, find_project_root},
    git, hooks,
};

pub fn run(branch_name: Option<&str>) -> Result<()> {
    // Find a git directory to work with
    let git_dir = find_git_directory()?;

    // Get the list of worktrees
    let worktrees = git::list_worktrees(Some(&git_dir))?;

    if worktrees.is_empty() {
        println!("{}", "No worktrees found.".yellow());
        return Ok(());
    }

    // Find the worktree to remove
    let target_worktree = find_target_worktree(&worktrees, branch_name)?;

    // Check if this is the bare repository
    if target_worktree.bare {
        bail!("Cannot remove the main (bare) repository.");
    }

    let branch_display = get_branch_display(target_worktree);

    // Show what will be removed
    println!("{}", "About to remove worktree:".cyan().bold());
    println!("  {}: {}", "Path".dimmed(), target_worktree.path.display());
    println!("  {}: {}", "Branch".dimmed(), branch_display.green());

    // Check if we're currently in the worktree being removed
    let current_dir = std::env::current_dir()?;
    let will_remove_current = current_dir.starts_with(&target_worktree.path);

    if will_remove_current {
        println!(
            "\n{}",
            "⚠️  You are currently in this worktree. You will be moved to the project root after removal.".yellow()
        );
    }

    // Ask for confirmation
    print!("\n{}", "Are you sure you want to remove this worktree? (y/N): ".cyan());
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let confirmation = input.trim().to_lowercase();

    if confirmation != "y" && confirmation != "yes" {
        println!("{}", "Removal cancelled.".yellow());
        return Ok(());
    }

    // Find project root
    let project_root = find_project_root()?;

    // Find another worktree to run git commands from
    let main_branches = constants::PROTECTED_BRANCHES;
    let git_working_dir = worktrees
        .iter()
        .find(|wt| {
            // Try to find a main branch first
            wt.path != target_worktree.path
                && wt
                    .branch
                    .as_ref()
                    .map(|b| {
                        let clean_branch = if b.starts_with("refs/heads/") { &b[11..] } else { b };
                        main_branches.contains(&clean_branch)
                    })
                    .unwrap_or(false)
        })
        .or_else(|| {
            // If no main branch, use any other worktree
            worktrees.iter().find(|wt| wt.path != target_worktree.path)
        })
        .ok_or_else(|| anyhow::anyhow!("No other worktrees found to execute git command from."))?;

    // Remove the worktree
    println!("\n{}", "Removing worktree...".cyan());
    git::execute_streaming(
        &["worktree", "remove", target_worktree.path.to_str().unwrap(), "--force"],
        Some(&git_working_dir.path),
    )?;

    println!(
        "{}",
        format!("✓ Worktree removed: {}", target_worktree.path.display()).green()
    );

    // Delete the branch if it's not a main branch
    if !main_branches.contains(&branch_display) {
        // First try to delete the branch normally
        match git::execute_capture(&["branch", "-d", branch_display], Some(&git_working_dir.path)) {
            Ok(_) => {
                println!("{}", format!("✓ Branch deleted: {}", branch_display).green());
            }
            Err(e) => {
                // If normal deletion fails, check if it's because of unmerged changes
                if e.to_string().contains("not fully merged") {
                    println!(
                        "{}",
                        format!("⚠️  Branch '{}' has unmerged changes", branch_display).yellow()
                    );

                    // Ask for confirmation to force delete
                    print!("{}", "Force delete the branch? (y/N): ".cyan());
                    io::stdout().flush()?;

                    let mut input = String::new();
                    io::stdin().read_line(&mut input)?;
                    let force_delete = input.trim().to_lowercase();

                    if force_delete == "y" || force_delete == "yes" {
                        match git::execute_streaming(&["branch", "-D", branch_display], Some(&git_working_dir.path)) {
                            Ok(_) => {
                                println!("{}", format!("✓ Branch force deleted: {}", branch_display).green());
                            }
                            Err(e) => {
                                println!(
                                    "{}",
                                    format!("❌ Failed to delete branch '{}': {}", branch_display, e).red()
                                );
                            }
                        }
                    } else {
                        println!(
                            "{}",
                            format!("⚠️  Branch '{}' was not deleted", branch_display).yellow()
                        );
                    }
                } else {
                    // Some other error occurred
                    println!(
                        "{}",
                        format!("❌ Failed to delete branch '{}': {}", branch_display, e).red()
                    );
                }
            }
        }
    } else {
        println!(
            "{}",
            format!("✓ Branch: {} (preserved - main branch)", branch_display).green()
        );
    }

    // If we removed the current worktree, change to project root before executing hooks
    if will_remove_current {
        std::env::set_current_dir(&project_root)?;
    }

    // Execute post-remove hooks
    hooks::execute_hooks(
        "postRemove",
        &project_root,
        &[
            ("branchName", branch_display),
            ("worktreePath", target_worktree.path.to_str().unwrap()),
        ],
    )?;

    // If we removed the current worktree, show message about moving to project root
    if will_remove_current {
        println!(
            "{}",
            format!("✓ Please navigate to project root: {}", project_root.display()).green()
        );
    }

    Ok(())
}

fn find_target_worktree<'a>(worktrees: &'a [git::Worktree], branch_name: Option<&str>) -> Result<&'a git::Worktree> {
    match branch_name {
        None => find_current_worktree(worktrees),
        Some(target_branch) => find_worktree_by_branch(worktrees, target_branch),
    }
}

fn find_current_worktree(worktrees: &[git::Worktree]) -> Result<&git::Worktree> {
    let current_dir = std::env::current_dir()?;
    worktrees
        .iter()
        .find(|wt| current_dir.starts_with(&wt.path))
        .ok_or_else(|| anyhow::anyhow!("Not in a git worktree. Please specify a branch to remove."))
}

fn find_worktree_by_branch<'a>(worktrees: &'a [git::Worktree], target_branch: &str) -> Result<&'a git::Worktree> {
    // First try to find by branch name
    if let Some(worktree) = find_by_branch_name(worktrees, target_branch) {
        return Ok(worktree);
    }

    // Then try to find by path
    if let Some(worktree) = find_by_path_name(worktrees, target_branch) {
        return Ok(worktree);
    }

    // Not found, show available worktrees
    show_available_worktrees(worktrees);
    bail!("Worktree for '{}' not found", target_branch)
}

fn find_by_branch_name<'a>(worktrees: &'a [git::Worktree], target_branch: &str) -> Option<&'a git::Worktree> {
    worktrees.iter().find(|wt| {
        wt.branch
            .as_ref()
            .map(|b| clean_branch_name(b) == target_branch)
            .unwrap_or(false)
    })
}

fn find_by_path_name<'a>(worktrees: &'a [git::Worktree], target_branch: &str) -> Option<&'a git::Worktree> {
    worktrees.iter().find(|wt| {
        wt.path
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name == target_branch)
            .unwrap_or(false)
    })
}

fn show_available_worktrees(worktrees: &[git::Worktree]) {
    println!("{}", "Error: Worktree not found.".red());
    println!("\n{}", "Available worktrees:".yellow());

    for worktree in worktrees {
        let branch_display = get_branch_display(worktree);
        println!(
            "  {} -> {}",
            branch_display.green(),
            worktree.path.display().to_string().dimmed()
        );
    }
}

fn get_branch_display(worktree: &git::Worktree) -> &str {
    worktree
        .branch
        .as_ref()
        .map(|b| clean_branch_name(b))
        .unwrap_or_else(|| {
            if worktree.bare {
                "(bare)"
            } else {
                &worktree.head[..8.min(worktree.head.len())]
            }
        })
}
