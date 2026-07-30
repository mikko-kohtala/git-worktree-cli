use colored::Colorize;
use dialoguer::{theme::ColorfulTheme, MultiSelect};
use std::io::{self, IsTerminal, Write};

use super::list_helpers::{colored_pr_status, PrContext, PullRequestInfo};
use crate::{
    constants,
    core::project::{
        clean_branch_name, find_git_directory, find_project_root, find_project_root_from, find_valid_git_directory,
        is_orphaned_worktree,
    },
    error::{Error, Result},
    git, hooks,
};

pub fn run(branch_name: Option<&str>, force: bool) -> Result<()> {
    // Check if we're trying to remove an orphaned worktree by directory name
    if let Some(branch) = branch_name {
        if let Ok(project_root) = find_project_root() {
            let potential_worktree_path = project_root.join(branch);
            if is_orphaned_worktree(&potential_worktree_path) {
                println!("{}", "⚠️  Detected orphaned worktree (stale git reference)".yellow());
                return remove_orphaned_worktree(&potential_worktree_path, branch, force);
            }
        }
    }

    // Find a git directory to work with
    let git_dir = find_git_directory()?;

    // Get the list of worktrees
    let worktrees = git::list_worktrees(Some(&git_dir))?;

    if worktrees.is_empty() {
        println!("{}", "No worktrees found.".yellow());
        return Ok(());
    }

    match branch_name {
        Some(branch) => {
            let target_worktree = find_worktree_by_branch(&worktrees, branch)?;
            remove_worktree(&worktrees, target_worktree, force, false)
        }
        None => run_interactive(&git_dir, worktrees, force),
    }
}

/// Show an interactive multi-select of worktrees and remove the chosen ones
fn run_interactive(git_dir: &std::path::Path, worktrees: Vec<git::Worktree>, force: bool) -> Result<()> {
    if !io::stdin().is_terminal() {
        return Err(Error::msg(
            "No branch specified and no terminal available for interactive selection. \
             Specify a branch name, e.g. 'gwt remove <branch>'.",
        ));
    }

    let removable: Vec<git::Worktree> = worktrees.into_iter().filter(|wt| !wt.bare).collect();

    if removable.is_empty() {
        println!("{}", "No removable worktrees found.".yellow());
        return Ok(());
    }

    let pr_infos = fetch_pr_statuses(&removable);

    let current_dir = std::env::current_dir()?;
    let items = build_picker_items(&removable, &pr_infos, &current_dir);

    let selection = MultiSelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Select worktrees to remove (space to toggle, enter to confirm)")
        .items(&items)
        .interact_opt()
        .map_err(|e| Error::msg(format!("Selection failed: {}", e)))?;

    let indices = match selection {
        Some(indices) if !indices.is_empty() => indices,
        Some(_) => {
            println!("{}", "No worktrees selected.".yellow());
            return Ok(());
        }
        None => {
            println!("{}", "Removal cancelled.".yellow());
            return Ok(());
        }
    };

    println!("\n{}", format!("Selected {} worktree(s):", indices.len()).cyan().bold());
    for &i in &indices {
        let worktree = &removable[i];
        println!(
            "  {} -> {}",
            get_branch_display(worktree).green(),
            worktree.path.display().to_string().dimmed()
        );
        if let Some(pr) = &pr_infos[i] {
            println!("    {} ({})", pr.url.blue().underline(), colored_pr_status(&pr.status));
            if !pr.title.is_empty() {
                println!("    {}", pr.title.dimmed());
            }
        }
    }

    if !force {
        print!(
            "\n{}",
            "Are you sure you want to remove the selected worktrees? (y/N): ".cyan()
        );
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let confirmation = input.trim().to_lowercase();

        if confirmation != "y" && confirmation != "yes" {
            println!("{}", "Removal cancelled.".yellow());
            return Ok(());
        }
    }

    let selected_paths: Vec<std::path::PathBuf> = indices.iter().map(|&i| removable[i].path.clone()).collect();

    for path in selected_paths {
        // Refresh the worktree list each round so removed entries are not reused
        let worktrees = git::list_worktrees(Some(git_dir))?;
        let Some(target_worktree) = worktrees.iter().find(|wt| wt.path == path) else {
            continue;
        };
        remove_worktree(&worktrees, target_worktree, force, true)?;
    }

    Ok(())
}

