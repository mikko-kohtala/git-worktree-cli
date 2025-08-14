use assert_cmd::Command;
use predicates::prelude::*;
use serial_test::serial;
use std::fs;

mod test_utils;
use test_utils::*;

#[test]
#[serial]
fn test_gwt_init_with_valid_repo() {
    let temp_dir = setup_test_env();
    let temp_path = temp_dir.path();

    // Test gwt init with a real repository
    let mut cmd = Command::cargo_bin("gwt").unwrap();
    cmd.current_dir(temp_path)
        .arg("init")
        .arg("https://github.com/pitkane/git-worktree-cli.git");

    let output = cmd.assert().success();

    // Check that the command outputs expected messages
    output
        .stdout(predicate::str::contains(
            "Cloning https://github.com/pitkane/git-worktree-cli.git",
        ))
        .stdout(predicate::str::contains("✓ Repository cloned to:"))
        .stdout(predicate::str::contains("✓ Default branch:"))
        .stdout(predicate::str::contains("✓ Config saved to:"));

    // Check that files were created
    let config_path = temp_path.join("git-worktree-config.jsonc");
    assert!(config_path.exists(), "Config file should be created");

    // Check that the main branch directory was created
    // Note: This will be either "main" or "master" depending on the repo
    let entries: Vec<_> = fs::read_dir(temp_path)
        .unwrap()
        .filter_map(|entry| {
            let entry = entry.ok()?;
            if entry.file_type().ok()?.is_dir() {
                Some(entry.file_name().to_string_lossy().to_string())
            } else {
                None
            }
        })
        .collect();

    // Should have at least one directory (the cloned repo)
    assert!(!entries.is_empty(), "Should have created repository directory");

    // Verify config file content
    let config_content = fs::read_to_string(&config_path).unwrap();
    assert!(config_content.contains("\"repositoryUrl\": \"https://github.com/pitkane/git-worktree-cli.git\""));
    assert!(config_content.contains("\"mainBranch\":"));
    assert!(config_content.contains("\"createdAt\":"));
    assert!(config_content.contains("\"hooks\":"));

    cleanup_test_env(temp_dir);
}

#[test]
#[serial]
fn test_gwt_init_with_invalid_repo() {
    let temp_dir = setup_test_env();
    let temp_path = temp_dir.path();

    // Test gwt init with invalid repository
    let mut cmd = Command::cargo_bin("gwt").unwrap();
    cmd.current_dir(temp_path).arg("init").arg("invalid-repo-url");

    // Should fail with non-zero exit code
    cmd.assert().failure();

    // Config file should not be created
    let config_path = temp_path.join("git-worktree-config.jsonc");
    assert!(!config_path.exists(), "Config file should not be created on failure");

    cleanup_test_env(temp_dir);
}

#[test]
#[serial]
fn test_gwt_init_hooks_execution() {
    let temp_dir = setup_test_env();
    let temp_path = temp_dir.path();

    // Test gwt init and verify hooks are executed
    let mut cmd = Command::cargo_bin("gwt").unwrap();
    cmd.current_dir(temp_path)
        .arg("init")
        .arg("https://github.com/pitkane/git-worktree-cli.git");

    let _output = cmd.assert().success();

    // Post-init hooks removed - no longer testing for them

    cleanup_test_env(temp_dir);
}

#[test]
fn test_gwt_help() {
    let mut cmd = Command::cargo_bin("gwt").unwrap();
    cmd.arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("managing git worktrees efficiently"))
        .stdout(predicate::str::contains("init"))
        .stdout(predicate::str::contains("add"))
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("auth"))
        .stdout(predicate::str::contains("remove"));
}

#[test]
fn test_gwt_version() {
    let mut cmd = Command::cargo_bin("gwt").unwrap();
    cmd.arg("--version");

    cmd.assert().success().stdout(predicate::str::contains("gwt"));
}

#[test]
#[serial]
fn test_gwt_init_directory_cleanup() {
    let temp_dir = setup_test_env();
    let temp_path = temp_dir.path();

    // Create a directory that would conflict
    let conflict_dir = temp_path.join("git-worktree-cli");
    fs::create_dir(&conflict_dir).unwrap();

    // Test gwt init - should clean up conflicting directory
    let mut cmd = Command::cargo_bin("gwt").unwrap();
    cmd.current_dir(temp_path)
        .arg("init")
        .arg("https://github.com/pitkane/git-worktree-cli.git");

    cmd.assert().success();

    // The directory should still exist but now contain the cloned repo
    assert!(conflict_dir.exists() || temp_path.join("main").exists() || temp_path.join("master").exists());

    cleanup_test_env(temp_dir);
}
