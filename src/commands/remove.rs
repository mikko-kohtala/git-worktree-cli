use colored::Colorize;
use ratatui::{
    backend::CrosstermBackend,
    crossterm::{
        event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    },
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};
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
    let rows = build_picker_rows(&removable, &pr_infos, &current_dir);

    let indices = match pick_worktrees(&rows)? {
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

const CURRENT_LABEL: &str = " (current)";
const COLUMN_GAP: usize = 2;

/// One row in the interactive picker
struct PickerRow {
    branch: String,
    current: bool,
    status: Option<String>,
    path: String,
}

fn build_picker_rows(
    worktrees: &[git::Worktree],
    pr_infos: &[Option<PullRequestInfo>],
    current_dir: &std::path::Path,
) -> Vec<PickerRow> {
    worktrees
        .iter()
        .zip(pr_infos)
        .map(|(wt, pr_info)| PickerRow {
            branch: get_branch_display(wt).to_string(),
            current: current_dir.starts_with(&wt.path),
            status: pr_info.as_ref().map(|pr| pr.status.clone()),
            path: display_path(&wt.path),
        })
        .collect()
}

/// Widths of the branch (including the current marker) and status columns
fn column_widths(rows: &[PickerRow]) -> (usize, usize) {
    let branch_width = rows
        .iter()
        .map(|row| {
            let current_len = if row.current { CURRENT_LABEL.len() } else { 0 };
            row.branch.chars().count() + current_len
        })
        .max()
        .unwrap_or(0);

    // Status column is only shown when at least one worktree has PR info
    let status_width = rows
        .iter()
        .map(|row| row.status.as_ref().map(|s| s.len()).unwrap_or(0))
        .max()
        .unwrap_or(0);

    (branch_width, status_width)
}

fn status_style(status: &str) -> Style {
    let color = match status {
        "OPEN" | "MERGED" => Color::Green,
        "DRAFT" => Color::Yellow,
        "CLOSED" => Color::Red,
        _ => Color::White,
    };
    Style::default().fg(color)
}

/// Render one picker row as a column-aligned styled line
fn picker_line(
    row: &PickerRow,
    selected: bool,
    branch_width: usize,
    status_width: usize,
    path_budget: usize,
) -> Line<'static> {
    let checkbox = if selected {
        Span::styled("[x] ".to_string(), Style::default().fg(Color::Green))
    } else {
        Span::raw("[ ] ".to_string())
    };

    let current_label = if row.current { CURRENT_LABEL } else { "" };
    let label_width = row.branch.chars().count() + current_label.len();
    let branch_pad = " ".repeat(branch_width.saturating_sub(label_width) + COLUMN_GAP);

    let mut spans = vec![
        checkbox,
        Span::styled(row.branch.clone(), Style::default().fg(Color::Cyan)),
        Span::styled(current_label.to_string(), Style::default().fg(Color::Yellow)),
        Span::raw(branch_pad),
    ];

    if status_width > 0 {
        let status = row.status.clone().unwrap_or_default().to_lowercase();
        let status_pad = " ".repeat(status_width.saturating_sub(status.chars().count()) + COLUMN_GAP);
        spans.push(Span::styled(status, status_style(row.status.as_deref().unwrap_or(""))));
        spans.push(Span::raw(status_pad));
    }

    spans.push(Span::styled(
        truncate_left(&row.path, path_budget),
        Style::default().fg(Color::DarkGray),
    ));

    Line::from(spans)
}

/// Run the full-screen worktree picker. Returns the selected indices,
/// or None if the user cancelled.
fn pick_worktrees(rows: &[PickerRow]) -> Result<Option<Vec<usize>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = picker_loop(&mut terminal, rows);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    result
}

fn picker_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    rows: &[PickerRow],
) -> Result<Option<Vec<usize>>> {
    let mut selected = vec![false; rows.len()];
    let mut cursor: usize = 0;
    let mut list_state = ListState::default();

    loop {
        list_state.select(Some(cursor));
        terminal.draw(|frame| render_picker(frame, rows, &selected, &mut list_state))?;

        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return Ok(None);
        }

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => cursor = cursor.saturating_sub(1),
            KeyCode::Down | KeyCode::Char('j') => cursor = (cursor + 1).min(rows.len() - 1),
            KeyCode::Home => cursor = 0,
            KeyCode::End => cursor = rows.len() - 1,
            KeyCode::Char(' ') => selected[cursor] = !selected[cursor],
            KeyCode::Char('a') => {
                let select_all = !selected.iter().all(|s| *s);
                selected.iter_mut().for_each(|s| *s = select_all);
            }
            KeyCode::Enter => {
                return Ok(Some(
                    selected
                        .iter()
                        .enumerate()
                        .filter_map(|(i, s)| s.then_some(i))
                        .collect(),
                ));
            }
            KeyCode::Esc | KeyCode::Char('q') => return Ok(None),
            _ => {}
        }
    }
}

fn render_picker(frame: &mut Frame, rows: &[PickerRow], selected: &[bool], list_state: &mut ListState) {
    let [list_area, help_area] = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).areas(frame.area());

    let (branch_width, status_width) = column_widths(rows);
    // Borders (2) + highlight symbol (2) + checkbox (4) + fixed columns;
    // whatever is left belongs to the path
    let fixed = 2 + 2 + 4 + branch_width + COLUMN_GAP + if status_width > 0 { status_width + COLUMN_GAP } else { 0 };
    let path_budget = (list_area.width as usize).saturating_sub(fixed);

    let items: Vec<ListItem> = rows
        .iter()
        .zip(selected)
        .map(|(row, sel)| ListItem::new(picker_line(row, *sel, branch_width, status_width, path_budget)))
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(" Remove worktrees "))
        .highlight_symbol("❯ ")
        .highlight_style(Style::default().add_modifier(Modifier::BOLD));

    frame.render_stateful_widget(list, list_area, list_state);

    let help = Paragraph::new("↑/↓/j/k move · space toggle · a all · enter confirm · q/esc cancel")
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(help, help_area);
}

