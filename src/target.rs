use std::path::Path;

use anyhow::{Result, bail};
use rusqlite::Connection;

use crate::db::{self, Session};

fn describe(session: &Session) -> String {
    format!(
        "{} ({}, {}{})",
        session.name,
        session.short_id(),
        session.cwd,
        session
            .git_branch
            .as_ref()
            .map(|b| format!(", {b}"))
            .unwrap_or_default()
    )
}

fn by_name(pool: &[Session], name: &str, cwd: &Path) -> Result<Vec<Session>> {
    let matches: Vec<&Session> = pool.iter().filter(|s| s.name == name).collect();
    if matches.is_empty() {
        return Ok(Vec::new());
    }
    // Pool is newest first, so the first session seen for a directory is its latest. Running sessions win over stopped ones.
    let mut latest: Vec<&Session> = Vec::new();
    for s in matches {
        if !latest.iter().any(|l| l.cwd == s.cwd) {
            latest.push(s);
        }
    }
    if latest.iter().any(|s| s.is_active()) {
        latest.retain(|s| s.is_active());
    }
    if latest.len() > 1 {
        let here: Vec<&Session> = latest
            .iter()
            .copied()
            .filter(|s| Path::new(&s.cwd) == cwd)
            .collect();
        if here.len() == 1 {
            return Ok(vec![here[0].clone()]);
        }
        bail!(
            "'{name}' matches several services: {}. Use the session id, or run from the service's directory",
            latest
                .iter()
                .map(|s| describe(s))
                .collect::<Vec<_>>()
                .join("; ")
        );
    }
    Ok(vec![latest[0].clone()])
}

/// Resolves a target to sessions, trying in order: full id, name, process id, short id, then (if allowed) tag.
/// Names and ids resolve to at most one session. Only a tag can match several.
pub fn lookup(
    conn: &Connection,
    target: &str,
    cwd: &Path,
    active_only: bool,
    allow_tag: bool,
) -> Result<Vec<Session>> {
    let pool: Vec<Session> = db::all_sessions(conn)?
        .into_iter()
        .filter(|s| !active_only || s.is_active())
        .collect();

    if let Some(s) = pool.iter().find(|s| s.id == target) {
        return Ok(vec![s.clone()]);
    }
    let named = by_name(&pool, target, cwd)?;
    if !named.is_empty() {
        return Ok(named);
    }
    if let Ok(pid) = target.parse::<u32>()
        && let Some(s) = pool.iter().find(|s| s.is_active() && s.process_id == pid)
    {
        return Ok(vec![s.clone()]);
    }
    if target.len() >= 8 && target.chars().all(|c| c.is_ascii_hexdigit() || c == '-') {
        let ids: Vec<&Session> = pool
            .iter()
            .filter(|s| s.id.ends_with(target) || s.id.starts_with(target))
            .collect();
        match ids.len() {
            0 => {}
            1 => return Ok(vec![ids[0].clone()]),
            _ => bail!("'{target}' matches several sessions; use more of the id"),
        }
    }
    if allow_tag {
        return Ok(pool
            .into_iter()
            .filter(|s| s.is_active() && s.has_tag(target))
            .collect());
    }
    Ok(Vec::new())
}

pub fn resolve_one(
    conn: &Connection,
    target: &str,
    cwd: &Path,
    active_only: bool,
) -> Result<Session> {
    match lookup(conn, target, cwd, active_only, false)?
        .into_iter()
        .next()
    {
        Some(session) => Ok(session),
        None if active_only => bail!("no running service '{target}'"),
        None => bail!("no service or session '{target}'"),
    }
}
