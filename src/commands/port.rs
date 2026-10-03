use anyhow::{Result, bail};

use crate::cli::TargetArgs;
use crate::commands::{Silent, current_dir};
use crate::ctx::Ctx;
use crate::{process, target};

pub fn run(ctx: &Ctx, args: TargetArgs) -> Result<()> {
    let session = target::resolve_one(&ctx.conn, &args.target, &current_dir()?, true)?;
    if !process::is_alive(session.process_id, &session.process_started) {
        eprintln!("'{}' is not running", session.name);
        bail!(Silent);
    }
    println!("{}", session.port);
    Ok(())
}
