use std::collections::HashSet;
use std::path::PathBuf;
use std::process::Child;
use std::time::Duration;

use anyhow::{Result, bail};
use chrono::Utc;
use rusqlite::TransactionBehavior;
use uuid::Uuid;

use crate::ctx::Ctx;
use crate::db::{self, Session};
use crate::{git, logs, ports, process};

pub const PORT_PLACEHOLDER: &str = "{port}";

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Detached,
    Foreground,
}

pub struct StartSpec {
    pub name: String,
    pub tags: Vec<String>,
    pub cwd: PathBuf,
    pub command: String,
    pub health_url: Option<String>,
    pub requested_port: u16,
    pub heal_count: u32,
    pub mode: Mode,
}

pub enum Outcome {
    Existing(Session),
    Started(Session, Child),
}

/// Picks a port and records the session in one write transaction, so concurrent starts cannot claim the same port.
/// A live session with the same name and directory is returned instead of starting a duplicate.
pub fn start(ctx: &mut Ctx, spec: &StartSpec) -> Result<Outcome> {
    if !spec.cwd.is_dir() {
        bail!("directory {} does not exist", spec.cwd.display());
    }
    if spec.command.trim().is_empty() {
        bail!("empty command");
    }
    if let Some(url) = &spec.health_url
        && !(url.starts_with("http://") || url.starts_with("https://"))
    {
        bail!("health URL must start with http:// or https://");
    }
    let cwd = spec.cwd.to_string_lossy().into_owned();

    let logs_dir = ctx.home.logs_dir();
    let tx = ctx
        .conn
        .transaction_with_behavior(TransactionBehavior::Immediate)?;

    let mut claimed = HashSet::new();
    for s in db::active_sessions(&tx)? {
        if !process::is_alive(s.process_id, &s.process_started) {
            if s.name == spec.name && s.cwd == cwd {
                db::mark_stopped(&tx, &s.id)?;
            }
            continue;
        }
        if s.name == spec.name && s.cwd == cwd {
            if s.command == spec.command {
                return Ok(Outcome::Existing(s));
            }
            bail!(
                "'{}' is already running in {} with a different command ({}). Stop it, or use -n to name this one",
                s.name,
                s.cwd,
                s.command
            );
        }
        claimed.insert(s.port);
    }

    let port = ports::pick(spec.requested_port, &claimed)?;
    let id = Uuid::now_v7().to_string();
    let log_path = logs_dir.join(format!("{id}.log"));
    let rendered = spec.command.replace(PORT_PLACEHOLDER, &port.to_string());
    let child = match spec.mode {
        Mode::Detached => process::spawn_detached(&rendered, &spec.cwd, port, &log_path)?,
        Mode::Foreground => {
            std::fs::File::create(&log_path)?;
            process::spawn_foreground(&rendered, &spec.cwd, port)?
        }
    };
    let git = git::info(&spec.cwd);
    let session = Session {
        id,
        name: spec.name.clone(),
        tags: spec.tags.join(","),
        cwd,
        command: spec.command.clone(),
        health_url: spec.health_url.clone(),
        requested_port: spec.requested_port,
        port,
        process_id: child.id(),
        process_started: process::start_time(child.id()).unwrap_or_default(),
        created_at: Utc::now().to_rfc3339(),
        stopped_at: None,
        log_path: log_path.to_string_lossy().into_owned(),
        git_branch: git.branch,
        git_root: git.root,
        git_worktree: git.worktree,
        heal_count: spec.heal_count,
    };
    db::insert_session(&tx, &session)?;
    tx.commit()?;
    Ok(Outcome::Started(session, child))
}

/// Waits for a detached session to accept connections. Errors, and marks the session stopped, if the process exits first.
/// Returns false when it is still running but not listening yet.
pub fn await_up(ctx: &Ctx, session: &Session, child: &mut Child, wait: Duration) -> Result<bool> {
    if process::wait_for_port(child, session.port, wait) {
        return Ok(true);
    }
    if child.try_wait()?.is_some() {
        db::mark_stopped(&ctx.conn, &session.id)?;
        let tail = logs::tail(&session.log_path, 15).join("\n");
        bail!(
            "'{}' exited before listening on port {}\n{tail}",
            session.name,
            session.port
        );
    }
    Ok(false)
}

/// Starts a fresh detached session from an old one, preferring the old port. The caller stops the old session first.
pub fn relaunch(ctx: &mut Ctx, old: &Session, heal_count: u32, wait: Duration) -> Result<Session> {
    let spec = StartSpec {
        name: old.name.clone(),
        tags: old.tag_list(),
        cwd: PathBuf::from(&old.cwd),
        command: old.command.clone(),
        health_url: old.health_url.clone(),
        requested_port: old.port,
        heal_count,
        mode: Mode::Detached,
    };
    match start(ctx, &spec)? {
        Outcome::Existing(session) => Ok(session),
        Outcome::Started(session, mut child) => {
            await_up(ctx, &session, &mut child, wait)?;
            Ok(session)
        }
    }
}

/// Stops a session's process if it is still ours, and marks the session stopped.
pub fn stop(ctx: &Ctx, session: &Session) -> Result<()> {
    process::terminate(
        session.process_id,
        &session.process_started,
        Duration::from_secs(5),
    );
    db::mark_stopped(&ctx.conn, &session.id)
}
