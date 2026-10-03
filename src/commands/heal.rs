use std::path::Path;
use std::time::Duration;

use anyhow::Result;
use serde::Serialize;

use crate::cli::HealArgs;
use crate::ctx::Ctx;
use crate::output::print_json;
use crate::{db, process, service};

/// A service healed this many times in a row is given up on until it is restarted by hand.
const MAX_HEALS: u32 = 3;

#[derive(Serialize)]
struct Outcome {
    name: String,
    session_id: String,
    action: &'static str,
    port: Option<u16>,
    detail: String,
}

pub fn run(ctx: &mut Ctx, args: HealArgs) -> Result<()> {
    let wait = Duration::from_secs(args.wait);
    let mut outcomes = Vec::new();
    for old in db::active_sessions(&ctx.conn)? {
        if process::is_alive(old.process_id, &old.process_started) {
            continue;
        }
        let mut outcome = Outcome {
            name: old.name.clone(),
            session_id: old.id.clone(),
            action: "skipped",
            port: None,
            detail: String::new(),
        };
        if old.heal_count >= MAX_HEALS {
            outcome.detail = format!(
                "died after {MAX_HEALS} heals in a row; check `ser log {}` then `ser restart {}`",
                old.name, old.name
            );
        } else if !Path::new(&old.cwd).is_dir() {
            outcome.detail = format!("directory {} no longer exists", old.cwd);
        } else {
            db::mark_stopped(&ctx.conn, &old.id)?;
            match service::relaunch(ctx, &old, old.heal_count + 1, wait) {
                Ok(session) => {
                    outcome.action = "healed";
                    outcome.port = Some(session.port);
                    outcome.detail = format!("now on http://localhost:{}", session.port);
                }
                Err(err) => {
                    outcome.action = "failed";
                    outcome.detail = format!("{err:#}");
                }
            }
        }
        outcomes.push(outcome);
    }
    if args.json {
        return print_json(&outcomes);
    }
    if outcomes.is_empty() {
        println!("nothing to heal");
    }
    for o in &outcomes {
        println!("{} {}: {}", o.action, o.name, o.detail);
    }
    Ok(())
}
