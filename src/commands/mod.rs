pub mod check;
pub mod heal;
pub mod log;
pub mod port;
pub mod prune;
pub mod restart;
pub mod run;
pub mod set;
pub mod show;
pub mod status;
pub mod stop;
pub mod update;

use std::fmt;
use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::cli::{Cli, Command};
use crate::ctx::Ctx;
use crate::db::Session;
use crate::health;
use crate::home::Home;
use crate::output::{STOPPED_LABEL, SessionView};

/// An error whose output was already printed. Exits non-zero without a message.
#[derive(Debug)]
pub struct Silent;

impl fmt::Display for Silent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "failed")
    }
}

impl std::error::Error for Silent {}

pub fn current_dir() -> Result<PathBuf> {
    std::env::current_dir()
        .and_then(std::fs::canonicalize)
        .context("reading the current directory")
}

pub fn view(session: &Session) -> SessionView {
    match session.stopped_at {
        Some(_) => SessionView::new(session, STOPPED_LABEL),
        None => SessionView::new(session, health::state(session).label()),
    }
}

pub fn run(cli: Cli) -> Result<()> {
    if matches!(cli.command, Command::Update) {
        return update::run();
    }
    let mut ctx = Ctx::open(Home::resolve(cli.home)?)?;
    match cli.command {
        Command::Update => unreachable!("handled before state is opened"),
        Command::Run(args) => run::run(&mut ctx, args),
        Command::Stop(args) => stop::run(&ctx, args),
        Command::Status(args) => status::run(&ctx, args),
        Command::Heal(args) => heal::run(&mut ctx, args),
        Command::Restart(args) => restart::run(&mut ctx, args),
        Command::Log(args) => log::run(&ctx, args),
        Command::Check(args) => check::run(&ctx, args),
        Command::Show(args) => show::run(&ctx, args),
        Command::Port(args) => port::run(&ctx, args),
        Command::Set(args) => set::run(&ctx, args),
        Command::Prune => prune::run(&ctx),
    }
}
