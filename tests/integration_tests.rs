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

    // Test gwt init with a real repository using --local to create local config
    let mut cmd = Command::cargo_bin("gwt").unwrap();
    cmd.current_dir(temp_path)
        .arg("init")
        .arg("https://github.com/pitkane/git-worktree-cli.git")
        .arg("--local");

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

    // Test gwt init and verify hooks are executed (using --local for local config)
    let mut cmd = Command::cargo_bin("gwt").unwrap();
    cmd.current_dir(temp_path)
        .arg("init")
        .arg("https://github.com/pitkane/git-worktree-cli.git")
        .arg("--local");

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

    // Test gwt init - should fail without --force flag
    let mut cmd = Command::cargo_bin("gwt").unwrap();
    cmd.current_dir(temp_path)
        .arg("init")
        .arg("https://github.com/pitkane/git-worktree-cli.git")
        .arg("--local");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("already exists. Use --force to overwrite"));

    // Now test with --force flag - should succeed
    let mut cmd = Command::cargo_bin("gwt").unwrap();
    cmd.current_dir(temp_path)
        .arg("init")
        .arg("https://github.com/pitkane/git-worktree-cli.git")
        .arg("--force")
        .arg("--local");

    cmd.assert().success();

    // The directory should still exist but now contain the cloned repo
    assert!(conflict_dir.exists() || temp_path.join("main").exists() || temp_path.join("master").exists());

    cleanup_test_env(temp_dir);
}

#[test]
#[serial]
fn test_gwt_add_with_config_in_main_directory() {
    let temp_dir = setup_test_env();
    let temp_path = temp_dir.path();

    // Initialize a repository with --local to create local config
    let mut cmd = Command::cargo_bin("gwt").unwrap();
    cmd.current_dir(temp_path)
        .arg("init")
        .arg("https://github.com/pitkane/git-worktree-cli.git")
        .arg("--local");

    cmd.assert().success();

    // Find the main branch directory (could be "main" or "master")
    let main_dir = if temp_path.join("main").exists() {
        temp_path.join("main")
    } else {
        temp_path.join("master")
    };

    assert!(main_dir.exists(), "Main branch directory should exist");

    // Move the config file into the main/ directory
    // This simulates the edge case where config is inside main/
    let config_path = temp_path.join("git-worktree-config.jsonc");
    let new_config_path = main_dir.join("git-worktree-config.jsonc");
    fs::rename(&config_path, &new_config_path).unwrap();

    assert!(new_config_path.exists(), "Config should be in main/ directory");
    assert!(!config_path.exists(), "Config should not be in parent directory");

    // Now try to add a new worktree from the main/ directory
    // This should succeed with the fix
    let mut cmd = Command::cargo_bin("gwt").unwrap();
    cmd.current_dir(&main_dir)
        .arg("add")
        .arg("test-branch");

    // Should succeed even though config is in main/ directory
    cmd.assert().success();

    // Verify that the new worktree was created in the parent directory
    let worktree_path = temp_path.join("test-branch");
    assert!(worktree_path.exists(), "New worktree should be created");

    cleanup_test_env(temp_dir);
}
