use std::time::Duration;

use anyhow::Result;

use crate::cli::RestartArgs;
use crate::commands::current_dir;
use crate::ctx::Ctx;
use crate::{service, target};

pub fn run(ctx: &mut Ctx, args: RestartArgs) -> Result<()> {
    let old = target::resolve_one(&ctx.conn, &args.target, &current_dir()?, false)?;
    service::stop(ctx, &old)?;
    let session = service::relaunch(ctx, &old, 0, Duration::from_secs(args.wait))?;
    println!("{}", session.port);
    eprintln!(
        "restarted {} on http://localhost:{}",
        session.name, session.port
    );
    Ok(())
}
