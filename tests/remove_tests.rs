use assert_cmd::Command;
use predicates::prelude::*;
use serial_test::serial;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;
use tempfile::TempDir;

mod test_utils;
use test_utils::*;

/// A project laid out the way `gwt init` + `gwt add` leave it:
/// `<tmp>/my-repo`, `<tmp>/my-repo-worktrees/<branch>`, config in `<tmp>`
struct Project {
    _temp_dir: TempDir,
    root: PathBuf,
    repo: PathBuf,
    worktrees: PathBuf,
    hook_log: PathBuf,
}

impl Project {
    fn new(with_hooks: bool) -> Self {
        let temp_dir = setup_test_env();
        // Canonicalize so paths match what git reports (/var -> /private/var on macOS)
        let root = temp_dir.path().canonicalize().unwrap();
        let repo = root.join("my-repo");
        let worktrees = root.join("my-repo-worktrees");
        let hook_log = root.join("hooks.log");
        fs::create_dir(&repo).unwrap();
        create_test_git_repo(&repo, "git@github.com:test/my-repo.git");

        let hooks = if with_hooks {
            let log = hook_log.display();
            format!(
                r#""hooks": {{
    "preRemove": ["echo \"preRemove|${{branchName}}|${{worktreePath}}|$(pwd)\" >> {log}"],
    "postRemove": ["echo \"postRemove|${{branchName}}|${{worktreePath}}|$(pwd)\" >> {log}"]
  }}"#
            )
        } else {
            r#""hooks": { "postAdd": [], "preRemove": [], "postRemove": [] }"#.to_string()
        };

        let config = format!(
            r#"{{
  "repositoryUrl": "git@github.com:test/my-repo.git",
  "mainBranch": "main",
  "createdAt": "2025-06-25T17:25:28.766876Z",
  "sourceControl": "github",
  "projectPath": "{}",
  "worktreesPath": "{}",
  {}
}}"#,
            repo.display(),
            worktrees.display(),
            hooks
        );
        fs::write(root.join("git-worktree-config.jsonc"), config).unwrap();

        Self {
            _temp_dir: temp_dir,
            root,
            repo,
            worktrees,
            hook_log,
        }
    }

    fn git(&self, args: &[&str], cwd: &Path) {
        let output = StdCommand::new("git").args(args).current_dir(cwd).output().unwrap();
        assert!(
            output.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    /// Create a worktree for `branch` with one commit that is not on the main branch,
    /// like a squash-merged PR branch
    fn add_worktree(&self, branch: &str) -> PathBuf {
        let path = self.worktrees.join(branch);
        self.git(&["worktree", "add", path.to_str().unwrap(), "-b", branch], &self.repo);
        fs::write(path.join("feature.txt"), "work").unwrap();
        self.git(&["add", "."], &path);
        self.git(&["commit", "-m", "Feature work"], &path);
        path
    }

    fn branch_exists(&self, branch: &str) -> bool {
        let output = StdCommand::new("git")
            .args(["branch", "--list", branch])
            .current_dir(&self.repo)
            .output()
            .unwrap();
        !String::from_utf8_lossy(&output.stdout).trim().is_empty()
    }

    fn hook_log(&self) -> Vec<String> {
        fs::read_to_string(&self.hook_log)
            .unwrap_or_default()
            .lines()
            .map(str::to_string)
            .collect()
    }

    /// `gwt remove -f <branch>` from the main checkout, isolated from the
    /// user's global gwt config
    fn gwt_remove(&self, branch: &str) -> assert_cmd::assert::Assert {
        Command::cargo_bin("gwt")
            .unwrap()
            .current_dir(&self.repo)
            .env("HOME", &self.root)
            .args(["remove", "-f", branch])
            .assert()
    }
}

#[test]
#[serial]
fn remove_runs_hooks_and_deletes_branch_when_worktree_already_removed_with_git() {
    require_git!();
    let project = Project::new(true);
    let path = project.add_worktree("feature-x");

    // What `gh pr merge --delete-branch` does: remove the worktree with plain git
    project.git(&["worktree", "remove", path.to_str().unwrap()], &project.repo);
    assert!(!path.exists());
    assert!(project.branch_exists("feature-x"));

    project
        .gwt_remove("feature-x")
        .success()
        .stdout(predicate::str::contains("Worktree for 'feature-x' was already removed"))
        .stdout(predicate::str::contains("ran the remove hooks, branch deleted"));

    let path = path.display();
    let repo = project.repo.display();
    assert_eq!(
        project.hook_log(),
        vec![
            format!("preRemove|feature-x|{path}|{repo}"),
            format!("postRemove|feature-x|{path}|{repo}"),
        ]
    );
    assert!(
        !project.branch_exists("feature-x"),
        "unmerged branch should be force-deleted with -f"
    );
}

#[test]
#[serial]
fn remove_runs_hooks_when_worktree_and_branch_are_already_gone() {
    require_git!();
    let project = Project::new(true);
    let path = project.add_worktree("feature-x");
    project.git(&["worktree", "remove", path.to_str().unwrap()], &project.repo);
    project.git(&["branch", "-D", "feature-x"], &project.repo);

    project
        .gwt_remove("feature-x")
        .success()
        .stdout(predicate::str::contains("feature-x (already deleted)"));

    assert_eq!(project.hook_log().len(), 2);
    assert!(project.hook_log()[0].starts_with("preRemove|feature-x|"));
    assert!(project.hook_log()[1].starts_with("postRemove|feature-x|"));
}

#[test]
#[serial]
fn remove_deletes_leftover_branch_without_hooks() {
    require_git!();
    let project = Project::new(false);
    let path = project.add_worktree("feature-x");
    project.git(&["worktree", "remove", path.to_str().unwrap()], &project.repo);

    project
        .gwt_remove("feature-x")
        .success()
        .stdout(predicate::str::contains("no remove hooks configured"));

    assert!(!project.branch_exists("feature-x"));
}

#[test]
#[serial]
fn remove_still_fails_when_nothing_is_left_to_clean_up() {
    require_git!();
    let project = Project::new(false);

    project
        .gwt_remove("no-such-branch")
        .failure()
        .stderr(predicate::str::contains("Worktree for 'no-such-branch' not found"));
}

#[test]
#[serial]
fn remove_never_cleans_up_protected_branches() {
    require_git!();
    let project = Project::new(true);
    // Detach the main checkout so `main` has no worktree but the branch exists
    project.git(&["branch", "-M", "main"], &project.repo);
    project.git(&["checkout", "--detach"], &project.repo);

    project
        .gwt_remove("main")
        .failure()
        .stderr(predicate::str::contains("Worktree for 'main' not found"));

    assert!(project.hook_log().is_empty());
}

#[test]
#[serial]
fn remove_runs_hooks_for_orphaned_worktree() {
    require_git!();
    let project = Project::new(true);
    let path = project.add_worktree("feature-y");

    // Orphan it: the worktree's .git file now points to a missing gitdir
    fs::remove_dir_all(project.repo.join(".git").join("worktrees").join("feature-y")).unwrap();

    project
        .gwt_remove("feature-y")
        .success()
        .stdout(predicate::str::contains("orphaned worktree"));

    assert!(!path.exists());
    assert!(!project.branch_exists("feature-y"), "orphan's branch should be deleted");
    let path = path.display();
    let repo = project.repo.display();
    assert_eq!(
        project.hook_log(),
        vec![
            format!("preRemove|feature-y|{path}|{path}"),
            format!("postRemove|feature-y|{path}|{repo}"),
        ]
    );
}

#[test]
#[serial]
fn remove_rejects_glob_names_without_running_hooks() {
    require_git!();
    let project = Project::new(true);
    let path = project.add_worktree("feature-a");
    project.git(&["worktree", "remove", path.to_str().unwrap()], &project.repo);

    project
        .gwt_remove("feature-*")
        .failure()
        .stderr(predicate::str::contains("Worktree for 'feature-*' not found"));

    assert!(project.hook_log().is_empty());
    assert!(project.branch_exists("feature-a"));
}
