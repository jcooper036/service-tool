use std::path::Path;
use std::process::Command;

#[derive(Debug, Default, Clone)]
pub struct GitInfo {
    pub branch: Option<String>,
    pub root: Option<String>,
    pub worktree: bool,
}

fn git(dir: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Branch, repository root, and whether the directory belongs to a linked worktree. All empty outside a repository.
pub fn info(dir: &Path) -> GitInfo {
    let Some(root) = git(dir, &["rev-parse", "--show-toplevel"]) else {
        return GitInfo::default();
    };
    // `--show-current` also works before the first commit, and is empty on a detached HEAD.
    let branch = match git(dir, &["branch", "--show-current"]).as_deref() {
        Some("") | None => {
            git(dir, &["rev-parse", "--short", "HEAD"]).map(|sha| format!("detached@{sha}"))
        }
        Some(name) => Some(name.to_string()),
    };
    let dirs = git(
        dir,
        &[
            "rev-parse",
            "--path-format=absolute",
            "--git-dir",
            "--git-common-dir",
        ],
    );
    let worktree = dirs
        .as_deref()
        .and_then(|text| text.split_once('\n'))
        .is_some_and(|(git_dir, common)| git_dir != common);
    GitInfo {
        branch,
        root: Some(root),
        worktree,
    }
}
