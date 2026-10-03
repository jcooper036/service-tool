mod common;

use std::process::Command;

use common::{Env, SERVER, free_port};

fn git(dir: &std::path::Path, args: &[&str]) {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["-c", "user.name=t", "-c", "user.email=t@t"])
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn branch_and_worktree_are_recorded() {
    let env = Env::new();
    let repo = env.work.path().join("repo");
    std::fs::create_dir(&repo).unwrap();
    git(&repo, &["init", "-b", "main"]);
    git(&repo, &["commit", "--allow-empty", "-m", "init"]);
    let tree = env.work.path().join("feature-tree");
    git(
        &repo,
        &["worktree", "add", "-b", "feature", tree.to_str().unwrap()],
    );

    env.ser_in(
        &repo,
        &["run", "-d", "-n", "api", SERVER, &free_port().to_string()],
    );
    env.ser_in(
        &tree,
        &["run", "-d", "-n", "api", SERVER, &free_port().to_string()],
    );

    let json = env.status_json();
    let services = json["services"].as_array().unwrap();
    assert_eq!(
        services.len(),
        2,
        "same name in two directories is two services"
    );
    let main = services.iter().find(|s| s["git_branch"] == "main").unwrap();
    let feature = services
        .iter()
        .find(|s| s["git_branch"] == "feature")
        .unwrap();
    assert_eq!(main["git_worktree"], false);
    assert_eq!(feature["git_worktree"], true);
}

#[test]
fn ambiguous_name_resolves_by_current_directory() {
    let env = Env::new();
    let a = env.work.path().join("a");
    let b = env.work.path().join("b");
    std::fs::create_dir(&a).unwrap();
    std::fs::create_dir(&b).unwrap();
    let pa = free_port().to_string();
    env.ser_in(&a, &["run", "-d", "-n", "api", SERVER, &pa]);
    env.ser_in(
        &b,
        &["run", "-d", "-n", "api", SERVER, &free_port().to_string()],
    );

    let outside = env.ser(&["stop", "api"]);
    assert!(!outside.status.success());
    assert!(String::from_utf8_lossy(&outside.stderr).contains("matches several"));

    let inside = env.ser_in(&a, &["port", "api"]);
    assert!(inside.status.success());
    assert_eq!(common::stdout(&inside).trim(), pa);

    assert!(env.ser_in(&a, &["stop", "api"]).status.success());
    assert_eq!(env.status_json()["services"].as_array().unwrap().len(), 1);
}

#[test]
fn default_name_is_the_directory_name() {
    let env = Env::new();
    let dir = env.work.path().join("myproject");
    std::fs::create_dir(&dir).unwrap();
    env.ser_in(&dir, &["run", "-d", SERVER, &free_port().to_string()]);
    assert_eq!(env.status_json()["services"][0]["name"], "myproject");
}

#[test]
fn branch_is_known_before_the_first_commit() {
    let env = Env::new();
    git(env.work.path(), &["init", "-b", "trunk"]);
    env.ok(&["run", "-d", SERVER, &free_port().to_string()]);
    assert_eq!(env.status_json()["services"][0]["git_branch"], "trunk");
}
