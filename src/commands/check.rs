use anyhow::{Result, bail};

use crate::cli::TargetArgs;
use crate::commands::{Silent, current_dir};
use crate::ctx::Ctx;
use crate::{health, target};

pub fn run(ctx: &Ctx, args: TargetArgs) -> Result<()> {
    let session = target::resolve_one(&ctx.conn, &args.target, &current_dir()?, true)?;
    let Some(url) = health::health_url(&session) else {
        bail!(
            "'{}' has no health URL (set one with `ser set {0} --health <url>`)",
            session.name
        );
    };
    let report = health::probe(&url);
    if report.ok {
        println!("healthy ({}) {}", report.detail, report.url);
        Ok(())
    } else {
        println!("unhealthy ({}) {}", report.detail, report.url);
        bail!(Silent)
    }
}
