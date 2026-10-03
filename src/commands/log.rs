use std::path::Path;

use anyhow::Result;

use crate::cli::LogArgs;
use crate::commands::current_dir;
use crate::ctx::Ctx;
use crate::{logs, target};

pub fn run(ctx: &Ctx, args: LogArgs) -> Result<()> {
    let session = target::resolve_one(&ctx.conn, &args.target, &current_dir()?, false)?;
    let path = Path::new(&session.log_path);
    let text = logs::read(path)?;
    for line in logs::tail_lines(&text, args.lines) {
        println!("{line}");
    }
    if args.follow {
        logs::follow(path, text.len() as u64)?;
    }
    Ok(())
}
