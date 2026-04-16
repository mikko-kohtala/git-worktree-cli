use crate::config::GitWorktreeConfig;
use crate::error::{Error, Result};

pub fn run() -> Result<()> {
    let (config_path, _config) = GitWorktreeConfig::find_config()?
        .ok_or_else(|| Error::config("Config not found. Run 'gwt init' from your project directory to create one."))?;

    println!("Opening config: {}", config_path.display());

    #[cfg(target_os = "macos")]
    let cmd = "open";
    #[cfg(target_os = "linux")]
    let cmd = "xdg-open";
    #[cfg(target_os = "windows")]
    let cmd = "start";

    std::process::Command::new(cmd)
        .arg(&config_path)
        .spawn()
        .map_err(|e| Error::config(format!("Failed to open config file: {}", e)))?;

    Ok(())
}
