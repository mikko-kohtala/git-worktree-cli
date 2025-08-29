use colored::Colorize;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::config::GitWorktreeConfig;
use crate::error::{Error, Result};

pub fn execute_hooks(hook_type: &str, working_directory: &Path, variables: &[(&str, &str)]) -> Result<()> {
    // Find the config file
    let config = match GitWorktreeConfig::find_config()? {
        Some((_, config)) => config,
        None => {
            // No config file found, skip hooks
            return Ok(());
        }
    };

    let hooks = match &config.hooks {
        Some(hooks) => hooks,
        None => return Ok(()),
    };

    let hook_commands = match hook_type {
        "postAdd" => &hooks.post_add,
        "preRemove" => &hooks.pre_remove,
        "postRemove" => &hooks.post_remove,
        _ => return Ok(()),
    };

    let hook_commands = match hook_commands {
        Some(commands) => commands,
        None => return Ok(()),
    };

    if hook_commands.is_empty() {
        return Ok(());
    }

    println!("{}", format!("🪝 Running {} hooks...", hook_type).cyan());

    for hook in hook_commands {
        // Replace variables in the hook command
        let mut command = hook.clone();
        for (var_name, var_value) in variables {
            let placeholder = format!("${{{}}}", var_name);
            command = command.replace(&placeholder, var_value);
        }

        println!("   {}", format!("Executing: {}", command).blue());

        // Execute with streaming output - this is the key improvement!
        match execute_command_streaming(&command, working_directory) {
            Ok(()) => {
                println!("   {}", "✓ Hook completed successfully".green());
            }
            Err(e) => {
                println!("   {}", format!("⚠️  Hook failed: {}", e).yellow());
                // Continue with other hooks even if one fails
            }
        }
    }

    Ok(())
}

fn execute_command_streaming(command: &str, working_directory: &Path) -> Result<()> {
    let mut cmd = Command::new("sh");
    cmd.arg("-c")
        .arg(command)
        .current_dir(working_directory)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .env("FORCE_COLOR", "1");

    let status = cmd.status().map_err(|e| Error::hook(format!("Failed to execute hook command: {}", e)))?;

    if !status.success() {
        return Err(Error::hook(format!("Command failed with exit code: {:?}", status.code())));
    }

    Ok(())
}
