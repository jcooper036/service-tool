use anyhow::{Result, bail};

use crate::cli::SetArgs;
use crate::commands::current_dir;
use crate::ctx::Ctx;
use crate::{db, service, target};

pub fn run(ctx: &Ctx, args: SetArgs) -> Result<()> {
    let new_tags = service::parse_tags(&args.tags);
    if args.health.is_none() && new_tags.is_empty() {
        bail!("nothing to set; give --health or --tag");
    }
    if let Some(url) = &args.health {
        service::validate_health_url(url)?;
    }
    let session = target::resolve_one(&ctx.conn, &args.target, &current_dir()?, true)?;
    if let Some(url) = &args.health {
        db::set_health_url(&ctx.conn, &session.id, url)?;
    }
    if !new_tags.is_empty() {
        let mut tags = session.tag_list();
        for tag in new_tags {
            if !tags.contains(&tag) {
                tags.push(tag);
            }
        }
        db::set_tags(&ctx.conn, &session.id, &tags.join(","))?;
    }
    Ok(())
}