/// Build colored, column-aligned lines for the interactive picker.
/// Padding is computed on the plain text before colors are applied,
/// since ANSI escape codes would break format-width alignment.
fn build_picker_items(
    worktrees: &[git::Worktree],
    pr_infos: &[Option<PullRequestInfo>],
    current_dir: &std::path::Path,
) -> Vec<String> {
    const CURRENT_LABEL: &str = " (current)";

    let branch_width = worktrees
        .iter()
        .map(|wt| {
            let current_len = if current_dir.starts_with(&wt.path) {
                CURRENT_LABEL.len()
            } else {
                0
            };
            get_branch_display(wt).len() + current_len
        })
        .max()
        .unwrap_or(0);

    // Status column is only shown when at least one worktree has PR info
    let status_width = pr_infos
        .iter()
        .map(|pr| pr.as_ref().map(|p| p.status.len()).unwrap_or(0))
        .max()
        .unwrap_or(0);

    worktrees
        .iter()
        .zip(pr_infos)
        .map(|(wt, pr_info)| {
            let branch = get_branch_display(wt);
            let is_current = current_dir.starts_with(&wt.path);
            let current_label = if is_current { CURRENT_LABEL } else { "" };

            let branch_pad = " ".repeat(branch_width - branch.len() - current_label.len() + 2);

            let status_column = if status_width > 0 {
                let status_len = pr_info.as_ref().map(|p| p.status.len()).unwrap_or(0);
                let status_pad = " ".repeat(status_width - status_len + 2);
                let status = match pr_info {
                    Some(pr) => colored_pr_status(&pr.status).to_string(),
                    None => String::new(),
                };
                format!("{}{}", status, status_pad)
            } else {
                String::new()
            };

            format!(
                "{}{}{}{}{}",
                branch.cyan(),
                current_label.yellow(),
                branch_pad,
                status_column,
                wt.path.display().to_string().dimmed()
            )
        })
        .collect()
}

/// Fetch PR status for each worktree, in the same order as the input.
/// Returns all None when no provider is configured/authenticated or fetching fails.
#[tokio::main]
async fn fetch_pr_statuses(worktrees: &[git::Worktree]) -> Vec<Option<PullRequestInfo>> {
    let no_info = |worktrees: &[git::Worktree]| worktrees.iter().map(|_| None).collect();

    let ctx = match PrContext::detect() {
        Ok(ctx) => ctx,
        Err(_) => return no_info(worktrees),
    };

    if !ctx.has_pr_info() {
        return no_info(worktrees);
    }

    println!("{}", "Fetching pull request status...".dimmed());

    let mut pr_infos = Vec::with_capacity(worktrees.len());
    for worktree in worktrees {
        let pr_info = match worktree.branch.as_ref().map(|b| clean_branch_name(b)) {
            Some(branch) => ctx.fetch_pr(branch).await,
            None => None,
        };
        pr_infos.push(pr_info);
    }
    pr_infos
}

