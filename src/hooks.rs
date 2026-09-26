use colored::Colorize;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::config::GitWorktreeConfig;
use crate::error::{Error, Result};

/// The flag that turns hook failures back into warnings
pub const IGNORE_HOOK_ERRORS_FLAG: &str = "--ignore-hook-errors";

/// Commands configured for a hook type, if any
fn configured_hooks(hook_type: &str) -> Result<Vec<String>> {
    let Some((_, config)) = GitWorktreeConfig::find_config()? else {
        return Ok(Vec::new());
    };
    let Some(hooks) = config.hooks else {
        return Ok(Vec::new());
    };

    let commands = match hook_type {
        "postAdd" => hooks.post_add,
        "preRemove" => hooks.pre_remove,
        "postRemove" => hooks.post_remove,
        _ => None,
    };
    Ok(commands.unwrap_or_default())
}

/// Run the configured commands for `hook_type` in order, streaming their output.
///
/// A command that exits non-zero (or cannot be started) stops the run and
/// returns `Error::Hook` naming the hook type and the command, so the caller
/// can fail closed. With `ignore_errors` the failure is only a warning and the
/// remaining commands still run.
pub fn execute_hooks(
    hook_type: &str,
    working_directory: &Path,
    variables: &[(&str, &str)],
    ignore_errors: bool,
) -> Result<()> {
    let hook_commands = configured_hooks(hook_type)?;
    if hook_commands.is_empty() {
        return Ok(());
    }

    println!("{}", format!("🪝 Running {} hooks...", hook_type).cyan());

    for hook in hook_commands {
        // Replace variables in the hook command
        let mut command = hook;
        for (var_name, var_value) in variables {
            let placeholder = format!("${{{}}}", var_name);
            command = command.replace(&placeholder, var_value);
        }

        println!("   {}", format!("Executing: {}", command).blue());

        match execute_command_streaming(&command, working_directory) {
            Ok(()) => {
                println!("   {}", "✓ Hook completed successfully".green());
            }
            Err(reason) if ignore_errors => {
                println!(
                    "   {}",
                    format!(
                        "⚠️  Hook failed ({}), continuing because of {}",
                        reason, IGNORE_HOOK_ERRORS_FLAG
                    )
                    .yellow()
                );
            }
            Err(reason) => {
                println!("   {}", format!("✗ Hook failed: {}", reason).red());
                return Err(Error::hook(format!(
                    "{} hook failed ({}): {}",
                    hook_type, reason, command
                )));
            }
        }
    }

    Ok(())
}

/// Run one hook command with inherited stdout/stderr. On failure returns a
/// short reason such as "exit code 1".
fn execute_command_streaming(command: &str, working_directory: &Path) -> std::result::Result<(), String> {
    let status = Command::new("sh")
        .arg("-c")
        .arg(command)
        .current_dir(working_directory)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .env("FORCE_COLOR", "1")
        .status()
        .map_err(|e| format!("could not start: {}", e))?;

    if status.success() {
        return Ok(());
    }
    Err(match status.code() {
        Some(code) => format!("exit code {}", code),
        None => "killed by a signal".to_string(),
    })
}
