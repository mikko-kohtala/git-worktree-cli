use assert_cmd::Command;
use predicates::prelude::*;
use serial_test::serial;
use std::fs;

mod test_utils;
use test_utils::*;

#[test]
#[serial]
fn test_gwt_init_existing_repo() {
    let temp_dir = setup_test_env();
    let temp_path = temp_dir.path();

    // Create a subdirectory for the repo (simulating /code/my-repo structure)
    let repo_dir = temp_path.join("my-repo");
    fs::create_dir(&repo_dir).unwrap();

    // Create a test git repo with a GitHub remote
    create_test_git_repo(&repo_dir, "git@github.com:test/my-repo.git");

    // Test gwt init in an existing repo with --local
    let mut cmd = Command::cargo_bin("gwt").unwrap();
    cmd.current_dir(&repo_dir).arg("init").arg("--local");

    let output = cmd.assert().success();

    // Check that the command outputs expected messages
    output
        .stdout(predicate::str::contains("Detected provider: Github"))
        .stdout(predicate::str::contains("✓ Repository: git@github.com:test/my-repo.git"))
        .stdout(predicate::str::contains("✓ Project path:"))
        .stdout(predicate::str::contains("✓ Worktrees path:"))
        .stdout(predicate::str::contains("my-repo-worktrees"))
        .stdout(predicate::str::contains("✓ Config saved to:"));

    // Check that config was created in parent directory (for --local)
    let config_path = temp_path.join("git-worktree-config.jsonc");
    assert!(config_path.exists(), "Config file should be created");

    // Verify config file content
    let config_content = fs::read_to_string(&config_path).unwrap();
    assert!(config_content.contains("\"repositoryUrl\": \"git@github.com:test/my-repo.git\""));
    assert!(config_content.contains("\"mainBranch\":"));
    assert!(config_content.contains("\"worktreesPath\":"));
    assert!(config_content.contains("my-repo-worktrees"));

    cleanup_test_env(temp_dir);
}

#[test]
#[serial]
fn test_gwt_init_not_in_git_repo() {
    let temp_dir = setup_test_env();
    let temp_path = temp_dir.path();

    // Test gwt init outside a git repository - should fail
    let mut cmd = Command::cargo_bin("gwt").unwrap();
    cmd.current_dir(temp_path).arg("init");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Not in a git repository"));

    cleanup_test_env(temp_dir);
}

#[test]
#[serial]
fn test_gwt_init_no_remote() {
    let temp_dir = setup_test_env();
    let temp_path = temp_dir.path();

    // Create a git repo without a remote
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(temp_path)
        .output()
        .expect("Failed to init git repo");

    // Test gwt init - should fail because no remote
    let mut cmd = Command::cargo_bin("gwt").unwrap();
    cmd.current_dir(temp_path).arg("init");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("No remote 'origin' found"));

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
fn test_gwt_init_bitbucket_repo() {
    let temp_dir = setup_test_env();
    let temp_path = temp_dir.path();

    // Create a subdirectory for the repo
    let repo_dir = temp_path.join("my-bb-repo");
    fs::create_dir(&repo_dir).unwrap();

    // Create a test git repo with a Bitbucket remote
    create_test_git_repo(&repo_dir, "git@bitbucket.org:workspace/my-bb-repo.git");

    // Test gwt init with --local
    let mut cmd = Command::cargo_bin("gwt").unwrap();
    cmd.current_dir(&repo_dir).arg("init").arg("--local");

    let output = cmd.assert().success();

    output.stdout(predicate::str::contains("Detected provider: BitbucketCloud"));

    cleanup_test_env(temp_dir);
}

#[test]
#[serial]
fn test_gwt_init_unsupported_provider() {
    let temp_dir = setup_test_env();
    let temp_path = temp_dir.path();

    // Create a subdirectory for the repo
    let repo_dir = temp_path.join("my-repo");
    fs::create_dir(&repo_dir).unwrap();

    // Create a test git repo with an unsupported remote
    create_test_git_repo(&repo_dir, "git@gitlab.com:user/repo.git");

    // Test gwt init - should fail with unsupported provider
    let mut cmd = Command::cargo_bin("gwt").unwrap();
    cmd.current_dir(&repo_dir).arg("init");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Could not detect repository provider"));

    cleanup_test_env(temp_dir);
}

#[test]
#[serial]
fn test_config_worktrees_path_derivation() {
    let temp_dir = setup_test_env();
    let temp_path = temp_dir.path();

    // Create repo at /tmp/xxx/agent-tools
    let repo_dir = temp_path.join("agent-tools");
    fs::create_dir(&repo_dir).unwrap();

    create_test_git_repo(&repo_dir, "git@github.com:test/agent-tools.git");

    // Initialize
    let mut cmd = Command::cargo_bin("gwt").unwrap();
    cmd.current_dir(&repo_dir).arg("init").arg("--local");

    cmd.assert().success();

    // Check config has correct worktrees_path
    let config_path = temp_path.join("git-worktree-config.jsonc");
    let config_content = fs::read_to_string(&config_path).unwrap();

    // Should have worktrees path as sibling with -worktrees suffix
    assert!(
        config_content.contains("agent-tools-worktrees"),
        "Config should have worktrees path with -worktrees suffix"
    );

    cleanup_test_env(temp_dir);
}
