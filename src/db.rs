use std::path::Path;
use std::time::Duration;

use anyhow::Result;
use chrono::Utc;
use rusqlite::{Connection, Row, TransactionBehavior, params};

const MIGRATIONS: &[(&str, &str)] = &[(
    "26100220_create_sessions",
    "CREATE TABLE sessions (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        tags TEXT NOT NULL,
        cwd TEXT NOT NULL,
        command TEXT NOT NULL,
        health_url TEXT,
        requested_port INTEGER NOT NULL,
        port INTEGER NOT NULL,
        process_id INTEGER NOT NULL,
        process_started TEXT NOT NULL,
        created_at TEXT NOT NULL,
        stopped_at TEXT,
        log_path TEXT NOT NULL,
        git_branch TEXT,
        git_root TEXT,
        git_worktree INTEGER NOT NULL,
        heal_count INTEGER NOT NULL DEFAULT 0
    );
    CREATE INDEX sessions_active_idx ON sessions (stopped_at);",
)];

const COLUMNS: &str = "id, name, tags, cwd, command, health_url, requested_port, port, \
    process_id, process_started, created_at, stopped_at, log_path, git_branch, git_root, \
    git_worktree, heal_count";

#[derive(Debug, Clone)]
pub struct Session {
    pub id: String,
    pub name: String,
    pub tags: String,
    pub cwd: String,
    pub command: String,
    pub health_url: Option<String>,
    pub requested_port: u16,
    pub port: u16,
    pub process_id: u32,
    pub process_started: String,
    pub created_at: String,
    pub stopped_at: Option<String>,
    pub log_path: String,
    pub git_branch: Option<String>,
    pub git_root: Option<String>,
    pub git_worktree: bool,
    pub heal_count: u32,
}

impl Session {
    fn from_row(row: &Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            name: row.get(1)?,
            tags: row.get(2)?,
            cwd: row.get(3)?,
            command: row.get(4)?,
            health_url: row.get(5)?,
            requested_port: row.get(6)?,
            port: row.get(7)?,
            process_id: row.get(8)?,
            process_started: row.get(9)?,
            created_at: row.get(10)?,
            stopped_at: row.get(11)?,
            log_path: row.get(12)?,
            git_branch: row.get(13)?,
            git_root: row.get(14)?,
            git_worktree: row.get(15)?,
            heal_count: row.get(16)?,
        })
    }

    pub fn tag_list(&self) -> Vec<String> {
        self.tags
            .split(',')
            .filter(|tag| !tag.is_empty())
            .map(str::to_string)
            .collect()
    }

    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.split(',').any(|t| t == tag)
    }

    /// The random tail of the uuid7. The head is a timestamp, so sessions started close together share it.
    pub fn short_id(&self) -> &str {
        &self.id[self.id.len().saturating_sub(8)..]
    }

    pub fn is_active(&self) -> bool {
        self.stopped_at.is_none()
    }
}

pub fn open(path: &Path) -> Result<Connection> {
    let mut conn = Connection::open(path)?;
    conn.busy_timeout(Duration::from_secs(10))?;
    migrate(&mut conn)?;
    Ok(conn)
}

/// Runs in one write transaction so two `ser` processes opening a fresh database cannot both apply a migration.
fn migrate(conn: &mut Connection) -> Result<()> {
    let conn = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS migrations (name TEXT PRIMARY KEY, applied_at TEXT NOT NULL)",
    )?;
    for (name, sql) in MIGRATIONS {
        let applied: bool = conn.query_row(
            "SELECT EXISTS (SELECT 1 FROM migrations WHERE name = ?1)",
            [name],
            |row| row.get(0),
        )?;
        if applied {
            continue;
        }
        conn.execute_batch(sql)?;
        conn.execute(
            "INSERT INTO migrations (name, applied_at) VALUES (?1, ?2)",
            params![name, Utc::now().to_rfc3339()],
        )?;
    }
    conn.commit()?;
    Ok(())
}

pub fn insert_session(conn: &Connection, s: &Session) -> Result<()> {
    conn.execute(
        &format!(
            "INSERT INTO sessions ({COLUMNS}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, \
             ?11, ?12, ?13, ?14, ?15, ?16, ?17)"
        ),
        params![
            s.id,
            s.name,
            s.tags,
            s.cwd,
            s.command,
            s.health_url,
            s.requested_port,
            s.port,
            s.process_id,
            s.process_started,
            s.created_at,
            s.stopped_at,
            s.log_path,
            s.git_branch,
            s.git_root,
            s.git_worktree,
            s.heal_count
        ],
    )?;
    Ok(())
}

fn query(conn: &Connection, filter: &str) -> Result<Vec<Session>> {
    let sql = format!("SELECT {COLUMNS} FROM sessions {filter}");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], Session::from_row)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// Active sessions, oldest first.
pub fn active_sessions(conn: &Connection) -> Result<Vec<Session>> {
    query(conn, "WHERE stopped_at IS NULL ORDER BY id")
}

/// Every session, newest first. uuid7 ids sort by creation time.
pub fn all_sessions(conn: &Connection) -> Result<Vec<Session>> {
    query(conn, "ORDER BY id DESC")
}

pub fn stopped_sessions(conn: &Connection) -> Result<Vec<Session>> {
    query(conn, "WHERE stopped_at IS NOT NULL")
}

pub fn mark_stopped(conn: &Connection, id: &str) -> Result<()> {
    conn.execute(
        "UPDATE sessions SET stopped_at = ?1 WHERE id = ?2 AND stopped_at IS NULL",
        params![Utc::now().to_rfc3339(), id],
    )?;
    Ok(())
}

pub fn delete_session(conn: &Connection, id: &str) -> Result<()> {
    conn.execute("DELETE FROM sessions WHERE id = ?1", [id])?;
    Ok(())
}
