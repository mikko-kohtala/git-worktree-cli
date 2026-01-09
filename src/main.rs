use clap::Parser;
use colored::Colorize;

use git_worktree_cli::{
    cli::{AuthAction, Cli, Commands, CompletionAction},
    commands::{add, auth, init, list, remove},
    completions,
    error::Result,
};

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { local } => {
            init::run(local)?;
        }
        Commands::Add { branch_name } => {
            add::run(&branch_name)?;
        }
        Commands::List { local } => {
            list::run(local)?;
        }
        Commands::Remove { branch_name, force } => {
            remove::run(branch_name.as_deref(), force)?;
        }
        Commands::Auth { action } => match action {
            AuthAction::Github => {
                auth::run()?;
            }
            AuthAction::BitbucketCloud { action } => {
                auth::run_bitbucket_cloud(action)?;
            }
            AuthAction::BitbucketDataCenter { action } => {
                auth::run_bitbucket_data_center(action)?;
            }
        },
        Commands::Completions { action } => {
            handle_completions(action)?;
        }
    }

    Ok(())
}

fn handle_completions(action: Option<CompletionAction>) -> Result<()> {
    match action {
        None => {
            // Default behavior: check if completions are installed
            check_completions_status()?;
        }
        Some(CompletionAction::Generate { shell }) => {
            // Output the pre-generated completion to stdout
            println!("{}", completions::get_completion_content(shell));
        }
        Some(CompletionAction::Install { shell }) => {
            let shell = shell.unwrap_or_else(|| completions::detect_shell().unwrap_or(clap_complete::Shell::Bash));
            completions::install_completions_for_shell(shell)?;
        }
    }
    Ok(())
}

fn check_completions_status() -> Result<()> {
    let shell = completions::detect_shell()?;
    println!("Detected shell: {}", shell.to_string().green());

    let installed = completions::check_completions_installed(shell)?;

    if installed {
        println!("✓ Completions appear to be installed");
        println!("\nTo reinstall or update, run: {}", "gwt completions install".cyan());
    } else {
        println!("✗ Completions not installed");
        println!("\nTo install completions, run: {}", "gwt completions install".cyan());
    }

    println!("\nTo generate completions for a specific shell:");
    println!("  {}", "gwt completions generate <shell>".cyan());
    println!("\nSupported shells: bash, zsh, fish, powershell, elvish");

    Ok(())
}
