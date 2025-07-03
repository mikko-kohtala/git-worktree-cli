use clap_complete::Shell;
use colored::Colorize;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{Error, Result};

// Include the generated completion files at compile time
const BASH_COMPLETION: &str = include_str!(concat!(env!("OUT_DIR"), "/completions/gwt.bash"));
const ZSH_COMPLETION: &str = include_str!(concat!(env!("OUT_DIR"), "/completions/_gwt"));
const FISH_COMPLETION: &str = include_str!(concat!(env!("OUT_DIR"), "/completions/gwt.fish"));
const POWERSHELL_COMPLETION: &str = include_str!(concat!(env!("OUT_DIR"), "/completions/_gwt.ps1"));
const ELVISH_COMPLETION: &str = include_str!(concat!(env!("OUT_DIR"), "/completions/gwt.elv"));

pub fn get_completion_content(shell: Shell) -> &'static str {
    match shell {
        Shell::Bash => BASH_COMPLETION,
        Shell::Zsh => ZSH_COMPLETION,
        Shell::Fish => FISH_COMPLETION,
        Shell::PowerShell => POWERSHELL_COMPLETION,
        Shell::Elvish => ELVISH_COMPLETION,
        _ => panic!("Unsupported shell: {:?}", shell),
    }
}

pub fn detect_shell() -> Result<Shell> {
    if let Ok(shell_path) = env::var("SHELL") {
        if shell_path.contains("zsh") {
            return Ok(Shell::Zsh);
        } else if shell_path.contains("bash") {
            return Ok(Shell::Bash);
        } else if shell_path.contains("fish") {
            return Ok(Shell::Fish);
        } else if shell_path.contains("elvish") {
            return Ok(Shell::Elvish);
        }
    }

    // Check for PowerShell on Windows
    if cfg!(windows) {
        return Ok(Shell::PowerShell);
    }

    // Default to zsh on macOS, bash on others
    if cfg!(target_os = "macos") {
        Ok(Shell::Zsh)
    } else {
        Ok(Shell::Bash)
    }
}

pub fn get_completion_install_path(shell: Shell) -> Result<PathBuf> {
    let home = env::var("HOME").map_err(|_| Error::msg("Could not determine home directory"))?;

    match shell {
        Shell::Bash => {
            // Check for common bash completion directories
            let paths = vec![
                PathBuf::from(&home).join(".local/share/bash-completion/completions"),
                PathBuf::from("/usr/local/share/bash-completion/completions"),
                PathBuf::from("/etc/bash_completion.d"),
            ];

            for path in paths {
                if path.exists() || path.parent().map(|p| p.exists()).unwrap_or(false) {
                    return Ok(path.join("gwt"));
                }
            }

            // Default to ~/.local/share
            Ok(PathBuf::from(&home).join(".local/share/bash-completion/completions/gwt"))
        }
        Shell::Zsh => {
            // For Zsh, we'll add to the user's fpath
            Ok(PathBuf::from(&home).join(".local/share/zsh/site-functions/_gwt"))
        }
        Shell::Fish => Ok(PathBuf::from(&home).join(".config/fish/completions/gwt.fish")),
        Shell::PowerShell => {
            // PowerShell profile location
            let profile_path = if cfg!(windows) {
                PathBuf::from(&env::var("USERPROFILE").unwrap_or(home))
                    .join("Documents/PowerShell/Modules/gwt-completions/gwt-completions.psm1")
            } else {
                PathBuf::from(&home).join(".config/powershell/Modules/gwt-completions/gwt-completions.psm1")
            };
            Ok(profile_path)
        }
        Shell::Elvish => Ok(PathBuf::from(&home).join(".elvish/lib/gwt-completions.elv")),
        _ => Err(Error::msg(format!("Unsupported shell: {:?}", shell))),
    }
}

pub fn install_completions_for_shell(shell: Shell) -> Result<()> {
    let content = get_completion_content(shell);
    let install_path = get_completion_install_path(shell)?;

    // Create parent directory if it doesn't exist
    if let Some(parent) = install_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Write the completion file
    fs::write(&install_path, content)?;

    println!(
        "✓ Installed {} completions to: {}",
        shell.to_string().green(),
        install_path.display().to_string().cyan()
    );

    // Shell-specific setup
    match shell {
        Shell::Bash => {
            println!("\nTo activate completions in your current shell, run:");
            println!("  {}", "source ~/.bashrc".cyan());
            println!("\nOr start a new terminal session.");
        }
        Shell::Zsh => {
            setup_zsh_completions()?;
        }
        Shell::Fish => {
            println!("\nCompletions will be available immediately in new fish sessions.");
        }
        Shell::PowerShell => {
            println!("\nTo activate completions, add the following to your PowerShell profile:");
            println!("  {}", "Import-Module gwt-completions".cyan());
            println!("\nYour profile is located at:");
            println!("  {}", "$PROFILE".cyan());
        }
        Shell::Elvish => {
            println!("\nTo activate completions, add the following to your ~/.elvish/rc.elv:");
            println!("  {}", "use gwt-completions".cyan());
        }
        _ => {}
    }

    Ok(())
}

fn setup_zsh_completions() -> Result<()> {
    let home = env::var("HOME")?;
    let zshrc_path = PathBuf::from(&home).join(".zshrc");

    ensure_zshrc_exists(&zshrc_path)?;

    let mut content = fs::read_to_string(&zshrc_path)?;
    let modified = add_zsh_completion_config(&mut content, &home)?;

    if modified {
        fs::write(&zshrc_path, content)?;
        println!("✓ Updated ~/.zshrc");
    }

    show_zsh_activation_instructions();
    Ok(())
}

fn ensure_zshrc_exists(zshrc_path: &Path) -> Result<()> {
    if !zshrc_path.exists() {
        println!("\n{}: ~/.zshrc does not exist. Creating it...", "Note".yellow());
        fs::write(zshrc_path, "")?;
    }
    Ok(())
}

fn add_zsh_completion_config(content: &mut String, home: &str) -> Result<bool> {
    let fpath_dir = format!("{}/.local/share/zsh/site-functions", home);

    if content.contains(&fpath_dir) {
        println!("\n✓ Completion path already configured in ~/.zshrc");
        return Ok(false);
    }

    println!("\n✓ Adding completion path to ~/.zshrc");

    // Ensure proper newline before adding content
    if !content.is_empty() && !content.ends_with('\n') {
        content.push('\n');
    }

    // Add fpath configuration
    content.push_str("\n# Git worktree CLI completions\n");
    content.push_str(&format!("fpath=({} $fpath)\n", fpath_dir));

    // Add compinit if not present
    if !content.contains("compinit") {
        content.push_str("autoload -Uz compinit && compinit\n");
    }

    Ok(true)
}

fn show_zsh_activation_instructions() {
    println!("\nTo activate completions in your current shell, run:");
    println!("  {}", "source ~/.zshrc".cyan());
    println!("\nOr start a new terminal session.");
}

pub fn check_completions_installed(shell: Shell) -> Result<bool> {
    let install_path = get_completion_install_path(shell)?;

    // Check if completion file exists
    if !install_path.exists() {
        return Ok(false);
    }

    // For Zsh, also check if fpath is configured
    if matches!(shell, Shell::Zsh) {
        let home = env::var("HOME")?;
        let zshrc_path = PathBuf::from(&home).join(".zshrc");

        if zshrc_path.exists() {
            let content = fs::read_to_string(&zshrc_path)?;
            let fpath_configured = content.contains(&format!("{}/.local/share/zsh/site-functions", home));
            return Ok(fpath_configured);
        }
    }

    Ok(true)
}
