use anyhow::Result;
use serde::Serialize;

use crate::db::Session;

#[derive(Debug, Serialize)]
pub struct SessionView {
    pub id: String,
    pub name: String,
    pub tags: Vec<String>,
    pub state: String,
    pub port: u16,
    pub url: String,
    pub process_id: u32,
    pub cwd: String,
    pub git_branch: Option<String>,
    pub git_root: Option<String>,
    pub git_worktree: bool,
    pub command: String,
    pub health_url: Option<String>,
    pub created_at: String,
    pub stopped_at: Option<String>,
    pub log_path: String,
    pub heal_count: u32,
}

pub const STOPPED_LABEL: &str = "stopped";

pub fn url(port: u16) -> String {
    format!("http://localhost:{port}")
}

impl SessionView {
    pub fn new(session: &Session, state: &str) -> Self {
        Self {
            id: session.id.clone(),
            name: session.name.clone(),
            tags: session.tag_list(),
            state: state.to_string(),
            port: session.port,
            url: url(session.port),
            process_id: session.process_id,
            cwd: session.cwd.clone(),
            git_branch: session.git_branch.clone(),
            git_root: session.git_root.clone(),
            git_worktree: session.git_worktree,
            command: session.command.clone(),
            health_url: session.health_url.clone(),
            created_at: session.created_at.clone(),
            stopped_at: session.stopped_at.clone(),
            log_path: session.log_path.clone(),
            heal_count: session.heal_count,
        }
    }
}

pub fn print_json<T: Serialize>(value: &T) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}
