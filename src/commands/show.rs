use anyhow::Result;
use serde::Serialize;

use crate::cli::ShowArgs;
use crate::commands::{current_dir, view};
use crate::ctx::Ctx;
use crate::health::{self, HealthReport};
use crate::output::{SessionView, print_json};
use crate::{logs, target};

#[derive(Serialize)]
struct Report {
    session: SessionView,
    health: Option<HealthReport>,
    log_tail: Vec<String>,
}

pub fn run(ctx: &Ctx, args: ShowArgs) -> Result<()> {
    let session = target::resolve_one(&ctx.conn, &args.target, &current_dir()?, false)?;
    let health = session
        .is_active()
        .then(|| health::health_url(&session))
        .flatten()
        .map(|url| health::probe(&url));
    let report = Report {
        session: view(&session),
        health,
        log_tail: logs::tail(&session.log_path, args.lines),
    };
    if args.json {
        return print_json(&report);
    }
    let s = &report.session;
    println!("name: {}", s.name);
    println!("session: {}", s.id);
    println!("state: {}", s.state);
    println!("url: {}", s.url);
    println!("process_id: {}", s.process_id);
    println!("directory: {}", s.cwd);
    match (&s.git_branch, &s.git_root) {
        (Some(branch), Some(root)) => println!(
            "git: {branch} ({root}{})",
            if s.git_worktree {
                ", linked worktree"
            } else {
                ""
            }
        ),
        _ => println!("git: not a repository"),
    }
    println!("command: {}", s.command);
    if !s.tags.is_empty() {
        println!("tags: {}", s.tags.join(", "));
    }
    println!("created_at: {}", s.created_at);
    println!("stopped_at: {}", s.stopped_at.as_deref().unwrap_or("-"));
    println!("log_path: {}", s.log_path);
    if let Some(h) = &report.health {
        println!(
            "health: {} ({}) {}",
            if h.ok { "ok" } else { "failing" },
            h.detail,
            h.url
        );
    }
    println!("log:");
    for line in &report.log_tail {
        println!("  {line}");
    }
    Ok(())
}
