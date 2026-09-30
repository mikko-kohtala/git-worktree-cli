use assert_cmd::Command;
use predicates::prelude::*;
use serial_test::serial;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;
use tempfile::TempDir;

mod test_utils;
use test_utils::*;

/// A home directory with `~/code/<name>` cloned from a local bare origin, so
/// `gwt add` can fetch without network access. The config lives in `~/code`
/// and names no worktreesPath, so gwt derives it.
struct Home {
    _temp_dir: TempDir,
    home: PathBuf,
    code: PathBuf,
    repo: PathBuf,
}

impl Home {
    fn new(name: &str) -> Self {
        let temp_dir = setup_test_env();
        // Canonicalize so paths match what git reports (/var -> /private/var on macOS)
        let home = temp_dir.path().canonicalize().unwrap();
        let code = home.join("code");
        let repo = code.join(name);
        let origin = home.join("origin.git");
        fs::create_dir_all(&repo).unwrap();

        git(&["init", "--bare", origin.to_str().unwrap()], &home);
        create_test_git_repo(&repo, origin.to_str().unwrap());
        git(&["branch", "-M", "main"], &repo);
        git(&["push", "-u", "origin", "main"], &repo);

        fs::write(
            code.join("git-worktree-config.jsonc"),
            r#"{
  "repositoryUrl": "git@github.com:test/my-repo.git",
  "mainBranch": "main",
  "createdAt": "2025-06-25T17:25:28.766876Z",
  "sourceControl": "github"
}"#,
        )
        .unwrap();

        Self {
            _temp_dir: temp_dir,
            home,
            code,
            repo,
        }
    }

    fn gwt(&self, cwd: &Path) -> Command {
        let mut cmd = Command::cargo_bin("gwt").unwrap();
        cmd.current_dir(cwd).env("HOME", &self.home);
        cmd
    }

    fn gwt_cd(&self, cwd: &Path) -> String {
        let output = self.gwt(cwd).arg("cd").output().unwrap();
        assert!(
            output.status.success(),
            "gwt cd failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap().trim().to_string()
    }
}

fn git(args: &[&str], cwd: &Path) {
    let output = StdCommand::new("git").args(args).current_dir(cwd).output().unwrap();
    assert!(
        output.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
#[serial]
fn test_worktrees_go_to_shared_folder_next_to_repo() {
    let home = Home::new("my-repo");
    let worktrees = home.code.join(".worktrees").join("my-repo");
    let worktree = worktrees.join("feature").join("x");

    home.gwt(&home.repo).args(["add", "feature/x"]).assert().success();
    assert!(
        worktree.join(".git").is_file(),
        "worktree should be created in .worktrees"
    );

    // The project is found from the main repo, a worktree and the worktrees folder
    assert_eq!(home.gwt_cd(&home.repo), worktrees.display().to_string());
    assert_eq!(home.gwt_cd(&worktree), worktrees.display().to_string());
    assert_eq!(home.gwt_cd(&worktrees), worktrees.display().to_string());

    // Removing the last worktree deletes the folders it leaves empty
    home.gwt(&worktree)
        .args(["remove", "-f", "feature/x"])
        .assert()
        .success();
    assert!(!worktree.exists());
    assert!(
        !home.code.join(".worktrees").exists(),
        "empty .worktrees folder should be removed"
    );
    assert!(home.repo.join(".git").is_dir(), "main repo should be untouched");
}

#[test]
#[serial]
fn test_remove_keeps_worktrees_folder_in_use() {
    let home = Home::new("my-repo");
    let worktrees = home.code.join(".worktrees").join("my-repo");

    home.gwt(&home.repo).args(["add", "feature/x"]).assert().success();
    home.gwt(&home.repo).args(["add", "feature-y"]).assert().success();
    home.gwt(&home.repo)
        .args(["remove", "-f", "feature/x"])
        .assert()
        .success();

    assert!(
        !worktrees.join("feature").exists(),
        "empty branch folder should be removed"
    );
    assert!(worktrees.join("feature-y").exists(), "other worktree should be kept");
}

#[test]
#[serial]
fn test_existing_legacy_worktrees_folder_is_kept() {
    let home = Home::new("my-repo");
    let legacy = home.code.join("my-repo-worktrees");
    fs::create_dir(&legacy).unwrap();

    home.gwt(&home.repo).args(["add", "feature-x"]).assert().success();
    assert!(legacy.join("feature-x").join(".git").is_file());
    assert!(!home.code.join(".worktrees").exists());
    assert_eq!(home.gwt_cd(&legacy.join("feature-x")), legacy.display().to_string());
}

#[test]
#[serial]
fn test_worktrees_root_from_settings() {
    let home = Home::new("my-repo");
    let config_dir = home.home.join(".config").join("git-worktree-cli");
    fs::create_dir_all(&config_dir).unwrap();
    fs::write(
        config_dir.join("settings.jsonc"),
        r#"{ "worktreesRoot": "~/.worktrees" }"#,
    )
    .unwrap();

    // Named after the repo's path relative to home
    let worktrees = home.home.join(".worktrees").join("code-my-repo");
    let worktree = worktrees.join("feature-x");

    home.gwt(&home.repo).args(["add", "feature-x"]).assert().success();
    assert!(
        worktree.join(".git").is_file(),
        "worktree should be created under the root"
    );
    assert_eq!(home.gwt_cd(&worktree), worktrees.display().to_string());

    home.gwt(&worktree)
        .args(["remove", "-f", "feature-x"])
        .assert()
        .success();
    assert!(!worktrees.exists(), "empty project folder should be removed");
    // A root named .worktrees is cleaned up like the shared folder; gwt add recreates it
    assert!(!home.home.join(".worktrees").exists());
}

/// A repository at `<home>/<path>` with a GitHub remote, ready for `gwt init`
fn init_repo(home: &Path, path: &str, remote: &str) -> PathBuf {
    let repo = home.join(path);
    fs::create_dir_all(&repo).unwrap();
    create_test_git_repo(&repo, remote);
    repo
}

#[test]
#[serial]
fn test_init_with_worktrees_root() {
    let temp_dir = setup_test_env();
    let home = temp_dir.path().canonicalize().unwrap();
    let repo = init_repo(&home, "code/my-repo", "git@github.com:test/my-repo.git");

    Command::cargo_bin("gwt")
        .unwrap()
        .current_dir(&repo)
        .env("HOME", &home)
        .args(["init", "--worktrees-root", "~/.worktrees"])
        .assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "✓ Worktrees path: {}",
            home.join(".worktrees").join("code-my-repo").display()
        )));
}

