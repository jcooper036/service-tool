use std::collections::HashSet;

use anyhow::{Result, bail};
use comfy_table::{Table, presets::NOTHING};
use serde::Serialize;

use crate::cli::StatusArgs;
use crate::commands::{Silent, view};
use crate::ctx::Ctx;
use crate::db;
use crate::health;
use crate::output::{SessionView, print_json};
use crate::ports::{self, Listener};
use crate::process::Liveness;

#[derive(Serialize)]
struct Report {
    services: Vec<SessionView>,
    unmanaged: Vec<Listener>,
}

fn quiet(ctx: &Ctx) -> Result<()> {
    let sessions = db::active_sessions(&ctx.conn)?;
    if !sessions.is_empty() && sessions.iter().all(|s| health::state(s) == Liveness::Up) {
        Ok(())
    } else {
        bail!(Silent)
    }
}

fn short_dir(dir: &str) -> String {
    match dirs::home_dir() {
        Some(home) => match dir.strip_prefix(&*home.to_string_lossy()) {
            Some(rest) => format!("~{rest}"),
            None => dir.to_string(),
        },
        None => dir.to_string(),
    }
}

fn branch_label(v: &SessionView) -> String {
    match &v.git_branch {
        Some(branch) if v.git_worktree => format!("{branch} (worktree)"),
        Some(branch) => branch.clone(),
        None => "-".to_string(),
    }
}

fn print_services(views: &[SessionView]) {
    let mut table = Table::new();
    table.load_style(NOTHING);
    table.set_header(["ID", "NAME", "STATE", "URL", "PID", "BRANCH", "DIR", "TAGS"]);
    for v in views {
        table.add_row([
            v.id[v.id.len().saturating_sub(8)..].to_string(),
            v.name.clone(),
            v.state.clone(),
            v.url.clone(),
            v.process_id.to_string(),
            branch_label(v),
            short_dir(&v.cwd),
            v.tags.join(","),
        ]);
    }
    println!("{table}");
}

fn print_unmanaged(listeners: &[Listener]) {
    if listeners.is_empty() {
        return;
    }
    let mut table = Table::new();
    table.load_style(NOTHING);
    table.set_header(["ADDRESS", "PID", "COMMAND"]);
    for l in listeners {
        table.add_row([
            format!("localhost:{}", l.port),
            l.pid.to_string(),
            l.command.clone(),
        ]);
    }
    println!("\nlistening, not managed by ser:\n{table}");
}

pub fn run(ctx: &Ctx, args: StatusArgs) -> Result<()> {
    if args.quiet {
        return quiet(ctx);
    }
    let sessions = if args.all {
        db::all_sessions(&ctx.conn)?
    } else {
        db::active_sessions(&ctx.conn)?
    };
    let managed: HashSet<u16> = db::active_sessions(&ctx.conn)?
        .iter()
        .map(|s| s.port)
        .collect();
    let services: Vec<SessionView> = sessions.iter().map(view).collect();
    let unmanaged: Vec<Listener> = ports::listeners()
        .into_iter()
        .filter(|l| !managed.contains(&l.port))
        .collect();

    if args.json {
        return print_json(&Report {
            services,
            unmanaged,
        });
    }
    if services.is_empty() {
        println!("no services");
    } else {
        print_services(&services);
    }
    print_unmanaged(&unmanaged);
    Ok(())
}
