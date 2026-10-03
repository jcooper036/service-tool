use anyhow::Result;

use crate::ctx::Ctx;
use crate::db;

pub fn run(ctx: &Ctx) -> Result<()> {
    let stopped = db::stopped_sessions(&ctx.conn)?;
    for session in &stopped {
        let _ = std::fs::remove_file(&session.log_path);
        db::delete_session(&ctx.conn, &session.id)?;
    }
    println!("pruned {} sessions", stopped.len());
    Ok(())
}
