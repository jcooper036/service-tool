use anyhow::{Result, bail};

use crate::cli::StopArgs;
use crate::commands::current_dir;
use crate::ctx::Ctx;
use crate::{db, service, target};

pub fn run(ctx: &Ctx, args: StopArgs) -> Result<()> {
    let sessions = match &args.target {
        Some(target) => {
            let found = target::lookup(&ctx.conn, target, &current_dir()?, true, true)?;
            if found.is_empty() {
                bail!("no running service matching '{target}'");
            }
            found
        }
        None => db::active_sessions(&ctx.conn)?,
    };
    if sessions.is_empty() {
        eprintln!("nothing is running");
        return Ok(());
    }
    for session in &sessions {
        service::stop(ctx, session)?;
        println!("stopped {} (port {})", session.name, session.port);
    }
    Ok(())
}