/// Remove a single worktree (and its branch, unless protected).
/// When `skip_confirm` is true the removal confirmation prompt is skipped
/// (e.g. the user already confirmed via interactive selection).
fn remove_worktree(
    worktrees: &[git::Worktree],
    target_worktree: &git::Worktree,
    force: bool,
    skip_confirm: bool,
) -> Result<()> {
    // Check if this is the bare repository
    if target_worktree.bare {
        return Err(Error::msg("Cannot remove the main (bare) repository."));
    }

    // Check if target worktree is orphaned (after finding it in the list)
    if is_orphaned_worktree(&target_worktree.path) {
        let branch_display = get_branch_display(target_worktree);
        println!("{}", "⚠️  Detected orphaned worktree (stale git reference)".yellow());
        return remove_orphaned_worktree(&target_worktree.path, branch_display, force || skip_confirm);
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

    // Ask for confirmation unless --force is used or the user already confirmed
    if !force && !skip_confirm {
        print!("\n{}", "Are you sure you want to remove this worktree? (y/N): ".cyan());
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let confirmation = input.trim().to_lowercase();

        if confirmation != "y" && confirmation != "yes" {
            println!("{}", "Removal cancelled.".yellow());
            return Ok(());
        }
    }

    // Find project root from the worktree being removed (go up one level)
    let project_root = if let Some(parent) = target_worktree.path.parent() {
        find_project_root_from(parent)?
    } else {
        find_project_root_from(&target_worktree.path)?
    };

    // Execute pre-remove hooks before any removal operations (run from worktree directory)
    hooks::execute_hooks(
        "preRemove",
        &target_worktree.path,
        &[
            ("branchName", branch_display),
            ("worktreePath", target_worktree.path.to_str().unwrap()),
        ],
    )?;

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
                        let clean_branch = b.strip_prefix("refs/heads/").unwrap_or(b);
                        main_branches.contains(&clean_branch)
                    })
                    .unwrap_or(false)
        })
        .or_else(|| {
            // If no main branch, use any other worktree
            worktrees.iter().find(|wt| wt.path != target_worktree.path)
        })
        .ok_or_else(|| Error::msg("No other worktrees found to execute git command from."))?;

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

                    // Ask for confirmation to force delete unless --force is used
                    let should_force_delete = if force {
                        true
                    } else {
                        print!("{}", "Force delete the branch? (y/N): ".cyan());
                        io::stdout().flush()?;

                        let mut input = String::new();
                        io::stdin().read_line(&mut input)?;
                        let force_delete = input.trim().to_lowercase();
                        force_delete == "y" || force_delete == "yes"
                    };

                    if should_force_delete {
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
    Err(Error::msg(format!("Worktree for '{}' not found", target_branch)))
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

/// Remove an orphaned worktree (one with a stale git reference)
fn remove_orphaned_worktree(worktree_path: &std::path::Path, branch_name: &str, force: bool) -> Result<()> {
    use std::fs;

    // Show what will be removed
    println!("{}", "About to remove orphaned worktree:".cyan().bold());
    println!("  {}: {}", "Path".dimmed(), worktree_path.display());
    println!("  {}: {}", "Name".dimmed(), branch_name.green());
    println!("  {}: {}", "Status".dimmed(), "Orphaned (stale reference)".yellow());

    // Check if we're currently in the worktree being removed
    let current_dir = std::env::current_dir()?;
    let will_remove_current = current_dir.starts_with(worktree_path);

    if will_remove_current {
        println!(
            "\n{}",
            "⚠️  You are currently in this worktree. You will be moved to the project root after removal.".yellow()
        );
    }

    // Ask for confirmation unless --force is used
    if !force {
        print!(
            "\n{}",
            "Are you sure you want to remove this orphaned worktree? (y/N): ".cyan()
        );
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let confirmation = input.trim().to_lowercase();

        if confirmation != "y" && confirmation != "yes" {
            println!("{}", "Removal cancelled.".yellow());
            return Ok(());
        }
    }

    let project_root = find_project_root()?;

    // If we're currently in the worktree being removed, change directory first
    if will_remove_current {
        std::env::set_current_dir(&project_root)?;
    }

    // Remove the directory
    println!("\n{}", "Removing orphaned worktree directory...".cyan());
    fs::remove_dir_all(worktree_path)
        .map_err(|e| Error::msg(format!("Failed to remove directory {}: {}", worktree_path.display(), e)))?;

    println!(
        "{}",
        format!("✓ Directory removed: {}", worktree_path.display()).green()
    );

    // Try to prune worktree references from a valid git directory
    if let Ok(valid_git_dir) = find_valid_git_directory(&project_root) {
        println!("{}", "Pruning stale worktree references...".cyan());
        match git::prune_worktrees(&valid_git_dir) {
            Ok(_) => {
                println!("{}", "✓ Worktree references pruned".green());
            }
            Err(e) => {
                println!("{}", format!("⚠️  Failed to prune worktree references: {}", e).yellow());
            }
        }
    }

    if will_remove_current {
        println!(
            "{}",
            format!("✓ Moved to project root: {}", project_root.display()).green()
        );
    }

    println!(
        "\n{}",
        "Note: Orphaned worktree removed. Hooks were skipped due to invalid git state.".dimmed()
    );

    Ok(())
}