/// Render a path with the home directory shortened to `~`
fn display_path(path: &std::path::Path) -> String {
    let displayed = path.display().to_string();
    if let Some(home) = dirs::home_dir() {
        if let Some(rest) = displayed.strip_prefix(&home.display().to_string()) {
            return format!("~{}", rest);
        }
    }
    displayed
}

/// Truncate a string to `max_width` characters, keeping the tail
/// (the most informative part of a path) behind a leading ellipsis
fn truncate_left(text: &str, max_width: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= max_width {
        return text.to_string();
    }
    if max_width == 0 {
        return String::new();
    }
    let tail: String = chars[chars.len() - (max_width - 1)..].iter().collect();
    format!("…{}", tail)
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn worktree(path: &str, branch: &str) -> git::Worktree {
        git::Worktree {
            path: PathBuf::from(path),
            head: "0123456789abcdef".to_string(),
            branch: Some(format!("refs/heads/{}", branch)),
            bare: false,
        }
    }

    fn pr(status: &str) -> Option<PullRequestInfo> {
        Some(PullRequestInfo {
            url: "https://example.com/pr/1".to_string(),
            status: status.to_string(),
            title: "A pull request".to_string(),
        })
    }

    #[test]
    fn truncate_left_keeps_short_strings() {
        assert_eq!(truncate_left("short", 10), "short");
        assert_eq!(truncate_left("exact", 5), "exact");
    }

    #[test]
    fn truncate_left_keeps_the_tail() {
        assert_eq!(truncate_left("abcdefgh", 5), "…efgh");
        assert_eq!(truncate_left("abc", 0), "");
        assert_eq!(truncate_left("abc", 1), "…");
    }

    fn line_text(line: &Line) -> String {
        line.spans.iter().map(|span| span.content.as_ref()).collect()
    }

    #[test]
    fn picker_rows_capture_current_and_status() {
        let worktrees = vec![
            worktree("/somewhere/code/salimake", "main"),
            worktree(
                "/somewhere/code/salimake-worktrees/decline-machine-press-images",
                "decline-machine-press-images",
            ),
        ];
        let pr_infos = vec![None, pr("OPEN")];

        let rows = build_picker_rows(&worktrees, &pr_infos, &worktrees[0].path.clone());

        assert!(rows[0].current);
        assert!(!rows[1].current);
        assert_eq!(rows[0].status, None);
        assert_eq!(rows[1].status, Some("OPEN".to_string()));
        assert_eq!(rows[1].branch, "decline-machine-press-images");
        assert_eq!(
            rows[1].path,
            "/somewhere/code/salimake-worktrees/decline-machine-press-images"
        );
    }

    #[test]
    fn picker_lines_align_columns() {
        let worktrees = vec![
            worktree("/repo", "main"),
            worktree("/repo-worktrees/a-much-longer-branch-name", "a-much-longer-branch-name"),
        ];
        let pr_infos = vec![pr("OPEN"), pr("MERGED")];
        let rows = build_picker_rows(&worktrees, &pr_infos, std::path::Path::new("/elsewhere"));

        let (branch_width, status_width) = column_widths(&rows);
        let lines: Vec<String> = rows
            .iter()
            .map(|row| line_text(&picker_line(row, false, branch_width, status_width, 200)))
            .collect();

        let path_columns: Vec<usize> = lines
            .iter()
            .map(|line| line.find('/').expect("path should be present"))
            .collect();
        assert_eq!(
            path_columns[0], path_columns[1],
            "paths should start at the same column: {:?}",
            lines
        );
    }

    #[test]
    fn picker_lines_respect_path_budget() {
        let worktrees = vec![worktree(
            "/somewhere/code/salimake-worktrees/decline-machine-press-images",
            "decline-machine-press-images",
        )];
        let pr_infos = vec![pr("OPEN")];
        let rows = build_picker_rows(&worktrees, &pr_infos, std::path::Path::new("/elsewhere"));

        let (branch_width, status_width) = column_widths(&rows);
        let path_budget = 20;
        let line = line_text(&picker_line(&rows[0], true, branch_width, status_width, path_budget));

        let expected_max = 4 + branch_width + COLUMN_GAP + status_width + COLUMN_GAP + path_budget;
        assert!(
            line.chars().count() <= expected_max,
            "line too wide ({} > {}): {}",
            line.chars().count(),
            expected_max,
            line
        );
        assert!(
            line.contains("machine-press-images"),
            "truncated path should keep its tail: {}",
            line
        );
        assert!(line.contains('…'), "long path should be truncated: {}", line);
        assert!(
            line.starts_with("[x] "),
            "selected row should show a checked box: {}",
            line
        );
    }

    #[test]
    fn status_column_collapses_without_pr_info() {
        let worktrees = vec![worktree("/repo", "main")];
        let pr_infos = vec![None];
        let rows = build_picker_rows(&worktrees, &pr_infos, std::path::Path::new("/elsewhere"));

        let (branch_width, status_width) = column_widths(&rows);
        assert_eq!(status_width, 0);

        let line = line_text(&picker_line(&rows[0], false, branch_width, status_width, 200));
        assert_eq!(line, format!("[ ] main{}/repo", " ".repeat(COLUMN_GAP)));
    }
}