#[test]
#[serial]
fn test_init_refuses_worktrees_folder_of_another_project() {
    let temp_dir = setup_test_env();
    let home = temp_dir.path().canonicalize().unwrap();
    // Both encode to code-my-app under the root
    let first = init_repo(&home, "code/my-app", "git@github.com:test/first.git");
    let second = init_repo(&home, "code-my/app", "git@github.com:test/second.git");

    let init = |repo: &Path| {
        let mut cmd = Command::cargo_bin("gwt").unwrap();
        cmd.current_dir(repo)
            .env("HOME", &home)
            .args(["init", "--worktrees-root", "~/.worktrees"]);
        cmd
    };

    init(&first).assert().success();
    init(&second)
        .assert()
        .failure()
        .stderr(predicate::str::contains("is already used by"))
        .stderr(predicate::str::contains(first.display().to_string()));

    // Re-running init for the first project is fine
    init(&first).assert().success();
}

#[test]
#[serial]
fn test_init_stores_relative_worktrees_root_as_absolute() {
    let temp_dir = setup_test_env();
    let home = temp_dir.path().canonicalize().unwrap();
    let repo = init_repo(&home, "code/my-repo", "git@github.com:test/my-repo.git");

    Command::cargo_bin("gwt")
        .unwrap()
        .current_dir(&repo)
        .env("HOME", &home)
        .args(["init", "--local", "--worktrees-root", "../wt"])
        .assert()
        .success();

    let config = fs::read_to_string(home.join("code").join("git-worktree-config.jsonc")).unwrap();
    let expected = home.join("code").join("wt").join("code-my-repo");
    assert!(
        config.contains(&format!("\"worktreesPath\": \"{}\"", expected.display())),
        "worktreesPath should be absolute: {}",
        config
    );
}

/// Bare layout: `proj/.bare` holds the repository, `proj/main` and
/// `proj-worktrees/<branch>` are its worktrees
#[test]
#[serial]
fn test_remove_from_inside_worktree_of_bare_layout() {
    let home = Home::new("seed");
    let origin = home.home.join("origin.git");
    let proj = home.code.join("proj");
    let bare = proj.join(".bare");
    let worktrees = home.code.join("proj-worktrees");
    let worktree = worktrees.join("feat");
    fs::create_dir_all(&worktrees).unwrap();

    git(
        &["clone", "--bare", origin.to_str().unwrap(), bare.to_str().unwrap()],
        &home.code,
    );
    git(&["worktree", "add", proj.join("main").to_str().unwrap(), "main"], &bare);
    git(
        &["worktree", "add", worktree.to_str().unwrap(), "-b", "feat", "main"],
        &bare,
    );
    fs::write(
        home.code.join("git-worktree-config.jsonc"),
        format!(
            r#"{{
  "repositoryUrl": "git@github.com:test/proj.git",
  "mainBranch": "main",
  "createdAt": "2025-06-25T17:25:28.766876Z",
  "sourceControl": "github",
  "projectPath": "{}",
  "worktreesPath": "{}"
}}"#,
            proj.display(),
            worktrees.display()
        ),
    )
    .unwrap();

    assert_eq!(home.gwt_cd(&worktree), worktrees.display().to_string());
    home.gwt(&worktree).args(["remove", "-f", "feat"]).assert().success();
    assert!(!worktree.exists());
    assert!(proj.join("main").join(".git").is_file(), "main worktree should be kept");
}
